"""Linux whole-job timing and process-attributed resource counters."""

import ctypes as c
import json
import os
import signal
import subprocess
import threading
import time
from contextlib import ExitStack
from dataclasses import dataclass
from itertools import chain
from pathlib import Path

_SAMPLE_NS = 8_000_000
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
        self.rows = (_DeviceProcess * 32)()
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
        count = c.c_uint(len(self.rows))
        result = getter(self.handle, c.byref(count), self.rows)
        if result == _INSUFFICIENT_SIZE:
            self.rows = (_DeviceProcess * (count.value + 16))()
            count.value = len(self.rows)
            result = getter(self.handle, c.byref(count), self.rows)
        self._check(result, operation)
        return self.rows[: count.value]

    def sample(self, pids):
        _, total = self._sample(pids.__contains__)
        return total

    def sample_session(self, session):
        def member(pid):
            try:
                return os.getsid(pid) == session
            except ProcessLookupError:
                return False

        return self._sample(member)

    def _sample(self, member):
        totals = {}
        for group in ("Compute", "Graphics"):
            query = getattr(self.lib, f"nvmlDeviceGet{group}RunningProcesses_v3")
            for row in self._query(query, group):
                if member(row.pid):
                    if row.memory_bytes == _UNAVAILABLE_MEMORY:
                        return set(totals) | {row.pid}, None
                    # The same driver's per-process total may appear in both lists.
                    totals[row.pid] = max(totals.get(row.pid, 0), row.memory_bytes)
        return set(totals), sum(totals.values())

    def close(self):
        self._check(self.lib.nvmlShutdown(), "NVML shutdown")


def _session_memory(session):
    """Sum RSS of the child session, excluding the harness and OS background tasks."""
    pids = set()
    pages = 0
    with os.scandir("/proc") as processes:
        for entry in processes:
            if not entry.name.isdecimal():
                continue
            pid = int(entry.name)
            try:
                if os.getsid(pid) != session:
                    continue
                with open(entry.path + "/stat", "rb") as stat:
                    fields = stat.read().rsplit(b")", 1)[1].split()
            except (FileNotFoundError, ProcessLookupError):
                continue  # Process exited during collection.
            # Recheck session in case the PID exited between getsid() and open().
            if int(fields[3]) == session and fields[0] != b"Z":
                pids.add(pid)
                pages += max(0, int(fields[21]))
    return pids, pages * os.sysconf("SC_PAGESIZE")


def _signal_group(pid, code):
    try:
        os.killpg(pid, code)
    except ProcessLookupError:
        pass


@dataclass
class _Counter:
    count: int = 0
    last_ns: int = 0
    gap_ns: int = 0
    peak: int | None = None
    unavailable: bool = False

    def observe(self, elapsed_ns, value):
        self.count += 1
        self.gap_ns = max(self.gap_ns, elapsed_ns - self.last_ns)
        self.last_ns = elapsed_ns
        if value is None:
            self.unavailable = True
        else:
            self.peak = value if self.peak is None else max(self.peak, value)


def _summarize_samples(rows, elapsed, *, gpu, errors):
    counters = {"rss": _Counter()}
    if gpu:
        counters["gpu"] = _Counter()
    result = {"max_sample_acquisition_ns": 0}
    for row in rows:
        for name, counter in counters.items():
            at = row.get(f"{name}_elapsed_ns")
            if at is not None and at <= elapsed:
                counter.observe(at, row[f"{name}_bytes"])
                result["max_sample_acquisition_ns"] = max(
                    result["max_sample_acquisition_ns"], at - row.get("started_ns", at)
                )
    for name, counter in counters.items():
        gap = max(counter.gap_ns, elapsed - counter.last_ns)
        adequate = gap <= _MAX_SAMPLE_GAP_NS and not (counter.unavailable or errors)
        result[f"maximum_{name}_sample_gap_ns"] = gap
        result[f"{name}_sample_count"] = counter.count
        result[f"observed_peak_{name}_bytes"] = counter.peak
        result[f"peak_{name}_bytes"] = (
            counter.peak if adequate and counter.peak else None
        )
    if not gpu:
        result.update(
            maximum_gpu_sample_gap_ns=None,
            gpu_sample_count=0,
            observed_peak_gpu_bytes=None,
            peak_gpu_bytes=None,
        )
    result["maximum_sample_gap_ns"] = max(
        result["maximum_rss_sample_gap_ns"], result["maximum_gpu_sample_gap_ns"] or 0
    )
    result["sample_count"] = result["rss_sample_count"]
    return result


def measure(argv, *, cwd, env, directory, timeout, gpu, rss_sampler_cpu=None):
    """Measure direct spawn-to-exit, record sampled process-tree and GPU peaks."""
    if timeout <= 0 or not Path("/proc/self/stat").is_file():
        raise ValueError("positive timeout and Linux procfs required")
    if rss_sampler_cpu is not None and (
        rss_sampler_cpu < 0 or rss_sampler_cpu in os.sched_getaffinity(0)
    ):
        raise ValueError("RSS sampler CPU must be outside the common job affinity")
    traces = {"rss": (directory / "resources.jsonl", _session_memory)}
    if gpu is not None:
        traces["gpu"] = (directory / "gpu-resources.jsonl", gpu.sample_session)
    stop = threading.Event()
    expired = threading.Event()
    launched = threading.Event()
    sample_failures = []
    sampler_cpu_ns = {}
    rss_sampler_affinity = None
    child = None
    start = 0

    def watchdog():
        launched.wait()
        if child is None:
            return
        remaining = max(0, timeout - (time.monotonic_ns() - start) / 1e9)
        if not stop.wait(remaining):
            expired.set()
            _signal_group(child.pid, signal.SIGTERM)
            if not stop.wait(2):
                _signal_group(child.pid, signal.SIGKILL)

    def collect(metric, path, snapshot, ready):
        nonlocal rss_sampler_affinity
        cpu_start = time.thread_time_ns()
        try:
            with path.open("x") as raw:
                if metric == "rss":
                    if rss_sampler_cpu is not None:
                        os.sched_setaffinity(0, {rss_sampler_cpu})
                    rss_sampler_affinity = sorted(os.sched_getaffinity(0))
                ready.set()
                launched.wait()
                if child is None:
                    return
                due = start
                while not stop.is_set():
                    sample_start = time.monotonic_ns()
                    snapshot_cpu_start = time.thread_time_ns()
                    pids, value = snapshot(child.pid)
                    snapshot_cpu = time.thread_time_ns() - snapshot_cpu_start
                    elapsed_ns = time.monotonic_ns() - start
                    raw.write(
                        json.dumps(
                            {
                                "elapsed_ns": elapsed_ns,
                                "scheduled_ns": due - start,
                                "started_ns": sample_start - start,
                                "snapshot_thread_cpu_ns": snapshot_cpu,
                                f"{metric}_elapsed_ns": elapsed_ns,
                                "processes": sorted(pids),
                                f"{metric}_bytes": value,
                            }
                        )
                        + "\n"
                    )
                    now = time.monotonic_ns()
                    # Go to the next absolute slot after a late wake without a catch-up burst.
                    due += max(1, (now - due) // _SAMPLE_NS + 1) * _SAMPLE_NS
                    stop.wait(max(0, due - now) / 1e9)
        except (IndexError, OSError, ValueError) as error:
            sample_failures.append(f"{metric}: {error}")
        finally:
            sampler_cpu_ns[metric] = time.thread_time_ns() - cpu_start
            ready.set()

    samplers = []
    for metric, (path, snapshot) in traces.items():
        ready = threading.Event()
        samplers.append(
            (
                threading.Thread(target=collect, args=(metric, path, snapshot, ready)),
                ready,
            )
        )
    timer = threading.Thread(target=watchdog)
    with (
        (directory / "stdout.txt").open("xb") as output,
        (directory / "stderr.txt").open("xb") as errors,
    ):
        try:
            for sampler, _ in samplers:
                sampler.start()
            timer.start()
            for _, ready in samplers:
                ready.wait()
            if rss_sampler_cpu is not None and rss_sampler_affinity != [
                rss_sampler_cpu
            ]:
                raise ValueError(
                    f"RSS sampler could not use CPU {rss_sampler_cpu} "
                    f"(effective mask {rss_sampler_affinity}, errors {sample_failures})"
                )
            start = time.monotonic_ns()
            child = subprocess.Popen(
                argv,
                cwd=cwd,
                env=env,
                stdout=output,
                stderr=errors,
                start_new_session=True,
            )
            launched.set()
            _, wait_status, usage = os.wait4(child.pid, 0)
            elapsed = time.monotonic_ns() - start
            # Popen shares the actual reaped exit status with cleanup.
            status = os.waitstatus_to_exitcode(wait_status)
            child.returncode = status
        finally:
            stop.set()
            launched.set()
            if child is not None and child.poll() is None:
                _signal_group(child.pid, signal.SIGKILL)
                child.wait()
            for sampler, _ in samplers:
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
    with ExitStack() as receipts:
        streams = [
            receipts.enter_context(path.open())
            for path, _ in traces.values()
            if path.is_file()
        ]
        summary = _summarize_samples(
            (json.loads(line) for line in chain.from_iterable(streams)),
            elapsed,
            gpu=gpu is not None,
            errors=sample_failures,
        )
    return {
        "returncode": status,
        "timed_out": expired.is_set(),
        "elapsed_ns": elapsed,
        **summary,
        "kernel_process_peak_rss_bytes": usage.ru_maxrss * 1024,
        "kernel_process_user_cpu_seconds": usage.ru_utime,
        "kernel_process_system_cpu_seconds": usage.ru_stime,
        "resource_errors": sample_failures,
        "sample_interval_ns": _SAMPLE_NS,
        "rss_sampler_affinity": rss_sampler_affinity,
        "sampler_thread_cpu_ns": sum(sampler_cpu_ns.values()),
        "sampler_threads_cpu_ns": sampler_cpu_ns,
    }
