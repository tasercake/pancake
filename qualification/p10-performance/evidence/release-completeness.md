# Frozen P10 evidence inventory

Both bounded CPU profiles in the dated release objective passed on
the exact notice-bearing `11cfcf8` package. E9 Exposure Fusion measured
warm 0.413300, cold 0.056313 and fresh matched RSS 0.315175. T50 Weighted
Average measured warm 0.468091, cold 0.394121 and fresh matched RSS
0.527495. The timing receipts preserve their over-limit RSS samples.
The separately admitted RSS schedules provide the revised release
memory results. Independent output and quality evidence is retained.
[Revised release objective](../../../pancake/docs/implementation-plan.md#revised-release-objective).
[E9 timing](integrated-e9-corrected-cpu-11cfcf8/README.md).
[E9 RSS](e9-cpu-rss-supplement-11cfcf8/README.md).
[T50 timing](t50-weighted-11c-limited-measured/README.md).
[T50 RSS](t50-weighted-11c-rss-limited-measured/README.md).

The original frozen campaign below remains at one of 26 logical
profiles qualified. Its schedules and telemetry retain their original
verdicts.

The [frozen target manifest](../../../pancake/docs/performance-targets.json)
defines 13 workload/method combinations, each measured with explicit CPU or
GPU selection. That gives 26 logical profiles. Vulkan and CUDA are separately
attributed NVIDIA implementations of the same GPU profile IDs. The frozen
comparison permits native CUDA against reference OpenCL on the same card.
The inspected target SHA-256 is `58bb68533311c48696a3a844d80eef2f37a3c1f39649305948c838b7002663dc`.

**One of 26 logical profiles has met every completed-job gate.** The
corrected-reference E9 CPU profile qualified at `c1e2cac`, `e81079d` and
the notice-bearing `0a9ae85` release,
each with 34 primary jobs, two CPU witnesses and the same independently
reviewed RGB16 pair. Five historical full-count schedules at `12f2151`
recorded 170 primary jobs and ten witnesses. Three used an ineligible E9
reference combiner. Three corrected E9 CUDA/OpenCL runs each completed
34 primary jobs and two witnesses. The standalone `481d805` run passed
warm, cold and process-tree RSS. Attributable GPU memory remains pending.
Three CPU profiles and one GPU profile have eligible full schedules.
Ten CPU and twelve GPU profiles lack a full eligible schedule.
[Latest exact packaged E9 CPU qualification](integrated-e9-corrected-cpu-0a9ae85/README.md).

## Profile coverage

**Qualified** means the exact profile passed its numeric and representative
quality gates. **Full count** means two warmups, ten warm and five cold
completed jobs per application. It records schedule coverage and leaves
any numeric, telemetry or quality issue open. **Ineligible** identifies the
historical E9 reference algorithm mismatch. **Open schedule** indicates
the full matched schedule is absent.

| Workload | Frozen method ID | Explicit CPU | Explicit GPU |
|---|---|---|---|
| T50 | `laplacian` | Open schedule | Open schedule |
| T50 | `hybrid` | Open schedule | Open schedule |
| T50 | `weighted_average` | Open schedule | Open schedule |
| T50 | `depth_map` | Open schedule | Open schedule |
| R50 | `laplacian` | Open schedule | Open schedule |
| R50 | `hybrid` | Full count at `12f2151` | Open schedule |
| R50 | `weighted_average` | Open schedule | Open schedule |
| R50 | `depth_map` | Open schedule | Open schedule |
| D150 | `laplacian` | Open schedule | Open schedule |
| D150 | `hybrid` | Full count at `12f2151` | Open schedule |
| D150 | `weighted_average` | Open schedule | Open schedule |
| D150 | `depth_map` | Open schedule | Open schedule |
| E9 | `exposure_fusion` | Qualified at `c1e2cac`, `e81079d` and `0a9ae85` | Corrected full CUDA/OpenCL schedules at `c1e2cac`, `e81079d` and standalone `481d805`. That last schedule passed warm at 0.494304. Device cadence and a full matched schedule at `0a9ae85` remain open. |

## Retained completed-job results

Ratios are Pancake/reference. Warm and each memory ratio must be at most
0.50. Cold must be at most 1.00. The `12f2151` E9 entries retain numeric
observations from the ineligible legacy reference combiner. The
`c1e2cac` CPU profile used the corrected equal final batch mean.
[E9 recipe and comparator audit](e9-reference-eligibility.md)

| Profile and exact Linux implementation | Warm ratio | Cold ratio | Process-tree RSS | Attributable GPU memory | Output quality |
|---|---:|---:|---|---|---|
| [R50 Hybrid CPU, packaged `12f2151`](integrated-r50-12f2151/README.md) | 2.325331, fail | 2.037556, fail | Pending cadence | — | Geometry matched with corrected reference RAW ingest. Color and selected-region detail remain open. |
| [D150 Hybrid CPU, packaged `12f2151`](integrated-d150-12f2151/README.md) | 0.491020, pass | 0.447042, pass | Pending cadence | — | Crop, doubled edges and detail remain open. |
| [E9 exposure fusion CPU, packaged `12f2151`](integrated-e9-12f2151/README.md) | 0.46036 | 0.06282 | 0.31883 | — | Ineligible comparator. RGB16 and common dimensions verified. |
| [E9 exposure fusion Vulkan/OpenCL, packaged `12f2151`](integrated-e9-12f2151/README.md) | 0.70500 | 0.09522 | 0.71987 | Pending cadence | Ineligible comparator. |
| [E9 exposure fusion CUDA/OpenCL, native `12f2151`](integrated-e9-cuda-12f2151/README.md) | 0.56340 | 0.08442 | 0.42523 | Pending cadence | Ineligible comparator. |
| [E9 exposure fusion CPU, packaged `c1e2cac`](integrated-e9-corrected-cpu-c1e2cac/README.md) | 0.425874, pass | 0.057144, pass | 0.318864, pass | — | Independent review passed with residual exposure differences and 364 additional floor and 731 additional ceiling samples. |
| [E9 exposure fusion CUDA/OpenCL, native `c1e2cac`](integrated-e9-corrected-cuda-c1e2cac/README.md) | 0.521168, fail | 0.078821, pass | 0.426292, pass | Pending cadence | Same sampled representative detail with residual exposure and endpoint differences. |
| [E9 exposure fusion CPU, packaged `e81079d`](integrated-e9-corrected-cpu-e81079d/README.md) | 0.421197, pass | 0.058141, pass | 0.316061, pass | — | The independently reviewed `c1e2cac` CPU pair repeated exactly. |
| [E9 exposure fusion CUDA/OpenCL, native `e81079d`](integrated-e9-corrected-cuda-e81079d/README.md) | 0.505452, fail | 0.074855, pass | 0.420741, pass | Pending cadence | Exact prior independently reviewed RGB16 pair and inputs. |
| [E9 exposure fusion CPU, packaged `0a9ae85`](integrated-e9-corrected-cpu-0a9ae85/README.md) | 0.409919, pass | 0.054769, pass | 0.311990, pass | — | The earlier independently reviewed CPU pair repeated exactly. |

The latest E9 campaign sampled device-memory lower bounds of 291,504,128 B
on CUDA and 320,864,256 B on OpenCL. Seventeen CUDA and four OpenCL traces
exceeded the 20 ms cadence ceiling. The strict paired peak and ratio remain
unavailable. Earlier repeated E9 campaigns
and backend/phase experiments cover the same logical IDs and add no profile
coverage. The [R50 packaged pair and phase diagnosis](../../../pancake/docs/p10-r50-phase-287828a/README.md)
and the [earlier physical T50 pair](t50-12f2151-preflight/pair-001/README.md)
are single matched diagnostics. The later exact `8694b55` Vulkan T50 Hybrid
pair took 22.171825 s versus 26.301475 s for OpenCL (ratio 0.842988).
The reference aligned on CPU and warped and fused on the RTX 3090. The
complete RGB16 exports repeated each target's earlier hashes. Paired RSS
and GPU coverage remain pending. Single observations supply no warm or cold
median. [Sealed T50 Hybrid diagnostic](t50-hybrid-pair-8694b55/README.md)

The exact `e81079d` T50 Weighted Average CPU pair measured 43.615742 s
against 65.857370 s (0.662276). The reference RSS gap was 25.190 ms and
the paired memory gate remains pending. The single diagnostic adds
no completed-job profile coverage.
[Measured T50 Weighted Average CPU pair](t50-weighted-cpu-e81079d/README.md).

The corrected `e81079d` GDB replay measured 18.921 s inside two original
RGB staging call sites. Both stock executions matched the TIFF bytes and
transforms. This diagnostic leaves the full T50 schedule open.
[Measured stock Weighted CPU phases](t50-weighted-phase-measured/README.md).

The authenticated T50 Weighted Average CPU triplet ran in 65.067805 s
on the pinned reference, 43.381687 s on the sealed `0a9ae85` package and
25.506298 s on standalone `a49ce69`. Both Pancake outputs and both map
families were exact matches. Independent review decoded all three RGB16
exports. The reference RSS acquisition exceeded 20 ms. This single
diagnostic leaves the full schedule and aggregate memory open.
[Exact Weighted Average CPU triplet](t50-weighted-triplet-0a9ae85-a49ce69/README.md).

At 3 ms nominal RSS sampling, a separate T50 triplet made all three
traces cadence-eligible. The scratch alias arm used 0.529305 of reference
process-tree RSS and 0.998586 of the standalone `a49ce69` arm's RSS. It
took 1.008061 of that arm's time and reproduced the exact output and
maps. The scratch product change remains isolated. This calibration
leaves the full matched schedule and frozen RSS verdict open.
[T50 scratch and RSS calibration](t50-memory-calibration-a49ce69-5f2623a/README.md).

## Native latency and matching release

The exact `8694b55` packaged viewer measured 200 idle and 200 loaded
events on the RTX 3090 with all 1,120 presented RGB patches validated.
Idle P95 passed at 23.811137 ms. Loaded P95 failed at 59.597912 ms
against 33 ms, with 33 of 200 events beyond the limit. Packaged
`287828a` had passed idle at 21.713 ms and missed under load at 33.818 ms.
The scoped `6489af6` candidate passed both at 20.793 and 23.827 ms.
[Exact `8694b55` viewer](../../../pancake/docs/p10-viewer-package-evidence/8694b55/README.md) ·
[Earlier packaged viewer](../../../pancake/docs/p10-viewer-package-evidence/287828a/README.md) ·
[Scoped viewer candidate](../../../pancake/docs/p10-viewer-command-evidence/README.md)

The exact `c1e2cac` stock package measured 52.322024 ms loaded P95 in
an ordered A/B test. The separate bounded-poll prototype measured
66.942485 ms. Both arms missed 33 ms and the prototype stayed outside
the product. [Stock and bounded-poll pair](../../../pancake/docs/p10-shared-poll-evidence/c1e2cac/README.md)

The packaged `d6359e1` lifecycle campaign passed all 120 acknowledgements.
The actual 64-path fixture and real D150 each contributed 30 pauses and
30 cancellations under load. Maximum pause and cancellation times were
150.773 ms and 151.283 ms against 250 ms. This audit found no later
same-release combined viewer and lifecycle pass.
[Exact lifecycle and settled-state evidence](native-lifecycle-d6359e1/README.md)

The `e81079d` Debian GUI passed all 120 physical acknowledgements on the
same workloads. Maximum pause and cancellation times were 211.515 ms and
212.889 ms. Its reducer had zero viewer observations. The matching native
latency gate remains open.
[Exact `e81079d` packaged lifecycle](native-lifecycle-e81079d/README.md).

## Holds and unavailable evidence

- **Host memory observation.** The full D150 and R50 CPU campaigns had
  process-tree sampling gaps above 20 ms. A dedicated collector core did
  not cure R50. Synthetic native procfs and private-namespace probes have
  narrower scope. An aggregate RSS result still needs complete timely
  process-tree observations or an equivalent verified high-water counter.
  [RSS cadence and containment evidence](resource-sampling/collector-cadence-followup.md) ·
  [Private procfs comparison](resource-sampling/rss-pid-namespace.md)
- **Device memory observation.** The installed NVIDIA process lists miss
  the 20 ms window on prior physical jobs. The local `ff9f5eb` overlap
  diagnostic found no complete Vulkan bypass and an inconclusive CUDA
  control. A new read-only check confirmed accounting is disabled on this
  RTX 3090 and the current session has no administrative access. Hardware
  calibration remains pending on an administrator-preenabled host. It
  must validate terminal CUDA and OpenCL maxima and complete GPU-owner
  provenance, with a separate Vulkan calibration before use in that lane.
  [NVIDIA per-call evidence](resource-sampling/collector-cadence-followup.md) ·
  [Accounting capability and lifetime requirements](resource-sampling/nvml-accounting.md) ·
  [Current read-only host check](resource-sampling/nvml-accounting-host-preflight.json)
- **Apple execution.** No final matching physical Apple CPU, wgpu Metal,
  native Metal, native GUI, or Apple footprint receipt is available here.
  These remain unverified along with exact Apple packaging. Retained builds
  are tracked separately. The historical `d6359e1` Windows build/link
  remains scoped to that revision. AMD/Intel GPU and Windows execution
  remain best effort. [Release matrix and Apple package prerequisites](../../../pancake/docs/p9-release-preparation.md#apple-and-retained-builds)

This inventory records existing evidence and leaves all frozen thresholds,
inputs and lane requirements unchanged.
