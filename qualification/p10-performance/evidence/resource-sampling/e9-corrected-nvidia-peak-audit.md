# NVIDIA peak memory evidence after corrected E9

This read-only audit used qualification revision
`71d130d08270a3bb30346d7de7e4f4b23de67ae5` and the retained corrected E9 run
on the RTX 3090 at PCI `0000:01:00.0`, driver `580.178.04`. It read no live
device state and changed no tooling, settings, or benchmark data.

## Frozen rule and the observed gap

The discrete-GPU target applies the 0.50 ratio separately to peak
process-tree RSS and attributable GPU allocated/resident bytes. The memory
protocol accepts a full-job high-water counter. If none is available, it
requires sampled counters at intervals of at most 20 ms over startup through
exit. A required unavailable peak remains pending.
[Frozen target and protocol](../../../../pancake/docs/performance-targets.json)

The sampled collector acquires both NVML compute and graphics v3 tables on
the selected physical UUID. It filters to the job session and takes the
larger reported total for duplicates. The reducer uses real acquisition
completion timestamps with launch and exit boundaries. It excludes
acquisitions finishing after exit. The archive also records acquisition
start times. Re-reducing only the 34 primary typed GPU streams reproduced
every reported sample count, observed peak and maximum gap.

| Physical runtime | Gap over 20 ms | Location | Largest gap | Observed attributed bytes |
| --- | ---: | --- | ---: | ---: |
| Pancake CUDA | 17/17 | Exit boundary in all 17 | 34.084586 ms | 295,698,432 |

In every CUDA job, a sample started before observed exit and finished after
it. The largest CUDA gap came from cold run five, whose longest acquisition
was 28.505612 ms. The largest OpenCL gap was cold run two, whose longest
acquisition was 31.173220 ms. Exactly one distinct attributed PID appeared
in each job's accepted rows. Discrete observation cannot establish that
other GPU owners never existed between samples.
[Corrected E9 results and raw archive](../integrated-e9-corrected-cuda-c1e2cac/README.md)

The two v3 calls return dynamic process records without a source sampling
timestamp. A long acquisition interval establishes a measurement gap under
the current reducer. It leaves the precise driver snapshot time unknown.
Backdating a result to call entry or using a post-exit result would not add
an authenticated in-lifetime sample. The 295,698,432-byte CUDA observation
means the true OpenCL peak would have to be at least 591,396,864 bytes to
allow the 0.50 ratio. The observed OpenCL value is a lower bound, so this
arithmetic supplies neither a measured ratio nor a strict failure.

## Available interfaces and an unblock condition

Earlier physical probes isolated the compute query itself to 26.972 ms
on Vulkan and 30.815 ms on CUDA, with the graphics calls below 0.15 ms.
Two independent full acquisitions showed zero complete bypasses on the
same device. Changing the observer language, moving graphics to another
worker, or increasing the nominal polling rate leaves that device evidence
unresolved. The installed `libnvidia-ml.so.580.178.04` exports
`nvmlDeviceGetRunningProcessDetailList`. NVIDIA's installed CUDA 12.5
`nvml.h` describes it as Hopper or newer, outside the RTX 3090 lane. Its
process-mode argument also requires coverage verification for a future
supported device.
[Physical per-call probe](collector-cadence-followup.md) ·
[Physical overlap probe](nvml-overlap-runner.md) ·
[NVIDIA device query contract](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html)

The installed NVIDIA kernel sources have no `show_fdinfo` accounting hook
for GPU memory. NVML utilization history measures activity, and device
framebuffer totals include other device clients. CUDA allocator/CUPTI
records and Vulkan memory-budget or memory-report interfaces have different
scope and cannot provide a symmetric hardware process peak for the pinned
OpenCL reference.
[Kernel and source inventory](README.md) ·
[NVIDIA device query contract](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html)

A controlled target-compatible NVIDIA host with accounting already enabled
could validate terminal `nvmlAccountingStats_t.maxMemoryUsage` for each
actual candidate backend and pinned OpenCL on the same UUID. It reports
one PID's lifetime allocated-byte high water. The current card's read-only
receipt showed mode disabled, a 4,000-record ring and no record for either
previously completed E9 GPU PID. Before using it, require fresh nonmissing
records after worker drain, a complete fork/exec/exit ledger tied to PID
start identity, and verified coverage of every GPU-owning process. One owner
or provably nonoverlapping GPU owners permits a job maximum. Concurrent GPU
owners require a common-time aggregate peak source. Record counter scope as
allocated bytes on both sides. A validated timestamped aggregate GPU series
covering the complete job at the frozen cadence is the other eligible route.
Neither route is established for the installed setup.
[Accounting capability receipt](nvml-accounting.md) ·
[NVIDIA accounting field definition](https://docs.nvidia.com/deploy/nvml-api/latest/api/structnvmlAccountingStats__t.html)
