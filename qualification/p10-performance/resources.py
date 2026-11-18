"""Linux whole-job timing and process-attributed resource counters."""

import ctypes as c
import json
import os
import signal
import subprocess
import threading
import time
from pathlib import Path

_SAMPLE_SECONDS = 0.010
_MAX_SAMPLE_GAP_NS = 20_000_000
_UNAVAILABLE_MEMORY = (1 << 64) - 1
_INSUFFICIENT_SIZE = 7


class _DeviceProcess(c.Structure):
    # nvmlProcessInfo_t from the NVIDIA management-library header.
    _fields_ = [
        ("pid", c.c_uint),
        ("memory_bytes", c.c_ulonglong),
        ("gpu_instance", c.c_uint),
        ("compute_instance", c.c_uint),
    ]


class NvidiaMemory:
    """Query both compute and graphics memory for one physical GPU UUID."""

    def __init__(self, uuid):
        self.lib = c.CDLL("libnvidia-ml.so.1")
        self.handle = c.c_void_p()
        self._check(self.lib.nvmlInit_v2(), "NVML initialization")
        try:
            self._check(
                self.lib.nvmlDeviceGetHandleByUUID(uuid.encode(), c.byref(self.handle)),
                f"GPU {uuid}",
            )
        except ValueError:
            self.close()
            raise

    @staticmethod
    def _check(result, operation):
        if result != 0:
            raise ValueError(f"{operation} failed with NVML status {result}")

    def _query(self, getter, operation):
        count = c.c_uint()
        result = getter(self.handle, c.byref(count), None)
        if result == 0 and count.value == 0:
            return []
        if result != _INSUFFICIENT_SIZE:
            self._check(result, operation)
        # Capacity margin covers processes created between enumeration and reading.
        rows = (_DeviceProcess * (count.value + 16))()
        count.value = len(rows)
        self._check(getter(self.handle, c.byref(count), rows), operation)
        return rows[: count.value]

    def sample(self, pids):
        totals = {}
        for group in ("Compute", "Graphics"):
            query = getattr(self.lib, f"nvmlDeviceGet{group}RunningProcesses_v3")
            for row in self._query(query, group):
                if row.pid in pids:
                    if row.memory_bytes == _UNAVAILABLE_MEMORY:
                        return None
                    # The same driver's per-process total may appear in both lists.
                    totals[row.pid] = max(totals.get(row.pid, 0), row.memory_bytes)
        return sum(totals.values())

    def close(self):
        self._check(self.lib.nvmlShutdown(), "NVML shutdown")


def _session_memory(session):
    """Sum RSS of the child session, excluding the harness and OS background tasks."""
    pids = set()
    pages = 0
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            fields = (
                (entry / "stat").read_text(errors="replace").rsplit(")", 1)[1].split()
            )
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue  # Process exited during collection or is inaccessible to this boundary.
        if int(fields[3]) == session and fields[0] != "Z":
            pids.add(int(entry.name))
            pages += max(0, int(fields[21]))
    return pids, pages * os.sysconf("SC_PAGESIZE")


def _signal_group(pid, code):
    try:
        os.killpg(pid, code)
    except ProcessLookupError:
        pass


def measure(argv, *, cwd, env, directory, timeout, gpu):
    """Measure direct spawn-to-exit, record sampled process-tree and GPU peaks."""
    if timeout <= 0 or not Path("/proc/self/stat").is_file():
        raise ValueError("positive timeout and Linux procfs required")
    resource_path = directory / "resources.jsonl"
    stop = threading.Event()
    expired = threading.Event()
    sample_failures = []
    samples = 0
    last_observation = 0
    largest_gap = 0
    peak_rss = 0
    peak_gpu = 0 if gpu is not None else None

    def watchdog():
        if not stop.wait(timeout):
            expired.set()
            _signal_group(child.pid, signal.SIGTERM)
            if not stop.wait(2):
                _signal_group(child.pid, signal.SIGKILL)

    def collect():
        nonlocal samples, last_observation, largest_gap, peak_rss, peak_gpu
        try:
            with resource_path.open("x") as raw:
                due = time.monotonic()
                while not stop.is_set():
                    pids, rss = _session_memory(child.pid)
                    gpu_bytes = gpu.sample(pids) if gpu is not None else None
                    row = {
                        "elapsed_ns": time.monotonic_ns() - start,
                        "processes": sorted(pids),
                        "rss_bytes": rss,
                        "gpu_bytes": gpu_bytes,
                    }
                    samples += 1
                    observed = row["elapsed_ns"]
                    largest_gap = max(largest_gap, observed - last_observation)
                    last_observation = observed
                    peak_rss = max(peak_rss, rss)
                    peak_gpu = (
                        max(peak_gpu, gpu_bytes)
                        if peak_gpu is not None and gpu_bytes is not None
                        else None
                    )
                    raw.write(json.dumps(row) + "\n")
                    due += _SAMPLE_SECONDS
                    stop.wait(max(0, due - time.monotonic()))
        except (OSError, ValueError) as error:
            sample_failures.append(str(error))

    sampler = threading.Thread(target=collect)
    timer = threading.Thread(target=watchdog)
    with (
        (directory / "stdout.txt").open("xb") as output,
        (directory / "stderr.txt").open("xb") as errors,
    ):
        start = time.monotonic_ns()
        child = subprocess.Popen(
            argv, cwd=cwd, env=env, stdout=output, stderr=errors, start_new_session=True
        )
        try:
            sampler.start()
            timer.start()
            _, wait_status, usage = os.wait4(child.pid, 0)
            elapsed = time.monotonic_ns() - start
            # Popen shares the actual reaped exit status with cleanup.
            status = os.waitstatus_to_exitcode(wait_status)
            child.returncode = status
        finally:
            stop.set()
            if child.poll() is None:
                _signal_group(child.pid, signal.SIGKILL)
                child.wait()
            if sampler.ident is not None:
                sampler.join()
            if timer.ident is not None:
                timer.join()
    remaining, _ = _session_memory(child.pid)
    if remaining:
        _signal_group(child.pid, signal.SIGKILL)
        sample_failures.append(
            f"process session still alive after CLI exit: {sorted(remaining)}"
        )
    largest_gap = max(largest_gap, elapsed - last_observation)
    adequate_sampling = largest_gap <= _MAX_SAMPLE_GAP_NS and not sample_failures
    return {
        "returncode": status,
        "timed_out": expired.is_set(),
        "elapsed_ns": elapsed,
        "peak_rss_bytes": peak_rss if adequate_sampling and peak_rss else None,
        "peak_gpu_bytes": peak_gpu if adequate_sampling and peak_gpu else None,
        "observed_peak_gpu_bytes": peak_gpu,
        "kernel_process_peak_rss_bytes": usage.ru_maxrss * 1024,
        "kernel_process_user_cpu_seconds": usage.ru_utime,
        "kernel_process_system_cpu_seconds": usage.ru_stime,
        "maximum_sample_gap_ns": largest_gap,
        "resource_errors": sample_failures,
        "sample_count": samples,
    }
