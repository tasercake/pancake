"""Hidden runtime files and command logging."""

import contextlib
import json
import os
import sys
import threading

from ._files import safe_path

METADATA = ".bokehforge"


def matches_pins(source, policy):
    if source.get("revision") != policy.get("revision"):
        return False
    return {item["path"]: item.get("sha256") for item in source.get("files", ())} == {
        item["path"]: item.get("sha256") for item in policy["files"]
    }


def local_source(root, dataset_id, *, snapshot=None, pins=None):
    """Find a published source with all its native files present."""
    raw = safe_path(root / "raw", dataset_id)
    if snapshot is None:
        manifest = raw / "download-manifest.json"
        if not manifest.is_file():
            return None
        snapshot = json.loads(manifest.read_text())
    files = snapshot.get("files", ())
    if (snapshot.get("id") != dataset_id or snapshot.get("status") != "available" or not files
            or (pins is not None and not matches_pins(snapshot, pins))):
        return None
    if all((asset := safe_path(raw, item["path"])).is_file()
           and (item.get("size") is None or asset.stat().st_size == item["size"]) for item in files):
        return snapshot
    return None


@contextlib.contextmanager
def exclusive(root, name):
    root.mkdir(parents=True, exist_ok=True)
    with (root / name).open("a+b") as handle:
        try:
            if os.name == "nt":
                import msvcrt
                handle.seek(0)
                msvcrt.locking(handle.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            raise RuntimeError(f"Another process holds {root / name}") from None
        yield


class _Tee:
    def __init__(self, console, log, mutex):
        self.console, self.log, self.mutex = console, log, mutex

    def write(self, value):
        with self.mutex:
            self.log.write(value)
            return self.console.write(value)

    def flush(self):
        with self.mutex:
            self.log.flush()
            self.console.flush()

    def __getattr__(self, name):
        return getattr(self.console, name)


@contextlib.contextmanager
def data_run(root, action):
    private = root / METADATA
    with exclusive(private / "locks", "data.lock"):
        for directory in ["logs", "reports", "probes", "work"]:
            (private / directory).mkdir(parents=True, exist_ok=True)
        with (private / "logs" / f"{action}.log").open("a", encoding="utf-8", buffering=1) as log:
            log.write(f"\n=== bokehforge data {action} ===\n")
            mutex = threading.Lock()
            try:
                yield _Tee(sys.stdout, log, mutex), _Tee(sys.stderr, log, mutex)
            except Exception as exc:
                log.write(f"ERROR: {exc}\n")
                raise
