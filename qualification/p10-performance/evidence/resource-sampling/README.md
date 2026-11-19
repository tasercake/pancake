# Linux memory collector cadence

The [collector follow-up](collector-cadence-followup.md) retains the native
RSS, delegated cgroup roster, and physical NVIDIA per-call tests.
The [private procfs and lineage check](rss-pid-namespace.md) records exact RSS
membership across cgroup movement and the launch-equivalence blocker.
The [NVML overlap audit](nvml-overlap.md) records when two complete workers can
cover a delayed acquisition and the remaining device-level evidence needed.
The [E9 overlap diagnostic runner](nvml-overlap-runner.md) documents its
device-free preflight and bounded physical invocation.

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

## Independent collectors on actual completed jobs

From 12:48:25–12:48:34 SGT, the same locked CLI ran three CPU and three
physical Vulkan E9 jobs on the Ryzen 9 7950X3D and RTX 3090, driver 580.178.04.
One separate short GPU diagnostic had already begun at the handoff and may
have overlapped the initial CPU work. These six launches qualify the sampling
behavior only. All exports decoded as genuine 337 × 510 RGB16 and each
backend repeated its exact previous decoded output hash.

| Collector and runtime | Max RSS gap (ms) | Max GPU gap (ms) | RSS measurement | GPU measurement |
| --- | ---: | ---: | --- | --- |
| Prior CPU, shared interval | 36.681 | — | Pending | — |
| Independent CPU, first | 11.916 | — | Captured | — |
| Independent CPU, second | 13.563 | — | Captured | — |
| Prior Vulkan, shared interval | 36.872 | 36.872 | Pending | Pending |
| Independent Vulkan, first | 8.432 | 34.367 | Captured | Pending |
| Independent Vulkan, second | 9.059 | 37.242 | Captured | Pending |

Both final Vulkan runs completed on the same physical PCI device. Their
largest NVML acquisition times were 26.739 and 30.153 ms near shutdown, with
RSS observations continuing independently. Both runs observed an attributable
GPU-memory lower bound of 204,443,648 bytes. The frozen GPU memory metric
remains pending. Its required acquisition cadence needs a dependable
driver-side counter spanning teardown or a matching aggregate high-water
counter for both applications. Host RSS cadence passed all four updated
smokes. The sampler consumed 0.269–0.330 CPU seconds for CPU jobs and
0.195–0.358 summed CPU seconds across both threads for Vulkan jobs, recorded
alongside their actual wall intervals. Temporary exports and caches were
removed and all nine original inputs rehashed unchanged. The retained
sampler files, launch commands, input acquisition receipt, and both counters’
raw timestamps identify the exact runs independently of the branch name.

The installed NVIDIA 580.178.04 kernel interfaces under
`/usr/src/nvidia-580.178.04/{nvidia-drm,nvidia,nvidia-uvm}` contain no registered
`show_fdinfo` callback or DRM resident-memory export. Linux makes those DRM
counters optional. NVIDIA documents the combined NVML process-detail API for
Hopper or newer devices, outside this RTX 3090 validation lane.
[Linux DRM accounting](https://docs.kernel.org/gpu/drm-usage-stats.html) ·
[NVIDIA process-detail API](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html)
