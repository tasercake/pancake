"""Trace one actual Vulkan exposure job and attribute process mappings.

Diagnostic only. Sampling and ptrace can affect elapsed time and memory.
"""
from collections import defaultdict
from hashlib import sha256
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

binary, inputs, output = map(lambda item: Path(item).resolve(), sys.argv[1:4])
mode = sys.argv[4] if len(sys.argv) > 4 else "smaps"
backend = "cpu" if mode == "cpu" else "gpu"
output.mkdir(parents=True, exist_ok=False)
sources = sorted(inputs.glob("DSC_*.png"))
assert [path.stem for path in sources] == [f"DSC_{index:04}" for index in range(163, 172)]
digests = lambda paths: {str(path): sha256(path.read_bytes()).hexdigest() for path in paths}
source_hashes = digests(sources)
input_receipt = json.loads(Path(__file__).with_name("input-receipt.json").read_text())
expected_files = {
    Path(frame["path"]).name: (frame["sha256"], frame["bytes"])
    for frame in input_receipt["frames"]
}
assert {
    path.name: (source_hashes[str(path)], path.stat().st_size)
    for path in sources
} == expected_files
image = output / "view.tif"
events = output / "events.jsonl"
selection = ["--cpu"] if backend == "cpu" else ["--gpu", "--gpu-id", "0"]
argv = [str(binary), "--input", *map(str, sources), "--output", str(image), *selection,
        "--method", "exposure_fusion", "--align", "--alignment-mode",
        "euclidean", "--alignment-ref", "first", "--kernel-size", "6", "--pyramid-levels", "8",
        "--scale-factor", "6", "--slab-size", "0", "--slab-overlap", "0", "--bit-depth", "16", "--auto-crop"]
env = {**os.environ, "PANCAKE_RUNTIME": "wgpu", "PANCAKE_COMPUTE_CACHE_DIR": str(output / "cache"),
       "CONFORMANCE_EVENTS_PATH": str(events)}
if backend == "cpu":
    env.pop("PANCAKE_RUNTIME", None)
for name in ("DISPLAY", "WAYLAND_DISPLAY", "CUBECL_DEBUG_LOG", "CUBECL_DEBUG_OPTION"):
    env.pop(name, None)
trace = ["strace", "-ff", "-tt", "-e", "trace=openat,openat2,mmap,munmap", "-s", "224", "-o", str(output / "driver")] if mode == "strace" else []
sampling = []
header = re.compile(r"^[0-9a-f]+-[0-9a-f]+ [rwxps-]{4} [0-9a-f]+ [0-9a-f]+:[0-9a-f]+ [0-9]+\s*(.*)$")
def mapped_rss(path):
    sections = defaultdict(int)
    current = "[anonymous]"
    for row in path.read_text().splitlines():
        match = header.match(row)
        if match:
            name = match.group(1).strip()
            current = Path(name).name if name.startswith("/") else name or "[anonymous]"
        elif row.startswith("Rss:"):
            sections[current] += int(row.split()[1])
    return dict(sorted(sections.items(), key=lambda item: item[1], reverse=True))

start = time.monotonic_ns()
with (output / "stdout.log").open("wb") as stdout, (output / "stderr.log").open("wb") as stderr:
    with subprocess.Popen([*trace, *argv], env=env, stdout=stdout, stderr=stderr) as process:
        while True:
            flags = 0 if mode in ("plain", "cpu") else os.WNOHANG
            pid, status, usage = os.wait4(process.pid, flags)
            if pid:
                process.returncode = os.waitstatus_to_exitcode(status)
                break
            if mode == "smaps":
                try:
                    proc = Path(f"/proc/{process.pid}")
                    at = time.monotonic_ns()
                    mapped = mapped_rss(proc / "smaps")
                    direct = {row.partition(":")[0]: row.partition(":")[2].strip()
                              for row in (proc / "status").read_text().splitlines()
                              if row.split(":")[0] in ("VmRSS", "VmHWM", "Threads")}
                    if mapped:
                        sampling.append({"elapsed_ms": (at - start) / 1e6, "rss_by_mapping_kib": mapped,
                                         "totals_kib": sum(mapped.values()), "status": direct})
                except FileNotFoundError:
                    pass
            time.sleep(0.010)
elapsed = (time.monotonic_ns() - start) / 1e9
assert process.returncode == 0, (process.returncode, (output / "stderr.log").read_text())
event = json.loads(events.read_text().splitlines()[-1])
assert event["completed"] is True and event["actual_backend"] == backend
if backend == "gpu":
    assert event["physical_gpu"] is True and event["runtime"] == "wgpu-vulkan" and not event["fallback_reason"]
    assert event["device"] == "NVIDIA GeForce RTX 3090"
    assert event["device_pci_address"] == "0000:01:00.0"
else:
    assert event["runtime"] == "cpu" and not event["physical_gpu"]
assert digests(sources) == source_hashes
receipt = {"scope": "instrumented, concurrent GUI and CPU work allowed; no performance gate credit",
           "mode": mode, "argv": argv, "binary_sha256": sha256(binary.read_bytes()).hexdigest(),
           "inputs": source_hashes, "events": event, "encoded_sha256": sha256(image.read_bytes()).hexdigest(),
           "duration_seconds": elapsed, "ru_maxrss_kib": usage.ru_maxrss,
           "user_seconds": usage.ru_utime, "system_seconds": usage.ru_stime, "mappings": sampling}
(output / "observation.json").write_text(json.dumps(receipt, indent=2) + "\n")
peak = max(sampling, key=lambda sample: sample["totals_kib"], default={})
print(json.dumps({key: receipt[key] for key in ("mode", "binary_sha256", "duration_seconds", "ru_maxrss_kib",
                                                "user_seconds", "system_seconds")}))
print(json.dumps({"at_peak": {**peak, "rss_by_mapping_kib": dict(list(peak.get("rss_by_mapping_kib", {}).items())[:16])}}))
image.unlink()
