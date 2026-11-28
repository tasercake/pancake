"""Local evaluation batches and their durable, per-scene results."""

from contextlib import contextmanager
from dataclasses import replace
from datetime import datetime, timezone
from hashlib import sha256
import json
import os
from pathlib import Path
import re
from tempfile import TemporaryDirectory
from threading import Lock, Thread
from uuid import uuid4

from ..bench import has_benchmark_reference
from . import _zerene
from .images import normalize_output, stage_frame
from .stacker_setup import RunSignal, RunStopped, StackerSetup, run_logged

RUN_ID = re.compile(r"\d{8}T\d{12}Z-[a-f0-9]{8}")


def _now():
    return datetime.now(timezone.utc)


def _read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path, payload):
    path.parent.mkdir(parents=True, exist_ok=True)
    pending = path.with_suffix(".tmp")
    pending.write_text(json.dumps(payload, ensure_ascii=False) + "\n", encoding="utf-8")
    pending.replace(path)


# These converter/loader revisions produce equivalent canonical frames. Match
# exact pairs so a future decoder change still invalidates previous results.
_COMPATIBLE_RECIPE_CODE = (
    # First full-reference benchmark release.
    ("f0b9ebf6223225b9ebd0810cbe450a5bd2ecbf3c38955a8be025745f118fc20b",
     "fa756299117c6a728d941742abafe6afab5c88c1ee282e80b102c53e083dedb6"),
    # Catalog selection update, also shipped in the initial Rust scorer.
    ("f0b9ebf6223225b9ebd0810cbe450a5bd2ecbf3c38955a8be025745f118fc20b",
     "402b8fd8c03e6b23ce67aa9b981c9798312d7f1109fbab0e0f387938c93940d7"),
    # Native runtime with the equivalent PNG fast path.
    ("2cb7a404d768a107fc03f2f54049559cae6127af8924190f5bd21aca42f377ac",
     "9af934c6991ea96d9935d43b932644c38f0a64aae7b927ad37522b6cb3e3061d"),
)


def scene_fingerprint(catalog, scene, *, recipe_sha256=None):
    files = [(frame_id, catalog.frame(frame_id).source_sha256)
             for frame_id in (*scene.frame_ids, *scene.reference_ids)]
    return sha256(json.dumps([recipe_sha256 or scene.metadata["recipe_sha256"], files]).encode()).hexdigest()


def compatible_scene_fingerprints(catalog, scene):
    fingerprints = {scene_fingerprint(catalog, scene)}
    recipe = scene.metadata["recipe"]
    current = recipe["converter_sha256"], recipe["loader_sha256"]
    if current in _COMPATIBLE_RECIPE_CODE:
        for converter, loader in _COMPATIBLE_RECIPE_CODE:
            if (converter, loader) == current:
                continue
            old_recipe = {**recipe, "converter_sha256": converter, "loader_sha256": loader}
            old_sha256 = sha256(json.dumps(old_recipe, sort_keys=True).encode()).hexdigest()
            fingerprints.add(scene_fingerprint(catalog, scene, recipe_sha256=old_sha256))
    return fingerprints


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
            raise RuntimeError(f"Another Stackroom server is using {directory}") from exc
        yield


def _render(tool, frames, work, log, stop):
    if tool.id == "zerene":
        preferences = _zerene.preferences_directory()
        destination = work / "zerene"
        destination.mkdir()
        batch = destination / "batch.xml"
        options = dict(tool.options)
        method = options.pop("method")
        _zerene.write_batch(batch, destination, (method,), **options)
        command = _zerene.batch_command(_zerene.launch_command(preferences), batch, (work / frame for frame in frames))
        with _zerene.batch_lock(preferences):
            run_logged(command, work, log, stop, tool.label, timeout=3600)
        return _zerene.batch_output(destination, method)
    delivery = f"{tool.id}.tif"
    run_logged(tool.command(frames, delivery), work, log, stop, tool.label)
    return work / delivery


class Runs:
    def __init__(self, store, stackers, on_change=lambda: None, *, record_score=None):
        self.root = store / "runs"
        self.stackers = stackers
        self.setup = StackerSetup(store)
        self.worker = None
        self.active = None
        self.stop = RunSignal()
        self.on_change = on_change
        self.record_score = record_score
        self.lock = Lock()
        for path in self.root.glob("*/progress.json"):
            progress = _read(path)
            if progress["status"] == "running":
                _write(path, {**progress, "status": "interrupted", "finished_at": _now().isoformat()})

    def tooling(self):
        return [{"id": tool.id, "label": tool.label, "method": tool.method,
                 "available": self.setup.available(tool) is not None,
                 "options": tool.describe_options(),
                 **({"setup_hint": _zerene.INSTALL_HINT} if tool.id == "zerene" else {})}
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
        fingerprints = compatible_scene_fingerprints(catalog, scene) if selected else set()
        rows = []
        for stacker in run["stackers"]:
            path = base / stacker / "results" / f"{scene.id}.json"
            if not selected:
                row = {"status": "outside_scope"}
            elif path.is_file():
                row = _read(path)
                if row["scene_fingerprint"] not in fingerprints:
                    row = {"status": "different_source"}
                elif row["status"] == "ready" and not (base / row["image"]).is_file():
                    row = {"status": "missing_file"}
            elif error := progress.get("setup_errors", {}).get(stacker):
                row = {"status": "failed", **error}
            else:
                row = {"status": "pending" if progress["status"] == "running" else "unfinished"}
            rows.append({"stacker": stacker, **row})
        return rows

    def start(self, catalog, scene_ids, stacker_ids, scope, options=None):
        if options is None:
            options = {}
        with self.lock:
            if self.worker is not None and self.worker.is_alive():
                raise RuntimeError("An evaluation is already running")
            if not stacker_ids or any(key not in self.stackers for key in stacker_ids):
                raise ValueError("Select at least one supported stacker")
            if not isinstance(options, dict) or options.keys() - set(stacker_ids):
                raise ValueError("Settings must belong to the selected stacking programs")
            tools = [self.stackers[key].with_options(options.get(key, {})) for key in dict.fromkeys(stacker_ids)]
            started = _now()
            run_id = started.strftime("%Y%m%dT%H%M%S%fZ-") + uuid4().hex[:8]
            base = self.directory(run_id)
            base.mkdir(parents=True, exist_ok=False)
            run = {"schema_version": 1, "id": run_id, "created_at": started.isoformat(),
                   "scope": scope, "scene_ids": scene_ids,
                   "stackers": [tool.id for tool in tools],
                   "methods": {tool.id: tool.method for tool in tools},
                   "options": {tool.id: tool.options for tool in tools},
                   "input_format": "16-bit SDR sRGB TIFF",
                   "output_format": "float32 linear-sRGB RGB TIFF",
                   "explain_score": self.record_score is not None}
            progress = {"status": "running", "total": len(scene_ids) * len(tools),
                        "ready": 0, "failed": 0, "diagnostics_failed": 0, "current_scene": None,
                        "setup_errors": {}, "prepared": []}
            _write(base / "progress.json", progress)
            _write(base / "run.json", run)
            self.on_change()
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

    def _save_progress(self, base, progress):
        _write(base / "progress.json", progress)
        self.on_change()

    def _worker(self, base, catalog, scene_ids, tools, progress):
        try:
            tools = self._prepare_tools(base, tools, len(scene_ids), progress)
            for scene_id in scene_ids if tools else ():
                self._check_stop()
                progress["current_scene"] = scene_id
                self._save_progress(base, progress)
                self._evaluate_scene(base, catalog, catalog.scene(scene_id), tools, progress)
            progress["status"] = "completed_with_errors" if progress["failed"] or progress["diagnostics_failed"] else "completed"
        except RunStopped:
            progress["status"] = "cancelled"
        except Exception as exc:
            progress.update(status="error", error=str(exc))
        finally:
            progress.update(finished_at=_now().isoformat(), current_scene=None, setup=None, scoring=None)
            self._save_progress(base, progress)

    def _prepare_tools(self, base, tools, scene_count, progress):
        ready = []
        for tool in tools:
            self._check_stop()
            log = base / tool.id / "setup.log"

            def tell(message):
                progress["setup"] = {"stacker": tool.id, "message": message}
                self._save_progress(base, progress)

            try:
                path = self.setup.prepare(tool, log, self.stop, tell)
                ready.append(replace(tool, executable=path))
                progress["prepared"].append(tool.id)
            except RunStopped:
                raise
            except Exception as exc:
                log.parent.mkdir(parents=True, exist_ok=True)
                with log.open("a", encoding="utf-8") as stream:
                    stream.write(f"\n{exc}\n")
                progress["setup_errors"][tool.id] = {
                    "error": f"Could not set up {tool.label}. {exc}",
                    "log": log.relative_to(base).as_posix(),
                }
                progress["failed"] += scene_count
                self._save_progress(base, progress)
        progress["setup"] = None
        self._save_progress(base, progress)
        return ready

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
                if self.record_score and row["status"] == "ready" and has_benchmark_reference(scene):
                    progress["scoring"] = tool.id
                    self._save_progress(base, progress)
                    row.update(self.record_score(catalog, scene, base, row))
                    progress["scoring"] = None
                    progress["diagnostics_failed"] += int("score_explanation_error" in row)
                _write(base / tool.id / "results" / f"{scene.id}.json", row)
                progress[row["status"]] += 1
                self._save_progress(base, progress)
