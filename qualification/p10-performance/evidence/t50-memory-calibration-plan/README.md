# T50 Weighted Average CPU memory calibration plan

## Exact roles and scope

| Arm | Producer revision | Source tree |
| --- | --- | --- |
| Stock | `a49ce692371558ff40f72b0af77f5249d00dcedb` | `965aa0c7dc33731d3aef94231afa3e59fc7bc330` |
| Scratch alias | `5f2623ac85682233c20abe12fc691000568ad127` | `c6b8730d3d16506022316c68ae1322a5ac90e266` |

Use the hashes and actual build receipts in [producer pins](producer-pins.json).
Both Pancake builds use the standalone default CPU/wgpu graph and same
Cargo.lock and adapter descriptor. Verify the independent exact producer
roots and binary hashes before and after the jobs. Diff
`a49ce69..5f2623a -- pancake` yields one runtime file
`pancake/src/fusion/focus.rs`, plus test and assurance changes. The prior
pair's dossier can identify the a49 default ELF. It certifies neither the
5f producer nor any package linkage. [Historical a49 diagnostic](../t50-weighted-triplet-0a9ae85-a49ce69/README.md)

The notice-bearing `11cfcf8` package stays under its own exact installer,
artifact and release-lane proofs. It is excluded from this calibration.
Later formal scheduling must use the verified exact producer chosen
for that release. Keep both native build receipts and the separate
tool source receipt. Compare build mode, features, toolchain, linker and
environment from the actual immutable manifests. Record a linkage
difference explicitly if found before interpreting a source-causality
result.

## Setup and measurement

- Obtain a new explicit read-only Stage6 loan from P3 and exclusive
  host authorization from root only after P9, P6 and other workloads
  are actually terminal. The owner confirms the existing
  [52-record seal](../t50-stage6-e81079d/shared-stage.json) is retained.
  Match the ownership marker, materialization SHA
  `1d9be3f1cd20a01083b11c10c4d17ab75a4f3b6bb7e2ebef682824e345ba649d`,
  and original encoded and independent decoded records. Verify actual
  encoded bytes/sizes before the first job and after the truth review.
  Feed the same 50 original focus frames in order. The two truth TIFFs
  are review-only. Return the readonly loan after the last original read.
- Bind the same CPU RSS tool
  locally by exact Git `df780846a4dcc40bf460bcaaba04f1538290e71c`.
  Independently verify both tool hashes in [producer pins](producer-pins.json).
  Load the one unchanged `T50.weighted_average.cpu` profile for all
  roles. The prepared diagnostic dispatch passes
  `rss_sample_interval_ms=3` to `p10._run_sample` for every arm.
  The corresponding CPU runner CLI flag is `--rss-sample-interval-ms 3`.
  Record `sample_interval_ns = 3000000` in the triplet, each launch and
  each sample receipt. Fail setup or preserve an invalid arm if any
  accepted setting differs. The existing default remains 8 ms.
- Use one fresh, empty case-owned cache per arm, serial ref →
  stock → alias order, and ordinary OS caches. Use job affinity 0–14
  on distinct host cores, 15 Rayon/OMP/Numba worker threads, one
  OpenBLAS thread and RSS sampler CPU15. Record actual masks,
  CPU topology/thermals, CPU and I/O pressure, deadline, scoped lease
  and target-specific argv. Use a monotonic pre-spawn through blocking
  `wait4` measurement and the existing 300 s per-job bound. On this
  host the prior 65 s reference and 26 s resident samples suggest
  roughly two minutes for the new jobs before auth and pixel checks.
  No profiling tool, device query, warmup job, or retry is part of
  the triplet.

## Acceptance of observations

Each arm must complete explicit CPU work, drain its process session, emit
its role's actual stack event and publish genuine 5998×3998 RGB16. Check
the ordered 50 source names, reference CPU per-stage evidence and crop
`[1, 1, 1, 1]`; the stock reference Similarity sequential field is null.
Both Pancake arms must emit 50 ordered keyed and 50 sequential identity
affines, and their encoded and decoded export digests must match each
other. Reuse the generated sharp-truth review on all 50 depth regions.
Preserve any quality or geometry discrepancy and end at the diagnostic
scope.

The metric remains summed Linux process-tree RSS from process launch to
exit with real observations at **at most 20 ms** intervals, including the
first and last edges. Three milliseconds is the requested scheduling
period, with actual cadence computed from each finished raw observation.
Retain nominal period, attributed PID rosters, timestamps, acquisition
time, raw peaks, exact maximum gap and validity. Verify the sampler's
real affinity is `[15]`. Keep observed peaks when an arm misses, and
leave every strict RSS ratio involving that arm pending. Even valid
single-triplet ratios are descriptive. The historical 8 ms reference
had a 22.476502 ms maximum gap, and P4's shadow 3 ms replay showed
possible improvement with unresolved collector overruns. No faster
real observation was collected for this preparation.

The [offline envelope checker](calibration-envelope.py.txt) accepts the
typed report scope plus each actual launch, sample and ordered observation
list. It verifies the common period and independently recomputes each
true gap, count, observed peak and eligible peak. Its [six synthetic tests](test-calibration-envelope.py.txt)
cover mismatched periods and collector source, real edge gaps, ordered
timestamps and the exact 20 ms validity boundary. It has no application
execution entry point.

If an arm still misses, reduce the exact offending gaps by approach
lateness, roster acquisition wall time and sampler thread CPU time before
proposing a separate experiment. Preserve the original observations.
A change such as a tighter scheduled period or a fully verified
process-roster provider needs its own documented method and new quiet
authorization for all roles. Keep the strict result pending until there
is a complete measured tree observation. Do not substitute `wait4`
single-process peaks, device-wide storage, a previous reference peak,
or inferred points. Keep typed argv, events, RSS traces, output hashes,
ROI checks, provenance and cleanup receipts. Keep ambient environment
and TIFF bodies out of Git. Share custody and derived-export cleanup
follow the same owner and checksum rules as the earlier triplet.

## Prepared operator

Only after the new actual authorization, provide a fresh quiet receipt
and a new private output root. The receipt must bind true
`exclusive_host_granted`, `stage6_read_granted` and
`other_workloads_terminal` fields, profile `T50.weighted_average.cpu`,
the exact materialization hash, `order` equal to
`["reference", "stock", "candidate"]`, and `sample_interval_ns` 3000000.
Append the grant time and host census to the receipt.

The dispatcher writes the actual child terminal marker and records
same-stage and same-producer postflight. Wait for it to exit and release
the common lock before handing off quiet. The three job RSS traces use
the explicitly requested 3 ms setting. The output proof remains separate
from the prior 8 ms diagnostic and the exact 11c package receipts.
