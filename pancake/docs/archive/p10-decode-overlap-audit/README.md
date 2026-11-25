# One-frame decode overlap audit

At `287828a`, keep decode ordered within the existing job. The archived
T50 trace shows a material upper bound for full overlap and little benefit
inside the already drained fusion stage. A lookahead would change when
original files are read, retain concurrent decoder allocations, and extend
the work that must drain before a truthful pause. The current inputs and
lifecycle leave those changes unqualified. The independent current-frame
TIFF and upload investigations can reduce those stages without expanding
the set of live original frames.

## What the measured intervals allow

The audited source includes bounded TIFF streaming and resident identity
RGB. The trace below comes from the earlier same-binary resident diagnostic
at `3f6cd9b`, before bounded TIFF streaming. It recorded 57.064 s parent
wall, 30.211 s decode, 23.529 s registration, and 1.134 s accumulation.
Registration included 20.433 s in the full RGB upload call. The same
resident allocation had already removed the separate fusion upload. The
call spans include host copies and queue waits where applicable.
[Resident T50 receipt](../p10-t50-resident-input/README.md).

For each adjacent pair, a single admitted next-frame decode can hide at
most the smaller of its recorded decode time and the preceding frame's
eligible owner work. Summing across 49 pairs leaves the first decode,
last compute, setup, reconstruction, encoding and export in place. The
model grants instantaneous launch, no CPU, memory or I/O contention, and
unchanged stage times. A tracked script verifies the archived trace hash
and recomputes the bound from all 50 measured frames. [Model](model.json).

| Owner work overlapped | Maximum savings on this trace | Best modeled wall |
| --- | ---: | ---: |
| All registration and fusion | 24.176 s | 32.888 s |
| One proxy upload, submit and readback stage | 22.747 s | 34.316 s |
| One already-admitted synchronized fusion stage | 1.113 s | 55.951 s |

R50's supported camera decoder already uses 14 nested RAW workers with
the frozen 15-CPU affinity. The independent CPU CubeCL work shares that
mask. On the same two verified Canon frames, the latest per-frame decoder
improvements reduced total decode from 1.853 s to 0.782 s for the pair.
The decoder alone reached median 639,534 KiB RSS in that arm. Each full
working RGB raster is 207,101,952 bytes. The old full R50 campaign measured
94.403 s warm against 40.598 s reference. The two-file diagnostic supplies
no measured full-stack overlap speedup and no guarantee that feeding the
RAW pool beside CPU registration and fusion leaves throughput or RSS intact.
[RAW worker measurements](../p10-r50-raw-audit/host-aware-workers/README.md)
· [Frozen R50 campaign](../../../qualification/p10-performance/evidence/integrated-r50-12f2151/README.md).

## Invariants a further experiment would have to prove

- Admit at most one next-frame operation or completed result against the
  current generation. Account for the same owner holding the current and
  next RGB allocations, next decoder scratch and encoded data, existing
  registration staging and fusion resources. Reserve the host allowance
  before any next-frame allocation. Check the next frame's actual header
  and decoder mode from the opened source. The first canvas cannot bound
  a larger or differently encoded input. The ready T50 frame alone adds
  288 MB of f32 storage. A permitted 50 MP frame can hold 600 MB before
  decode scratch. A tight budget, unfamiliar codec or geometry, alternate
  anchor and overlapping slabs keep the original one-frame execution.
  [Job plan](../../../src/job/stream.rs) · [Decode limits](../../../src/image_io/mod.rs).
- Keep registration, `ResidentRgb`, CubeCL, progress and publication on
  the existing job owner. `ResidentRgb` intentionally cannot move between
  threads. A diagnostic using one scoped join inside an existing bounded
  stage would need only one owned result slot. The decoder must never call
  the shared blocking checkpoint. Join and release outstanding work on
  every success, source failure, compute failure, and cancellation path.
  Preserve the current accumulator's device drain. [Resident input](../../../src/registration/mod.rs)
  · [Accumulator drain](../../../src/fusion/advanced/mod.rs).
- Preserve stage and generation truth. Today each checkpoint can publish
  Paused or acknowledge cancellation because its preceding owned batch
  has drained. With concurrent decode, acknowledge only after the work
  required by that endpoint has really withdrawn or drained. A settled
  Idle worker has released both slots and any decoder thread. The 50
  archived whole decodes took 582–662 ms. An unconditional join of a full
  next decode can therefore cover more than the frozen 250 ms deadline.
  This is a boundary risk, separate from a measured event-to-acknowledgement
  result. Host codec cancellation needs an observed safe granularity before
  relying on a sub-250 ms drain. [Control and checkpoints](../../../src/job/lifecycle.rs)
  · [Frozen lifecycle endpoint](../performance-targets.json).
- Preserve input order and the filesystem read contract. The request owns
  paths, and a progress callback or a paused user's edits can replace the
  next original before its ordered Decode checkpoint. Earlier file-open
  or decoder errors can also be repaired before that turn. Keep any
  speculation hidden until the ordered checkpoint. Preserve current-frame
  errors, honor cancellation at the same next checkpoint, and reopen or
  discard speculation if the original changed or could not be authenticated.
  Size and mtime alone cannot prove identical encoded bytes. The frozen
  benchmark's authenticated files supply such a premise for measurement.
  Ordinary path-based jobs need an equivalent verified premise or retain
  their original decode timing. [Job entry and publication](../../../src/job/mod.rs)
  · [Actual source open](../../../src/image_io/mod.rs).

## Verification before considering production

Start with a real locked CLI job using already authenticated RGB16 TIFFs.
A test-first trace assertion requires a genuine next Decode interval
inside the previous frame's admitted stage, with at most one outstanding
source at every event. The current serial job supplies the red run. Record
monotonic stage events in bounded diagnostic memory and write them after
execution. Keep steady-state acceptance timing free of the tracing hook.

Retain the CPU job checks and add a three-source mutation case using real
TIFFs. During the first frame's progress callback, replace the next file
with damaged bytes, or repair a previously damaged file. Require the same
visible source error or decoded output, ordered progress, and untouched
sentinel export on failure. Cancel while the actual decoder stage is active
and verify that the correct generation settles, can restart, and publishes
neither a file nor a completion record. Pause and resume a real job and
verify no codec work survives pause acknowledgement. Exercise a tight
budget and an ineligible format with the original synchronous route.
[Existing job and lifecycle tests](../../../tests/cpu_job.rs).

Only a positive exact-source diagnostic with bit-identical TIFFs,
transforms, progress and backend events justifies a further matched run.
Capture physical CUDA/Vulkan identity separately as applicable, native
pause/cancel and viewer endpoints, process RSS and attributable GPU memory
at the frozen cadence. A full passing campaign still requires the actual
frozen schedule and applicable regressions. No product or harness source
changed for this audit. Reproduce the analysis from the repository root
with `python3 pancake/docs/p10-decode-overlap-audit/model.py.txt .`.
