"""Run the verified downloader in the background and summarize its transfer events."""

from collections import deque
from contextlib import redirect_stdout
import json
import subprocess
import sys
from threading import Lock, Thread
from time import monotonic

from ..data import download


class Downloads:
    def __init__(self, root, config, lock, on_dataset_ready, on_change):
        self.root = root
        self.config = config
        self.source_lock = lock
        self.on_dataset_ready = on_dataset_ready
        self.on_change = on_change
        self.mutex = Lock()
        self.process = None
        self.worker = None
        self.closed = False
        self.status = "idle"
        self.error = None
        self.files = {}
        self.samples = deque()
        self.transferred = 0

    def start(self, datasets):
        with self.mutex:
            if self.closed:
                raise RuntimeError("Review is shutting down")
            if self.status == "running":
                raise RuntimeError("A dataset download is already running")
            if not datasets:
                raise ValueError("There are no missing datasets to download")
            self.status = "running"
            self.error = None
            self.files.clear()
            self.samples.clear()
            self.transferred = 0
            self.worker = Thread(target=self._run, args=(datasets,), daemon=True,
                                 name="bokehforge-download")
            self.worker.start()
        self.on_change()

    def _run(self, datasets):
        command = [sys.executable, "-m", "bokehforge.review.downloads",
                   "--root", str(self.root), "--config", str(self.config)]
        if self.source_lock:
            command += ["--lock", str(self.source_lock)]
        for dataset in datasets:
            command += ["--dataset", dataset]
        try:
            with self.mutex:
                if self.closed:
                    return
                self.process = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                                stdout=subprocess.PIPE, text=True, encoding="utf-8")
                process = self.process
            with process.stdout:
                for line in process.stdout:
                    self._event(json.loads(line))
            succeeded = process.wait() == 0
            with self.mutex:
                self.status = "complete" if succeeded else "failed"
                if not succeeded and not self.error:
                    self.error = "Download stopped before all files were verified."
        except (OSError, ValueError) as exc:
            with self.mutex:
                self.status = "failed"
                self.error = str(exc)
        finally:
            self.on_change()

    def _event(self, event):
        kind = event["type"]
        if kind == "dataset_ready":
            self.on_dataset_ready()
            return
        with self.mutex:
            if kind == "inventory":
                self.files = {(item["dataset"], item["path"]): {**item, "phase": "queued", "received": 0}
                              for item in event["files"]}
            elif kind == "error":
                self.error = event["message"]
            elif kind == "file":
                previous = self.files[event["dataset"], event["path"]]
                if event["phase"] == "downloading":
                    if previous["phase"] == "downloading":
                        self.transferred += max(0, event["received"] - previous["received"])
                    now = monotonic()
                    self.samples.append((now, self.transferred))
                    while len(self.samples) > 1 and self.samples[1][0] < now - 10:
                        self.samples.popleft()
                previous.update(phase=event["phase"], received=event["received"],
                                total=event["total"] if event["total"] is not None else previous["total"])
        self.on_change()

    def snapshot(self):
        with self.mutex:
            files = list(self.files.values())
            received = sum(item["received"] for item in files)
            total = (sum(item["total"] for item in files)
                     if files and all(item["total"] is not None for item in files) else None)
            phases = {item["phase"] for item in files}
            rate = None
            if "downloading" in phases and len(self.samples) > 1:
                now = monotonic()
                start, count = self.samples[0]
                if now - self.samples[-1][0] < 4 and now - start >= 0.5 and self.transferred > count:
                    rate = (self.transferred - count) / (now - start)
            phase = next((phase for phase in ("downloading", "retrying", "verifying", "checking")
                          if phase in phases), "preparing")
            remaining = max(0, total - received) / rate if total is not None and rate else None
            return {"status": self.status, "phase": phase, "received": received, "total": total,
                    "rate": rate, "seconds_remaining": remaining, "error": self.error}

    def close(self):
        with self.mutex:
            self.closed = True
            process = self.process
        if process is not None and process.returncode is None:
            process.terminate()
        if self.worker is not None:
            self.worker.join(timeout=5)
            if self.worker.is_alive() and process is not None:
                process.kill()
                self.worker.join()


def main():
    # Keep CLI diagnostics in the service log and send only structured events
    # over stdout. The downloader continues to write its regular local log.
    pipe = sys.stdout
    mutex = Lock()

    def emit(event):
        with mutex:
            print(json.dumps(event), file=pipe, flush=True)

    try:
        with redirect_stdout(sys.stderr):
            return download.main(sys.argv[1:], progress=emit)
    except (OSError, RuntimeError, ValueError) as exc:
        emit({"type": "error", "message": str(exc)})
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
