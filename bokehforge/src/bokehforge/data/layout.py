"""Hidden runtime files and command logging."""

import contextlib
import sys
import threading

METADATA = ".bokehforge"


@contextlib.contextmanager
def exclusive(root, name):
    import fcntl
    root.mkdir(parents=True, exist_ok=True)
    with (root / name).open("a") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
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
            with contextlib.redirect_stdout(_Tee(sys.stdout, log, mutex)), contextlib.redirect_stderr(_Tee(sys.stderr, log, mutex)):
                try:
                    yield private
                except Exception as exc:
                    log.write(f"ERROR: {exc}\n")
                    raise
