# E9 overlap diagnostic runner

The runner executes at most four launches, in Vulkan serial, Vulkan overlap,
CUDA serial, CUDA overlap order. Each arm uses the exact `12f2151` release
binary and settings recorded by the prior physical E9 diagnostics. It uses
fresh task-owned caches and matching archived CPU affinity. The receipt has
no qualification credit or frozen performance verdict. Absence of a long
compute call in either arm makes the comparison inconclusive.

For a diagnostic bypass, the second worker must start its compute call
during the first worker's long compute call and finish its whole pair before
the first compute call returns and before job exit. Workers use independent
buffers and preserve both raw lists, their actual call windows, PID/session
and start ticks observed for each record, and the paired result. The merger
matches the primary per-PID aggregate, using the larger value for a PID seen
in both API lists. PID reuse, unknown attribution, unavailable bytes and
query failures are retained as unusable records. Observed completion gaps,
the separate conservative window bound, long or post-exit rows, and actual
bypass witnesses are all retained. NVML still provides no atomic shared
snapshot time for the two lists.

The runner shares the repository performance lock. A diagnostic operator
must also have the device lease before executing it. The monotonic interval
covers spawning the real CLI through observed wait completion. A watchdog
ends a synthetic or real job that exceeds its 12-second budget. The observer
prearms each lane and starts at most two pairs at once, using the same 8 ms
absolute slots and skipping missed or busy slots. It drains the active lanes
after exit, then closes NVML before releasing the lock. If a native call
will not drain, the runner records which slots were active and exits without
closing NVML or unwinding the lock. The kernel reclaims it when the process
actually terminates. Verify termination before transferring the device
lease. The normal path deletes owned caches and output TIFFs after retaining
the independently checked output shape, decoded RGB16 pixels and hashes.

## Preflight and bounded run

The preflight reads local files only. Omitting `--run-diagnostic` never opens
NVML, queries a device, starts a measured job, or acquires the performance
lock. The earlier bundle contains the acquisition reproducer that verifies
the Waterloo archive and extracts only the nine selected originals. Keep
the creator's copyright notice with the staged copies. [Archived E9 source and reference provenance](../integrated-e9-12f2151/README.md) ·
[University of Waterloo source and reuse terms](https://ivc.uwaterloo.ca/database/MEF/MEF-Database.html)

During the reserved device window, add `--run-diagnostic "$fresh_receipt"`
to that same command. Keep the whole terminal receipt even when either arm
is inconclusive or invalid. Remove staged images after the final read.

The initial preflight verified both immutable binaries, both historical
paired archive digests, frozen target hash and all nine source-file hashes.
Fourteen new device-free tests cover deduplication, identity errors, distinct
result buffers, absolute slots, a blocked lane, exit and gap bounds, a real
synthetic child reaped by the watchdog, process-exit lease cleanup,
independent TIFF decoding and an exact physical device check. All
48 qualification tests passed.

## Physical diagnostic result

All four authorized launches completed at 20:11:37–20:11:40 SGT on
November 18, 2026, on the RTX 3090 at `0000:01:00.0` with NVIDIA driver
580.178.04. The prelaunch census recorded 0% GPU utilization and 575 MiB of
ordinary desktop residency. Each runtime's serial and overlapping runs
reproduced the earlier encoded TIFF digest and independently decoded
337 × 510 RGB16 pixel digest. The actual GPU completion events confirmed
Vulkan and CUDA as selected, with no CPU fallback. Both binaries and all
nine originals remained byte-identical at the end. No arm was retried.

| Runtime and arm | Longest compute call | Maximum bounded-pair completion gap | Conservative window gap | Complete bypasses |
|---|---:|---:|---:|---:|
| Vulkan, serial | 24.350 ms | 39.361 ms | 40.454 ms | — |
| Vulkan, overlap | 40.815 ms | 55.320 ms | 57.124 ms | 0 |
| CUDA, serial | 19.800 ms | 27.415 ms | 27.737 ms | — |
| CUDA, overlap | 24.907 ms | 31.639 ms | 32.822 ms | 0 |

The Vulkan control reproduced a compute call over 20 ms. In the Vulkan
overlap arm, two compute calls began at 200.105 and 208.115 ms and returned
at 240.921 and 241.092 ms. Near exit, two more began at 664.093 and 672.100
ms and returned at 688.366 and 688.384 ms. Both times the second whole pair
missed the bypass condition. The close returns are consistent with a common
stall. The user-space trace cannot identify an internal driver lock.

The CUDA serial arm had a gap above 20 ms but never reproduced a compute
call above 20 ms. Its comparison is inconclusive under the planned
reproduction rule. The CUDA overlap arm observed compute calls of 24.907
and 17.047 ms, with both whole pairs completing after the observed job exit.
They supply no in-interval coverage or peak credit.
