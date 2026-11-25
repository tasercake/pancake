# P0.01 performance targets and measurement contract

Frozen initial engineering targets: 2026-11-17. Machine-readable authority:
[performance-targets.json](performance-targets.json), version `p0-initial-1`.

These are concrete initial engineering targets set during the authorized P0
execution. They are not figures previously supplied by the user, measured
achievements, or promises that the selected stack already meets them. Changes
must reopen P0.01 before evaluating a replacement target; a failing result does
not justify moving the threshold after the fact.

## Fixed targets

| Metric | Target | Qualification rule |
|---|---|---|
| Cold completed-job median | **No regression** | Candidate/reference ratio at most 1.00, including process startup, fresh application/compiler caches, full input processing and export. Normal OS disk caches are not claimed cold. |
| Peak memory | At most **50%** of reference | Compare equivalent process high-water marks. On discrete GPUs also enforce half attributable peak GPU memory where measurable; moving allocations to GPU does not establish a memory win. |
| Viewer input-to-present | **P95 at most 33 ms** | At least 200 native events covering selection, pan, zoom and comparison, both idle and under real compute load. Correct presented state is the endpoint, not enqueue time. |
| Pause acknowledgement | **At most 250 ms** | At least 30 requests; worker reaches a safe boundary, stops new processing submissions and exposes actual Paused state. UI request receipt alone is insufficient. |
| Cancel acknowledgement | **At most 250 ms** | At least 30 requests; worker accepts cancellation and stops new decode/compute submissions. Record final reusable Idle/resource-drain latency separately, without calling pending cleanup Complete. |

For pause/cancel, every sampled acknowledgement must meet the deadline; report
P50/P95/max and settled-state latency as well. Exercise the unchanged real
64-frame lifecycle case and selected real workloads. No sleep, processing
barrier, fake completion, or business-operation hook may extend the interaction
window or substitute for actual worker state.

Apple unified memory is not two independent physical pools. Use peak task
physical footprint as primary physical-memory metric; record RSS and logical
GPU allocations separately, never sum them with footprint. On discrete systems,
record process-tree RSS and attributable GPU peak separately. If a required
metric cannot be measured reliably, that profile remains pending rather than
passing by omission.

## Workload slots

These are recipes, not selected benchmark identities. Input hashes, exposure
metadata, exact dimensions, rights, conversion commands and selected frame order
must be frozen before a qualification baseline. Never repeat a frame or upscale
small images to satisfy a missing real workload. A later explicit user decision
authorizes deterministic generated T50 scale inputs and cleanup; it does not
authorize synthetic substitutes for R50/D150/E9.

| Slot | Required input | Methods and geometry |
|---|---|---|
| T50 | 50 distinct deterministic generated focal-scale frames, native 6000x4000 genuine RGB16 ZIP/Deflate TIFF. Freeze generator/seed/ordered hashes, label GENERATED and clean up after qualification. No 8-bit promotion or resize; regenerate exact bytes before later comparisons. | Laplacian, Hybrid, weighted average, depth map; similarity alignment; first reference. |
| R50 | 50 distinct real native camera RAW focus frames; native dimensions and metadata recorded. No embedded-thumbnail substitute. | Same four focus methods and alignment. |
| D150 | 150 distinct real deep-focus frames at fixed native resolution, native supported camera files or losslessly derived TIFF. Record true source precision. | Same four methods; breathing alignment; slabs of 40 with overlap 4. |
| E9 | Nine distinct real fixed-focus exposure-bracket frames; original supported files or provenance-pinned float TIFF derivatives. | Exposure fusion only, Euclidean alignment; current 4+4+1 batch semantics. This is not a focus-stack speed test. |

Common explicit settings: kernel size 6, pyramid levels 8, alignment scale 6,
first reference, auto parameters off, auto-crop on, TIFF16 output. Record actual
relevant options per method; do not benchmark reduced quality against a more
expensive reference profile. Native-source precision is never upgraded merely
by converting an 8-bit JPEG into a 16-bit container.

## Pinned comparison and samples

Use the same physical host/device, OS, driver, power mode, input bytes, settings,
and output depth. Explicit CPU compares with explicit CPU; explicit GPU compares
with explicit GPU on the same physical GPU. CubeCL CUDA versus reference OpenCL
on that physical device is valid when both APIs are disclosed. Cross-device or
CPU-versus-GPU comparisons cannot qualify the targets. Evaluate Auto separately.

Warm protocol: two complete warm-up jobs, then **10 completed measured jobs**.
Each CLI job starts a fresh process with warmed persistent compiler/shader/Numba
caches and ordinary OS file caches. Decoded images/results do not carry between
jobs. This includes launch overhead and full decode/alignment/fusion/export.
Persistent-GUI-engine measurements must be reported separately. Retain all
samples, min/median/nearest-rank P95/max; at n=10 job P95 is descriptive, not tail
certification. Viewer P95 uses at least 200 measured native events.

Cold protocol: **five completed fresh-process, fresh-application-cache jobs** per
profile. Use unique case-owned compiler/cache directories for each target;
never delete global or user caches. Include JIT/initialization and record setup
breakdown separately where available. This does not promise physically cold
filesystem or driver caches.

Time with a monotonic parent clock from immediately before process spawn until
successful exit after export is closed/published. Decode and validate the output
independently after the timer. GPU submission duration is not completed-job
duration. Primary runs have coverage/profiling disabled; use separately recorded
or identically instrumented runs for truthful backend validation.

Serialize benchmarks per device and alternate target order across pairs. Record
thermal/power state; no other audit/GPU load may run concurrently. Retain invalid
samples and the reason for any replacement. Numerical failure is not measurement
noise and may not be retried away.

Capture process and device high-water marks through initialization, decode,
compute, export and cleanup. Use high-water counters where possible; otherwise
sample at 20 ms or less and disclose missed-short-peak limits. Record OS units,
counter/tool versions and worker accounting.

## Quality, provenance and current progress

Every measurement needs ordered input SHA-256/size/shape/metadata, source/rights,
conversion commands, program/runtime locks and options, hardware/driver/OS,
counter versions, all raw samples, output hashes and validation results.

**Targets are fixed; full corpus/baseline qualification remains pending.** Initial
read-only workspace discovery found no RAW/TIFF corpus under `datasets/`, only
Bokehforge state/cache/report directories. Versioned source policy lists D02
Sood CR3 as a possible real source, but naming a source is not selecting or
qualifying its files. D01/LSFD remains excluded and the existing 2 GiB download
budget remains in force. The subsequent [bounded acquisition/baseline audit](p0-performance-baseline.md)
pinned 150 real 2304x1728 dorsal focus JPEGs and nine real 512x339 Balloons
exposure PNGs. It completed one reference CPU job per selected method: D150
Hybrid 30.585 seconds with peak RSS 2,897,117,184 bytes; E9 exposure 0.877 seconds
with peak RSS 237,469,696 bytes. These are diagnostic observations, not medians,
unified-memory footprint measurements or target passes.

The diagnostic jobs use kernel 3/levels 4 instead of the frozen 6/8 profile;
auto-crop is absent, and E9 has no alignment instead of Euclidean alignment.
They cannot become qualified baselines without the matched profile and full
sampling/validation protocol. R50 is now selected and fully decoded; T50 uses
the subsequently authorized deterministic generated recipe. No candidate full-job
timing or native input-latency target result exists. Under the adopted smaller
P0, complete matched comparisons belong to the first full pipeline/P10, not the
P0 architecture exit. See the [work log](p0-work-log.md) for actual input and
cleanup receipts, and the plan for current scope.
