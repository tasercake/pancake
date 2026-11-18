"""Compare stock and experimental lock without changing driver configuration."""
import json
from pathlib import Path
import re
import statistics
import subprocess
import sys

root = Path(__file__).resolve().parent
stock, patch, photos, output = map(Path, sys.argv[1:])
output.mkdir(exist_ok=False)
measurements = {"stock": [], "patched": []}
paths = {"stock": stock, "patched": patch}


def run(name, variant, mode):
    case = output / name
    subprocess.check_call([sys.executable, str(root / "observe.py"), str(paths[variant]), str(photos), str(case), mode])
    return json.loads((case / "observation.json").read_bytes())


for mode in ("strace", "smaps", "cpu"):
    for variant in paths:
        run(f"{mode}-{variant}", variant, mode)
for index in range(4):
    order = list(paths) if index % 2 == 0 else list(reversed(paths))
    for variant in order:
        run_data = run(f"plain-{variant}-{index}", variant, "plain")
        if index:
            measurements[variant].append({key: run_data[key] for key in (
                "duration_seconds", "ru_maxrss_kib", "encoded_sha256", "binary_sha256")})

library = re.compile(r'openat\(AT_FDCWD, "/lib/x86_64-linux-gnu/libvulkan.so.1", .*\) = [0-9]+')
loaders = {
    variant: sum(len(library.findall(file.read_text())) for file in (output / f"strace-{variant}").glob("driver.*"))
    for variant in paths
}
assert loaders == {"stock": 2, "patched": 1}, loaders
assert len({row["encoded_sha256"] for rows in measurements.values() for row in rows}) == 1
cpu = [json.loads((output / f"cpu-{variant}" / "observation.json").read_bytes()) for variant in paths]
assert len({item["encoded_sha256"] for item in cpu}) == 1
summary = {"scope": "experimental diagnostics only; normal quiet, thermal and memory cadence gates were not applied",
           "fresh_vulkan_loader_opens": loaders,
           "samples": measurements,
           "median_seconds": {label: statistics.median(row["duration_seconds"] for row in items)
                              for label, items in measurements.items()},
           "max_rss_kib": {label: max(row["ru_maxrss_kib"] for row in items)
                           for label, items in measurements.items()},
           "cpu_pixel_comparison": "byte-identical before and after"}
(output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
print(json.dumps(summary, indent=2))
