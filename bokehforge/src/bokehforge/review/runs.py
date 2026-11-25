"""Local evaluation batches and their durable, per-scene results."""

from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import re
from shutil import which
import subprocess
from tempfile import TemporaryDirectory
from threading import Event, Lock, Thread
from uuid import uuid4

from .images import normalize_output, stage_frame

REPO = Path(__file__).resolve().parents[4]
RUN_ID = re.compile(r"\d{8}T\d{12}Z-[a-f0-9]{8}")


@dataclass(frozen=True)
class Stacker:
    id: str
    label: str
    method: str
    executable: str | None
    setup: str

    def command(self, frames, output):
        if self.id == "pancake":
            return [self.executable, "--cpu", "--bit-depth", "16", "--output", output, "--input", *frames]
        if self.id == "focus-stack":
            return [self.executable, f"--output={output}", *frames]
        if self.id == "openfocus":
            return [self.executable, "-i", "frames", "-o", output, "--frame-order", "name", "-m", "stackmffv3", "--cpu"]
        return [self.executable, str(Path(__file__).with_name("_shine.py")), "frames", output]


def _executable(override, *defaults):
    for candidate in (override,) if override else defaults:
        found = which(str(candidate))
        if found:
            # Resolving a venv's Python symlink would discard its environment.
            return str(Path(found).absolute())
    return None


def configured_stackers(args):
    modules = REPO / ".submodules"
    bin_dir = "Scripts" if os.name == "nt" else "bin"
    extension = ".exe" if os.name == "nt" else ""
    pancake = "pancake-cli" + extension
    focus = "focus-stack" + extension
    openfocus = "openfocus" + extension
    python = "python" + extension
    return {tool.id: tool for tool in (
        Stacker("pancake", "Pancake", "Default fusion · CPU",
                _executable(args.pancake, REPO / "pancake/target/release" / pancake, pancake),
                "Build pancake-cli or pass --pancake."),
        Stacker("focus-stack", "focus-stack", "Complex wavelet",
                _executable(args.focus_stack, modules / "focus-stack/build" / focus, focus),
                "Build .submodules/focus-stack or pass --focus-stack."),
        Stacker("openfocus", "OpenFocus", "StackMFF V3 · CPU",
                _executable(args.openfocus, modules / "OpenFocus/.venv" / bin_dir / openfocus, openfocus),
                "Install .submodules/OpenFocus into its .venv or pass --openfocus."),
        Stacker("shinestacker", "ShineStacker", "Alignment + Pyramid",
                _executable(args.shine_python, modules / "shinestacker/.venv" / bin_dir / python),
                "Install .submodules/shinestacker into its .venv or pass --shine-python."),
    )}


def _now():
    return datetime.now(timezone.utc)


def _read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_suffix(".tmp")
    pending.write_text(json.dumps(payload, ensure_ascii=False) + "\n", encoding="utf-8")
    pending.replace(path)


def scene_fingerprint(catalog, scene):
    files = [(frame_id, catalog.frame(frame_id).source_sha256)
             for frame_id in (*scene.frame_ids, *scene.reference_ids)]
    return sha256(json.dumps([scene.metadata["recipe_sha256"], files]).encode()).hexdigest()


@contextmanager
def lock_store(directory):
    """Keep two server processes from treating each other's live runs as interrupted."""
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / ".lock").open("a+b") as handle:
        try:
            if os.name == "nt":
                import msvcrt
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as exc:
            raise RuntimeError(f"Another review server is using {directory}") from exc
        yield


class RunStopped(Exception):
    pass


def _render(tool, frames, work, log, stop):
    delivery = f"{tool.id}.tif"
    command = tool.command(frames, delivery)
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("wb") as stream:
        stream.write((json.dumps(command) + "\n").encode())
        stream.flush()
        process = subprocess.Popen(command, cwd=work, stdin=subprocess.DEVNULL,
                                   stdout=stream, stderr=subprocess.STDOUT)
        while process.poll() is None and not stop.wait(0.25):
            pass
        if stop.is_set():
            process.terminate()
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise RunStopped()
        if process.returncode:
            raise RuntimeError(f"{tool.label} exited with code {process.returncode}. See the stacker log.")
    return work / delivery


class Runs:
    def __init__(self, store, stackers):
        self.root = store / "runs"
        self.stackers = stackers
        self.worker = None
        self.active = None
        self.stop = Event()
        self.lock = Lock()
        for path in self.root.glob("*/progress.json"):
            progress = _read(path)
            if progress["status"] == "running":
                _write(path, {**progress, "status": "interrupted", "finished_at": _now().isoformat()})

    def tooling(self):
        return [{"id": tool.id, "label": tool.label, "method": tool.method,
                 "available": tool.executable is not None, "setup": tool.setup}
                for tool in self.stackers.values()]

    def list(self):
        summaries = []
        for path in sorted(self.root.glob("*/run.json"), reverse=True):
            run = _read(path)
            del run["scene_ids"]
            summaries.append({**run, **_read(path.with_name("progress.json"))})
        return summaries

    def directory(self, run_id):
        if not isinstance(run_id, str) or not RUN_ID.fullmatch(run_id):
            raise ValueError("Invalid evaluation run ID")
        return self.root / run_id

    def scene_results(self, run_id, catalog, scene):
        if not run_id:
            return []
        base = self.directory(run_id)
        if not (base / "run.json").is_file():
            raise KeyError("Evaluation run not found")
        run = _read(base / "run.json")
        progress = _read(base / "progress.json")
        selected = scene.id in run["scene_ids"]
        fingerprint = scene_fingerprint(catalog, scene) if selected else None
        rows = []
        for stacker in run["stackers"]:
            path = base / stacker / "results" / f"{scene.id}.json"
            if not selected:
                row = {"status": "outside_scope"}
            elif path.is_file():
                row = _read(path)
                if row["scene_fingerprint"] != fingerprint:
                    row = {"status": "different_source"}
                elif row["status"] == "ready" and not (base / row["image"]).is_file():
                    row = {"status": "missing_file"}
            else:
                row = {"status": "pending" if progress["status"] == "running" else "unfinished"}
            rows.append({"stacker": stacker, **row})
        return rows

    def start(self, catalog, scene_ids, stacker_ids, scope):
        with self.lock:
            if self.worker is not None and self.worker.is_alive():
                raise RuntimeError("An evaluation is already running")
            if not stacker_ids or any(key not in self.stackers for key in stacker_ids):
                raise ValueError("Select at least one supported stacker")
            tools = [self.stackers[key] for key in dict.fromkeys(stacker_ids)]
            missing = [tool.label for tool in tools if tool.executable is None]
            if missing:
                raise ValueError("Stacker setup needed for " + ", ".join(missing))
            started = _now()
            run_id = started.strftime("%Y%m%dT%H%M%S%fZ-") + uuid4().hex[:8]
            base = self.directory(run_id)
            base.mkdir(parents=True, exist_ok=False)
            run = {"schema_version": 1, "id": run_id, "created_at": started.isoformat(),
                   "scope": scope, "scene_ids": scene_ids,
                   "stackers": [tool.id for tool in tools],
                   "methods": {tool.id: tool.method for tool in tools},
                   "input_format": "16-bit SDR sRGB TIFF",
                   "output_format": "float32 linear-sRGB RGB TIFF"}
            progress = {"status": "running", "total": len(scene_ids) * len(tools),
                        "ready": 0, "failed": 0, "current_scene": None}
            _write(base / "progress.json", progress)
            _write(base / "run.json", run)
            self.active = run_id
            self.stop.clear()
            self.worker = Thread(target=self._worker, args=(base, catalog, scene_ids, tools, progress),
                                 name="bokehforge-evaluation")
            self.worker.start()
            return run_id

    def cancel(self, run_id):
        with self.lock:
            if run_id != self.active or self.worker is None or not self.worker.is_alive():
                raise ValueError("This evaluation is no longer running")
            self.stop.set()

    def close(self):
        if self.worker is not None:
            self.stop.set()
            self.worker.join()

    def _check_stop(self):
        if self.stop.is_set():
            raise RunStopped()

    def _worker(self, base, catalog, scene_ids, tools, progress):
        try:
            for scene_id in scene_ids:
                self._check_stop()
                progress["current_scene"] = scene_id
                _write(base / "progress.json", progress)
                self._evaluate_scene(base, catalog, catalog.scene(scene_id), tools, progress)
            progress["status"] = "completed_with_errors" if progress["failed"] else "completed"
        except RunStopped:
            progress["status"] = "cancelled"
        except Exception as exc:
            progress.update(status="error", error=str(exc))
        finally:
            progress.update(finished_at=_now().isoformat(), current_scene=None)
            _write(base / "progress.json", progress)

    def _evaluate_scene(self, base, catalog, scene, tools, progress):
        fingerprint = scene_fingerprint(catalog, scene)
        with TemporaryDirectory(prefix="work-", dir=base) as scratch:
            work = Path(scratch)
            (work / "frames").mkdir()
            frames = []
            input_error = None
            try:
                for ordinal, frame_id in enumerate(scene.frame_ids):
                    self._check_stop()
                    path = f"frames/{ordinal:06d}.tif"
                    stage_frame(work / path, catalog.decode(frame_id).data)
                    frames.append(path)
            except RunStopped:
                raise
            except Exception as exc:
                input_error = f"Could not prepare {frame_id}: {exc}"

            for tool in tools:
                self._check_stop()
                row = {"scene_id": scene.id, "stacker": tool.id, "scene_fingerprint": fingerprint}
                if input_error:
                    row.update(status="failed", error=input_error)
                else:
                    log = base / tool.id / "logs" / f"{scene.id}.log"
                    dest = base / tool.id / "images" / f"{scene.id}.tif"
                    try:
                        delivery = _render(tool, frames, work, log, self.stop)
                        pixel_metadata = normalize_output(delivery, dest)
                        row.update(status="ready", image=dest.relative_to(base).as_posix(), **pixel_metadata)
                    except RunStopped:
                        raise
                    except Exception as exc:
                        row.update(status="failed", error=str(exc))
                    if log.is_file():
                        row["log"] = log.relative_to(base).as_posix()
                _write(base / tool.id / "results" / f"{scene.id}.json", row)
                progress[row["status"]] += 1
                _write(base / "progress.json", progress)
