# NVIDIA process-list acquisition overlap

This audit used qualification source `b820da05`. It replayed the existing
physical RTX 3090 data and ran two CPU threads against synthetic functions.
It issued no GPU queries, launched no Pancake or reference jobs, and made no
changes to the host or driver. The 20 ms limit, sampled metric and prior
verdicts stay in force.

## Two different ways to overlap

Giving each worker an entire compute-then-graphics acquisition can cover a
stall if the other worker really completes another fresh pair on that same
device. The controlled two-thread test held the first compute call until an
explicit release. The second worker returned a complete pair before release
with independent backend calls. It could only finish after release when the
two compute calls shared a lock. The same PID appeared across both lists and
was charged once at its larger reported value. An unrelated PID was excluded.
A separate replay fed five verified fresh synthetic pairs to the existing
reducer. The maximum gap was 16 ms, including the launch and exit edges.
[Synthetic controls and reducer receipts](nvml-overlap-probe.json)

| Archived candidate | Measured gap | Compute wall / CPU | Hypothetical common stall inside the measured job |
|---|---:|---:|---:|
| Vulkan | 34.874 ms interior | 26.972 / 0.781 ms | At least 26.972 ms |
| CUDA | 35.329 ms through exit | 30.815 / 2.319 ms | At least 30.532 ms |

If every worker needs a lock for the corresponding archived driver stall,
even a completion immediately before the lock blocks would leave a gap
above 20 ms. The archive identifies the delayed call and its off-CPU time.
It cannot show whether parallel requests share that stall on this driver.
NVML documents simultaneous calls as thread-safe. Its API supplies no
independent completion or latency guarantee. [NVML thread-safety contract](https://docs.nvidia.com/deploy/nvml-api/latest/nvml-api-reference.html)

## Interval and peak witnesses

- A slower pair started at 10 ms and finished at 42 ms after another pair
  finished at 24 ms. Sorting by completion and treating the old pair as a
  fresh observation appeared to close the 58 ms interval with an 18 ms
  maximum gap. The last independently fresh result remains at 24 ms, leaving
  34 ms. Completion order alone cannot establish where the two tables were
  sampled when the driver's returned process records supply no sample time.
- The synthetic compute list returned 100 bytes for PID 11. That process
  freed them before PID 12 acquired 100 bytes, which appeared in the later
  graphics list. Combining those lists returned 200 bytes although the
  simultaneous job total never exceeded 100. Pairing arbitrary rows across
  slots increases this ambiguity already present in serial acquisition.
  Independent buffers protect against data races. They cannot provide an
  atomic read of both process lists.
- A pair that starts at 21 ms and finishes at 38 ms crosses the observed job
  exit at 33 ms. The existing reducer excluded it, retained the earlier 100
  bytes as a lower bound, and left the peak pending for a 25 ms tail gap.
  Backdating that pair to dispatch would fabricate a 13 ms gap and a 999-byte
  peak. A missing value for an attributed live process left the peak pending
  even when every completion fit the limit.

Any experimental collector should keep at most two in-flight complete pairs,
avoid queued catch-up calls, and give each worker its own mutable NVML
buffers. A pair needs a stable slot ID, the actual interval of both queries,
an immutable copy of both tables, and verified lifetime identity for each
matching session PID on the selected UUID. Release comparisons require the
same attribution and treatment of duplicate PIDs. A successful, bounded
measurement must span spawn through observed exit. Keep every raw outcome.
Use only complete, correctly attributed, temporally credible observations in
cadence and peak reduction. An overtaken valid peak cannot be dropped merely
to improve the result. A stale or ambiguously combined peak cannot qualify
the metric. No dispatch-time substitution, zero filling, result reuse, or
post-exit completion can bridge a gap. Stop scheduling at the actual exit and
drain the outstanding calls before unloading NVML. Worker timeouts cannot
establish that a blocked native query actually completed.

If a coherent observation is known only to fall within a query window,
one conservative cadence check uses a series of non-overlapping windows.
The largest possible gap between adjacent observation points is bounded by
the current window's end minus the prior window's start. The corresponding
launch and exit bounds are first end minus launch and exit minus last start.
This bound requires coherence of the data inside each window. By itself it
cannot turn two different process-list captures into a simultaneous total.

## Installed sources and remaining requirement

The on-disk NVML 580.178.04 library exports both v3 process-list calls, their
combined detail API, utilization and accounting entry points. The combined
running-process detail API requires supported Hopper or newer hardware.
Per-PID utilization samples have time and activity fields. They lack a
timestamped per-PID allocation total. `nvidia-smi pmon` reports instantaneous
memory at a minimum interval of one second. None supplies the required
timestamped graphics-plus-compute total on this RTX 3090. [NVML process queries](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlDeviceQueries.html) ·
[NVML utilization structure](https://docs.nvidia.com/deploy/nvml-api/latest/api/structnvmlProcessUtilizationSample__t.html) ·
[NVIDIA process monitor](https://docs.nvidia.com/deploy/nvidia-smi/index.html)

CUPTI 12.5 and Nsight Systems 2024.2.3 are installed. Nsight's CUDA memory
trace records allocations and frees for profiled CUDA processes. It does
not provide the same device-level process accounting for the pinned
OpenCL and Vulkan jobs. Device-wide totals cannot exclude desktop or other
GPU users. The installed driver source provides no observed DRM resident
`fdinfo` counters, which drivers implement optionally. [Nsight CUDA memory](https://docs.nvidia.com/nsight-systems/UserGuide/index.html#cuda-gpu-memory-allocation-graph) ·
[DRM process usage statistics](https://docs.kernel.org/gpu/drm-usage-stats.html)

The last physical read-only accounting receipt recorded disabled mode and
unavailable post-exit PID records. Post-exit allocation high-water evidence
would require the same validated process accounting on this device for both
targets, plus complete PID lineage and either one GPU-owning PID per job or
an actual aggregate high-water source. Independent PID maxima cannot be
summed to recover a concurrent peak. The alternative is a coordinated
physical test showing two full snapshots can actually progress independently
through the relevant stall while preserving all interval and attribution
checks. Neither route is qualified by the synthetic results. [NVML accounting receipt](nvml-accounting.md) ·
[NVIDIA accounting interface](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlAccountingStats.html)

## Replay

```sh
proof=qualification/p10-performance/evidence/resource-sampling
python3 "$proof/nvml-overlap-probe.py.txt" "$proof/nvml-process-call-probe.json.gz"
```

The static inventory used `readlink`, `nm -D`, `dpkg-query`, and `ldconfig -p`
on installed libraries and package metadata. No runtime tool was opened.

## Proposed read-only physical overlap diagnostic

Use a separate diagnostic wrapper around the public NVML v3 calls. Initialize
NVML once, prearm the workers, and give each its own result buffer. Use the
existing absolute 8 ms slots with at most the available workers in flight.
Skip slots when busy. Each worker queries both tables in order for one slot;
there is no reuse or merge across workers or slots. Record monotonic start and
end of each actual call, returned session records and attributed lifetime
identities, slot order, the completion of each pair and the independently
observed job exit. Keep overtaken, missing, invalid and post-exit rows in the
diagnostic receipt. After the job exits, drain both workers before unloading
NVML. Preserve stdout, stderr and the exit record, then remove owned outputs
and caches.

The key witness is a second slot whose compute invocation begins after the
first enters a reproduced long compute call and whose complete pair returns
before that first call returns and before job exit. This demonstrates that
the second whole observation can progress during the measured stall. If the
serial arm does not reproduce a stall, or both active slots remain blocked
through the same interval, the attempt cannot establish that overlap covers
teardown. Shared completion delays are consistent with serialization; these
user-space timings do not attribute a particular internal driver lock.

These acquisitions are instrumented observations of serial versus concurrent
NVML calls. They carry no qualification timing, warm/cold, or memory-peak
credit. Even a completed second pair must be evaluated for coherence and
complete session attribution before it could support a subsequent frozen
measurement. This diagnostic plan has not been run.
