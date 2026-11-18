# Linux memory collector cadence

## Isolate the slow counter

The retained smoke used the same immutable locked Linux CLI
`d0554ce43411b39da66bd04ea5de82b2cc74384e8bb576b5ec74408aa5c6da12` and all nine
original hash-verified E9 PNGs. Each launch used the actual frozen profile and
fresh application caches. All six launches completed with genuine 337 × 510
RGB16 exports. Each device’s three decoded pixel hashes were identical. The
Vulkan runs verified the physical RTX 3090 at PCI `0000:01:00.0` through the
application’s completion event. These short tests of collector behavior ran
amid ordinary developer activity, with brief exclusive use of the GPU.

| Resource acquisition | CPU worst gap (ms) | Vulkan CPU/GPU worst gap (ms) |
| --- | ---: | ---: |
| Prior whole-host procfs scan | 63.800 | 36.565 |
| Session-filtered sequential prototype, run 1 | 12.413 | 32.132 |
| Session-filtered sequential prototype, run 2 | 13.583 | 38.591 |

On Vulkan, the two longer sequential samples spent 22.094 and 30.801 ms
between finishing their RSS observations and obtaining the NVML results near
process exit. The affected RSS reads took about two milliseconds, yet their
following intervals missed the deadline because the same sampling thread was
waiting for the driver. The CPU-only prototype’s worst capture spent 6.500 ms
on one RSS observation and its sampler used 0.397–0.409 CPU seconds across
1.387–1.511 seconds of child lifetime. The two short runs establish acquisition
cost, cadence, and output consistency under the observed conditions.

The final collector separates RSS and driver sampling, prearms both threads
before launch, and checks actual completion gaps for each metric against the
frozen limit. A late or unavailable NVIDIA reading can still leave GPU peak
memory pending. The CUDA/graphics attribution comes from NVIDIA’s per-device
process lists and live kernel session membership, so concurrent unrelated GPU
clients are excluded. The Linux `memory.peak` counter includes charged file
cache and kernel memory. `wait4` RSS and optional NVML accounting describe
individual processes. The host's NVML accounting mode was disabled. Neither
provides the frozen concurrent process-tree aggregates.
[Linux memory controller](https://docs.kernel.org/admin-guide/cgroup-v2.html) ·
[NVML accounting](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlAccountingStats.html) ·
[NVML process queries](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html)
