# Linux RSS and NVIDIA cadence follow-up

This investigation ran on November 18, 2026, from qualification source
`2143eda2b308497caee96334357dd23d324e1e8e`. It used the Ryzen 9 7950X3D
and physical RTX 3090, driver 580.178.04 at PCI `0000:01:00.0` and UUID
`GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`. The probes keep the original
sampled aggregates and 20 ms deadline. They are separate from the frozen
warm and cold campaigns.

## Linux RSS

The C prototype uses `readdir`, `getsid`, `openat`, and `read` to take the same
full-session `/proc/PID/stat` RSS sum as the Python collector. It kept the
preliminary session check and rechecked session membership and zombie state
from stat. With Python 3.12.3 and GCC 13.3.0 on Linux 6.8, CPU 15 performed 64
alternated comparisons on a stable four-process session, with 979 PIDs visible
from the observer. Both tools returned the same four PIDs and 56,201,216 B in
every pair. The native median acquisition time was 0.991 ms versus 1.274 ms
for Python. The observed maxima were 2.133 and 3.147 ms. This demonstrates
reduced scan overhead on these live processes. Longer scan delays on real
workloads remain unqualified. [Raw probe](native-rss-probe.json.gz)

The sealed R50 run dedicated a physical core to RSS collection. Its longest
reference scan took 43.020 ms wall and 6.396 ms thread CPU. Five other
reference scans each consumed more than 20 ms of thread CPU, up to 22.417 ms.
These results include both off-CPU delays and scans that spent substantial
time on CPU. A shorter idle scan alone cannot establish long-job cadence.
[R50 cadence](../integrated-r50-12f2151/rss-cadence.json)

A task-owned transient systemd user scope also supported direct
`clone3(CLONE_INTO_CGROUP)` into a precreated leaf, with the harness outside
that leaf. Reading `cgroup.procs` and summing the same per-process RSS fields
agreed with the full session in 24 comparisons at 19,292,160 B. Median
acquisition was 0.061 ms compared with 1.140 ms for the full scan. The test
then moved its own worker to an adjacent leaf using the same unprivileged UID.
The worker retained its kernel session, while the original leaf omitted its
7,233,536 B. Setting `cgroup.max.descendants=0` on the measurement leaf did
not prevent that move. The scoped roster requires independently enforced
containment to cover the full session. The temporary user scope stopped and
was removed. [Raw cgroup probe](cgroup-roster-probe.json.gz) ·
[Linux cgroup delegation rules](https://docs.kernel.org/admin-guide/cgroup-v2.html#delegation-containment) ·
[Direct cgroup spawn](https://man7.org/linux/man-pages/man2/clone.2.html) ·
[Transient user scopes](https://www.freedesktop.org/software/systemd/man/latest/systemd-run.html)

## NVIDIA process-list acquisition

Replaying the retained physical archives recovered all original completion
gaps and counts. The 17 Vulkan candidate E9 jobs had 17 interior gaps over
20 ms, with a 33.885 ms maximum. The 17 CUDA candidate E9 jobs each had a
terminal gap over 20 ms with an NVML collection in flight at observed exit.
The sealed T50 diagnostic pair had 13 candidate and two reference interior
misses, including one candidate acquisition lasting 62.189 ms. Each of these
archives observed at most one NVIDIA-attributed session PID per job.
[Raw replay reduction](gpu-cadence-replay.json.gz)

| Runtime | Maximum RSS gap | Maximum GPU gap | Compute call wall / CPU at the miss | Graphics call wall at the miss |
|---|---:|---:|---:|---:|
| wgpu Vulkan | 8.948 ms | 34.874 ms, interior | 26.972 / 0.781 ms | 0.097 ms |
| CUDA | 9.156 ms | 35.329 ms, through exit | 30.815 / 2.319 ms | 0.146 ms |

The existing collector scheduled those samples within 0.073 ms of their
nominal slots. One individual compute-process query exceeded the entire
20 ms budget in each run. Concurrent compute and graphics calls would still
have to complete that slow query. Observed GPU memory was 204,443,648 B on
Vulkan and 295,698,432 B on CUDA. Both are lower bounds and remain pending
under the original reducer.

NVIDIA’s combined running-process detail API requires Hopper or newer.
CUPTI memory activity describes CUDA allocation and release operations tied
to that runtime and cannot establish cross-backend Vulkan/OpenCL/CUDA
process-list memory. Device-wide totals include unrelated clients. Linux DRM
resident counters are optional and the installed NVIDIA interface previously
showed no registered driver `show_fdinfo` implementation for them. NVML
allocation high-water accounting remains a separate possibility for a future
controlled setup with verified support, enabled mode, and complete process
lineage. [NVML device queries](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html) ·
[CUPTI memory activity](https://docs.nvidia.com/cupti/api/structCUpti__ActivityMemory4.html) ·
[DRM client statistics](https://docs.kernel.org/gpu/drm-usage-stats.html) ·
[Previous accounting capability receipt](nvml-accounting.md)

## Replay commands

From the repository root, use a fresh task scratch directory. The cgroup
probe requires Linux cgroup v2 and a delegated systemd user scope. The NVIDIA
probe requires the two verified `12f2151` Linux binaries, a verified E9
staging root, the locked qualification Python environment and exclusive GPU
use. Add `--prepare-only` to verify inputs and binary identities without
opening NVML. The probe removes its product outputs and caches on exit.

```sh
samples=qualification/p10-performance/evidence/resource-sampling
retained=qualification/p10-performance/evidence
scratch=$(mktemp -d)
cc -std=c17 -Wall -Wextra -Werror -O2 -fPIC -shared -x c \
  "$samples/native-rss-probe.c.txt" -o "$scratch/native-rss-probe.so"
taskset --cpu-list 15 python3 "$samples/native-rss-probe.py.txt" "$scratch/native-rss-probe.so"
cc -std=c17 -Wall -Wextra -Werror -O2 -fPIC -shared -x c \
  "$samples/cgroup-roster-probe.c.txt" -o "$scratch/cgroup-roster-probe.so"
systemd-run --user --scope --quiet -p Delegate=yes --unit codex-pancake-cadence-roster-replay \
  python3 "$samples/cgroup-roster-probe.py.txt" "$scratch/cgroup-roster-probe.so"
python3 "$samples/r50-process-membership.py.txt" "$retained/integrated-r50-12f2151/raw-paired-runs.tar.gz"
python3 "$samples/gpu-cadence-replay.py.txt" \
  "$retained/integrated-e9-12f2151/raw-paired-runs.tar.gz" \
  "$retained/integrated-e9-cuda-12f2151/raw-paired-cuda.tar.gz" \
  "$retained/t50-12f2151-preflight/pair-001/raw-pair.tar.gz"
"$qualification_python" "$samples/nvml-process-call-probe.py.txt" \
  --input-root "$verified_e9" --vulkan-cli "$sealed_linux_cli" \
  --cuda-cli "$sealed_native_cuda_cli" --receipt "$scratch/nvml-probe.json.gz"
```

The 34 qualification unit tests, script lint and format checks, and byte-for-byte
replays of both archive reductions passed. The [receipt digests](collector-cadence-followup.sha256)
cover the five retained compressed results.
