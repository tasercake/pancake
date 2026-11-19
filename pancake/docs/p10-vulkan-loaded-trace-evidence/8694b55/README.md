# Direct loaded Vulkan trace at 8694b55

The exact `8694b55` packaged GUI completed one physical NVIDIA RTX 3090
Vulkan Hybrid stack on 12 exact source TIFFs and drove 12 private
source-only viewing transitions. Six were idle and six had a real
immediately preceding `Running · Fuse` observation. The GPU completion
and exact source bytes were independently verified. All 36 captured
RGB patches matched the independent projections within one channel
level. The run took 21 seconds from 20:15:23 through 20:15:44 SGT on
2026-11-18. [Exact verification](summary.json), [pixel patches](presented-pngs.tar.gz)
and [input verification](authenticated-preflight.json).

The same actual GUI PID 463285 produced 422 Vulkan submissions, 200
image acquisitions and 200 presents. Its exit was observed independently
through its pidfd after the native close and without forced cleanup.
Nsight's separate report flush succeeded and the launcher exited 0.
The largest observed UTC-to-monotonic envelope was 1.412 µs.
[Matched API intervals](timeline.json), [clock verification](utc-normalization-preflight.json)
and [native exit](gui-exit.json).

The earlier loaded P95 tail did not recur in this instrumented,
12-event source-only sample. The longest observed transition was
23.8471 ms. This trace demonstrates available Vulkan telemetry
and supplies no attribution of the prior tail or frozen viewer
performance result. Ordinary compilation ran on CPUs 6 and 7,
and a separate hygiene check was in progress. The full target
requires a fresh 400-event exact packaged capture with an
uninstrumented observer and all original T50 inputs and witnesses.

The Nsight report, SQLite, full original capture and process
census remain owner-only outside Git under
`artifacts/p10-viewer-queue-trace-preflight/direct-loaded-8694`.
The Git receipt contains only independently selected measurements,
scope hardware, real application patches and raw artifact hashes.
Authenticated cleanup rehashed and removed all 12 generated TIFFs,
releasing 622,541,539 bytes. [Cleanup](cleanup.json) and [evidence checksums](SHA256SUMS).
