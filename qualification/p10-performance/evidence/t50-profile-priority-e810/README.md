# T50 profile priorities for the exact e81079d package

No T50 profile currently supports a passing stock campaign. The only measured
full-scale profile is Hybrid on the physical RTX 3090. It has a known speed
shortfall and unresolved memory cadence and detail transfer. The shortest useful
next run is one paired `T50.weighted_average.cpu` diagnostic after the isolated
P9 lane releases. Use it to decide whether a full campaign is warranted.

## Frozen profiles

| Frozen profile | Exact T50 evidence | Priority |
| --- | --- | --- |
| `T50.laplacian.cpu` | No full matched run. The reference has a local tone-map step and different pyramid ownership. | Lower priority. Native region detail, tone transfer and cadence need their own evidence. |
| `T50.hybrid.cpu` | No full T50 CPU pair. The historical full R50 Hybrid CPU schedule at `12f2151` measured 94.403/40.598 s warm and 95.290/46.767 s cold for Pancake/reference, with RSS pending. | Lower priority. R50 uses distinct RAW inputs and provides no T50 duration or ratio. |
| `T50.weighted_average.gpu` | No full matched run. | First new GPU profile to screen if accurate NVIDIA attribution becomes available. CPU results cannot supply its timings, memory or quality. |
| `T50.depth_map.gpu` | No full matched run. | Second GPU screening profile. Review the per-source focus decisions independently before a full schedule. |
| `T50.laplacian.gpu` | No full matched run. | Lower priority. Native pyramid detail and GPU memory need separate evidence. |
| `T50.hybrid.gpu` | Same-card pinned pair at `8694b55` and exact packaged `fb9c399` versus experimental `e5b6260` ABBA. Diagnostic warm ratio margin, GPU attribution and equal detail are unresolved. | Defer a full schedule. A future exact new-package pair can measure any material change first. |

[R50 CPU comparability limits](../integrated-r50-12f2151/README.md) ·
[Candidate weighted and depth kernels](../../../../pancake/src/fusion/focus.rs)

## What the physical Hybrid measurements show

Exact `8694b55` Debian Vulkan and pinned OpenCL took 22.171825 s and
26.301475 s, respectively, a single-pair ratio of 0.843. The older exact
`12f2151` pair took 82.441 s and 26.737 s, with ratio 3.084.
The `fb9c399` stock jobs took 21.966022 s and 22.045896 s. The experimental
`e5b6260` jobs took 19.544322 s and 19.513047 s, an 11.0–11.5% reduction,
with identical encoded and decoded RGB16 hashes across the four outputs.
Using the earlier 26.301475 s reference purely as a scale estimate, the two
stock timings average 0.837 of it and the two experimental timings average
0.742 of it. A 0.50 ratio would require 13.150738 s at that reference duration.
It would take 40.2% less elapsed time than the measured stock average or
32.7% less than the experimental average. The cold diagnostics used fresh
case caches. None estimates a frozen warm median or qualifies a cross-run
cold ratio.

The paired outputs were 5998×3998 native RGB16, with all 50 identity maps
and 50 known-truth strips. The reference and candidate mean absolute sharp
truth differences were 979.595 and 843.328 u16 codes. The reference amplified
vertical second-difference variance to 1.1664× sharp truth across the strips.
The candidate retained 0.8667–0.8918×. The 1,792.133-code inter-output MAE
and this texture difference leave equal quality open. Both targets explicitly
completed GPU fusion and warping. The reference's optional GPU alignment helper
logged three singular-matrix warnings and its estimates used CPU alignment.
The record contains no whole-job CPU fallback.

The `8694b55` observed process-tree values were 0.988/6.701 GB, ratio 0.147.
One RSS stream exceeded the maximum observation gap. The GPU observations
were 2.518/4.316 GB, ratio 0.583, with gaps of 78.153/27.563 ms. Both GPU
observations are lower bounds. The ABBA GPU peaks were all 2.518 GB with
maximum gaps of 68–112 ms. Other NVML methods tested here have yet to show
bounded acquisition under the same rule. More CPUs alone do not fix that.
At the observed recent per-job times, 18 Hybrid pairs would spend about
14–15 minutes inside application timers. Independent TIFF decoding, input
rehashes, setup and real cold variability add to that estimate.
[Exact paired measurements and truth](../t50-hybrid-pair-8694b55/README.md) ·
[Earlier sealed pair](../t50-12f2151-preflight/pair-001/README.md) ·
[Exact dual-scan ABBA](../../../../pancake/docs/p10-dual-scan-evidence/e5b6260/README.md) ·
[GPU and RSS cadence investigation](../resource-sampling/collector-cadence-followup.md)

The exact new package is `e81079dee4078d7a1149da0f5950aab8c89f8d44`.
Its sealed Debian SHA-256 is
`8ed6e70d7552140c0257520a07ac2b9fc5f97ff82a5fb3126864df821df4507e`
and CLI SHA-256 is
`9630572be5e637c38c604f2fcd6d7a44e31e171a5ccf17dadb1f59ba345c24f0`.
A direct product source diff from `e5b6260` retains the same tiled scan and
resident Hybrid changes, with a further difference in registration spectral
coefficients. The ELF differs and e81079d has no observed T50 timing here.

## Scheduled CPU diagnostic and handoff

The Ryzen 9 7950X3D provides 16 physical cores and 32 logical CPUs. The protocol
requires matched affinity, thread flags and host settings for the two arms.
It specifies no fixed thread count. The retained runner requires `--threads`
to equal the common affinity width and matches its Rayon, Numba and OpenMP
settings. Use worker CPUs 0–14, `--threads 15`, RSS sampler CPU 15, and keep
its SMT sibling 31 and the other SMT siblings idle. The earlier official R50
schedule used this topology. The prior T50 GPU diagnostics used 0–31 and
32 threads. Their timing values stay separate from this 15-thread CPU pair.

Use one new stage6 materialization from the exact verified recipe. Preserve
all 52 encoded and dual decoded hash records, 2,775,174,405 bytes, and the
ownership marker. Stage with CPU14, nice +12, idle I/O, a singleton worker,
and the common performance lock. Require the 3,067,213,930-byte full-set
preflight plus the frozen 15 GiB reserve. The prior recipe guard requires
24 GiB free at launch, leaving space for isolated caches, reference setup
and outputs. This host observed 122,458,742,784 B free and 45,706,396 KiB
available memory in the read-only review. Retest headroom before staging.
The two previous exact-package materializations took about 226–228 seconds
at about 82 MiB peak RSS. The prior stage5 was authenticated and removed
under evidence commit `c102c3c`. [Stage4 exact recipe and cleanup](../t50-shared-fb9c399/README.md)

Review the measured transforms by original path, finite values, geometry,
precision and registration logs. The older exact Hybrid geometry suggests a
one-pixel symmetric crop if both new identity-map and output shape witnesses
confirm it. With those checks, compare real weighted RGB readbacks to the
sharp truth across all 50 depth strips and retain full-image MAE/RMS and
region detail ratios. Report differing crop origins or transforms for
independent coordinate review. The sealed CLI publishes the fused weighted
RGB output and its completion event. It has no public per-pixel weight-map
readback with the frozen settings. Any independent weighted patch reconstruction
is an oracle derived from the verified source rasters and the two pinned
weight rules, distinct from an observed intermediate app buffer.

Consider a full campaign only after the exact diagnostic gives a credible
speed margin, valid matched tree RSS under the 20 ms rule and consistent
quality. It would use the same affinity and one unchanged stage for all
34 primary jobs and two backend checks. Scale the quiet reservation from
the new actual arm durations as 18 times their sum plus independent TIFF
decodes, rehashes and cold-cache margin. Retain complete samples and the
first warm pair for independent review. If a target or required counter
remains unresolved, retain the failed or pending status and prioritize the
corresponding bottleneck.

Rehash the two diagnostic export streams before removing the task-owned TIFFs
and caches. Reauthenticate all 52 originals after the last truth or source
read, then release the borrower and have the owner run the unchanged
`--cleanup-from` generator under the common lock. Retain command, revisions,
scope, observed metrics, checksum receipts and zero remaining files. Remove
the owned reference environment and package extraction after receipt sealing.
The viewer's separate frozen loaded p95 limit remains 33 ms, regardless of
experimental replay criteria.

### Replay command

This command is prepared for the future grant. Only a syntax parse was performed
for this review. No media, application, toolchain build or physical device was
started.
