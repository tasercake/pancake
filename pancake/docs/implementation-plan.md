# Pancake implementation plan

## Revised release objective

| Release measure | Limit |
|---|---:|
| Median warm elapsed | At most 1/1.80 of reference |
| Median cold elapsed | At most 1.00 of reference |
| Peak process-tree RSS | At most 0.55 of reference |
| Actual RSS observation gap | At most 20 ms across each measured job, including launch and exit |

Both CPU profiles pass the revised local limits on the exact packaged
`11cfcf8` CLI. The ratio is Pancake divided by the pinned reference.

| CPU profile | Warm elapsed | Cold elapsed | Peak process-tree RSS |
|---|---:|---:|---:|
| E9 Exposure Fusion, nine real exposure originals | 0.413300 (10 pairs) | 0.056313 (5 pairs) | 0.315175 (3 fresh pairs) |
| T50 Weighted Average, 50 generated RGB16 TIFFs | 0.468091 (3 pairs) | 0.394121 (1 pair) | 0.527495 (1 fresh cold pair) |

E9's original full timing run had one 28.073 ms RSS gap. A separate
3 ms attempt stopped under contention and contributes no result. The
three fresh RSS pairs all met the unchanged 20 ms actual limit, with a
5.982 ms maximum gap. Independent review verified the RGB16 exports.
[E9 timing and interrupted attempt](../../qualification/p10-performance/evidence/integrated-e9-corrected-cpu-11cfcf8/README.md).
[E9 matched RSS evidence](../../qualification/p10-performance/evidence/e9-cpu-rss-supplement-11cfcf8/README.md).

The first T50 launch failed in the candidate wrapper. The corrected
ten-job schedule completed one warmup, three warm jobs and one cold job
per application. The warm median was 25.890339 s versus 55.310501 s and
the cold times were 25.667489 s versus 65.125906 s. One reference RSS
trace had a 22.399 ms gap. A separately preregistered fresh cold pair
sampled at 2 ms nominal and passed at 1,831,284,736 B versus
3,471,663,104 B, with 14.349 ms and 18.418 ms actual maximum gaps.
The outputs repeated their exact per-application RGB16 hashes. The
review covered the 50 ordered inputs, all 50 maps and independent
sharp-truth regions. These are the bounded release checks. The
original 34-job plus two-witness schedules retain their own verdicts.
[T50 invalid launch](../../qualification/p10-performance/evidence/t50-weighted-11c-limited-invalid-launch/README.md).
[T50 timing and quality](../../qualification/p10-performance/evidence/t50-weighted-11c-limited-measured/README.md).
[T50 matched RSS evidence](../../qualification/p10-performance/evidence/t50-weighted-11c-rss-limited-measured/README.md).

## Earlier host checkpoints

| Roadmap phase | Passed | Host evidence |
|---|---:|---|
| P5 motion, crop and registration | 64/64 | [P5 replay](p5-registration.md) |

## Active delivery checkpoint

The exact combined `c1e2cac` source passed the unchanged Linux CPU ASan and
TSan gates. Each ran 146 passing tests across 19 groups with no diagnostics.
The same 146 tests passed under stable CPU coverage. The strict coverage
comparator exited 1 with 4,658 of 6,475 required lines covered. Stable
instrumentation supplied no branch map. The earlier nightly coverage
verdict retains its separate source pin.
[Exact combined ASan](p9-cpu-assurance-evidence/c1e2cac/address/README.md).
[Exact combined TSan](p9-tsan-audit/integrated-c1e2cac/README.md).
[Exact stable CPU coverage](p9-cpu-assurance-evidence/c1e2cac/stable/README.md).

An ordinary CPU integration test exercised real native About on the exact
`fb9c399` application. Its PID- and build-ID-matched profile added 205
covered lines, 22 functions and 210 regions through the existing exporter.
The stable strict comparator still exited 1 on uncovered lines. The local
test prototype remains separate from the integrated suite.
[Native About coverage evidence](p9-cpu-assurance-evidence/fb9c399/native-about-target/README.md).

The scoped headless GTK child now keeps the compiler's profile path.
The same filtered test hits the error path once and emits separate
PID-bound parent and child profiles. Production code is unchanged
outside this test module. Strict full coverage and branch results remain
open. The latest sealed package replay identifies `11cfcf8`.
[Scoped GTK child profiles](p9-cpu-assurance-evidence/gtk-child-profile/README.md).

At `7748045`, the full fast gate and all 147 selected CPU dev tests
passed, with 19 test executables and zero skipped. Strict coverage and
the full mutation campaign remain open.
[Canonical fast and CPU receipts](p9-cpu-assurance-evidence/7748045/README.md).

The exact integrated `0a9ae85` source passed canonical fast and all 148
CPU dev tests across 19 executables. The unchanged Linux CPU ASan and
TSan gates each passed all 148 with zero diagnostics. Stable coverage
and nightly branch coverage both exited 1 at the strict comparator.
They covered 4,730/6,516 lines, 485/760 functions and 4,849/7,328 regions.
Nightly recorded 577/764 branch outcomes. Stable emitted no branch map.
[Fast and CPU dev](p9-cpu-assurance-evidence/0a9ae85/README.md).
[Stable coverage](p9-cpu-assurance-evidence/0a9ae85-stable-coverage/README.md).
[Nightly branches](p9-cpu-assurance-evidence/0a9ae85-nightly-branches/README.md).
[CPU ASan](p9-cpu-assurance-evidence/0a9ae85/address/README.md).
[CPU TSan](p9-tsan-audit/integrated-0a9ae85/README.md).

A test-only registration fixture at `d9ea867` verified that repeated
structure leaves a completed CPU stack unwarped. The exact nightly replay
passed all 149 tests and exercised the ambiguity guard's true branch once.
Branch coverage rose from 577 to 578 of 764 outcomes. The strict
comparator still exited 1. This receipt covers the registration test on
its own tested revision.
[Native registration abstention and nightly coverage](p9-cpu-assurance-evidence/registration-abstention/README.md).

The exact `870f7f9`, `5b98890`, `54e3a8d` and `11cfcf8` revisions passed
canonical fast and all 151, 160, 162 and 164 CPU dev tests, respectively.
Supplemental strict Linux Miri passed 26 tests on `5b98890` and 27 each
on `54e3a8d` and `11cfcf8`. The latest receipts include the Weighted
Average production change. The canonical Apple Miri gate remains open.
[First combined fast and CPU replay](p9-cpu-assurance-evidence/870f7f9/README.md).
[Second combined fast and CPU replay](p9-cpu-assurance-evidence/5b98890/README.md).
[Third combined fast and CPU replay](p9-cpu-assurance-evidence/54e3a8d/README.md).
[Weighted release fast and CPU replay](p9-cpu-assurance-evidence/11cfcf8/README.md).
[Latest supplemental Linux Miri](p9-cpu-assurance-evidence/11cfcf8-linux-miri/README.md).

The exact `11cfcf8` stable and nightly CPU coverage runs each passed
all 164 tests. Both strict comparators exited 1. Each recorded
4,799/6,596 lines, 494/771 functions and 4,923/7,416 regions. Nightly
recorded 589/778 branch outcomes. Stable emitted no branch map.
[Exact stable coverage](p9-cpu-assurance-evidence/11cfcf8-stable-coverage/README.md).
[Exact nightly branch coverage](p9-cpu-assurance-evidence/11cfcf8-nightly-branches/README.md).

The exact Linux CPU AddressSanitizer and ThreadSanitizer gates each
passed all 164 tests across 20 executables with zero diagnostics.
The randomized layout gate passed the same 164 tests under seeds 1 and
42. It retains 40 native executable identities and exact suite rosters.
[Exact AddressSanitizer replay](p9-cpu-assurance-evidence/11cfcf8/address/README.md).
[Exact ThreadSanitizer replay](p9-tsan-audit/integrated-11cfcf8/README.md).
[Exact Linux randomized layouts](p9-cpu-assurance-evidence/11cfcf8/linux-layouts/README.md).

The separate exact `11cfcf8` Linux CUDA and HIP CLIs compiled and linked
under their locked native feature graphs. The two physical CUDA smoke
runs also passed. HIP execution remains outside the build scope.
[Exact Linux native CUDA build](p9-linux-native-evidence/11cfcf8/cuda/README.md).
[Physical CUDA execution](../../qualification/p10-performance/evidence/native-cuda-11cfcf8-real-e9/README.md).
[Exact Linux native HIP build](p9-linux-native-evidence/11cfcf8/hip/README.md).

At exact `0a9ae85`, both Linux CPU randomized layout seeds passed all 148
tests, with 38 fingerprinted executable paths across the two builds.
Supplemental strict Linux Miri passed all 23 library tests. The canonical
Apple Miri gate remains separate.
[CPU layouts](p9-cpu-assurance-evidence/0a9ae85/linux-layouts/README.md).
[Supplemental Linux Miri](p9-cpu-assurance-evidence/0a9ae85-linux-miri/README.md).

The exact `0a9ae85` native Linux CUDA and HIP release CLIs linked with
their separate locked feature sets. Two real E9 smoke jobs on the exact
CUDA ELF completed on the RTX 3090 with zero fallback. Both reproduced
the prior RGB16 hashes and nine first-reference and sequential maps.
HIP execution was outside the build scope.
[Native CUDA build and smoke preparation](../../qualification/p10-performance/evidence/native-cuda-0a9ae85/README.md).
[Actual physical CUDA witnesses](../../qualification/p10-performance/evidence/native-cuda-0a9ae85-real-e9/README.md).
[Native HIP build](p9-linux-native-evidence/0a9ae85/hip/README.md).

At `25cc498`, the unchanged fast gate and all 147 CPU dev tests passed.
That source includes the two scan removals. The later FFT coefficient
change has its own scoped P5 and runtime results.
[Exact fast and CPU tests](p9-cpu-assurance-evidence/25cc498/README.md).

The `c1e2cac` source compiled and linked the native Linux CUDA and HIP
CLI profiles. Two CPU and two physical RTX 3090 CUDA cases ran through the
hashed CUDA release with zero fallback, and all four export digests matched
the previous native replay. The retained HIP receipt covers compilation
and linking.
[Exact native builds and executions](p9-linux-native-evidence/c1e2cac/README.md).

The private Apple Silicon packaging recipe records the locked build, SDK,
ARM64 linkage, executable digests and ad-hoc signing identity. Its six
portable staging tests passed on Linux. Native Mac build, package
signing, AX and Automation, and both Metal execution lanes remain
unrun here. The owner permits the Linux-first compatibility assumption
while Apple evidence is unavailable. Distribution signing and
notarization remain owner decisions.
[Private Apple package recipe](p9-release-preparation.md#apple-and-retained-builds).

The Linux packaging tools stage a common GPL-3 notice for the three
first-party Cargo entries and the pinned native LLVM notices. The exact
`e81079d` build receipt binds 112 LLVM archive inputs and 2,182 matching
object digests within the CubeCL rlib. It covers rlib membership. The
final ELF symbol and broader distribution notice scope remain separate.
The existing `e81079d` Debian archive predates this staging. A new
package needs matching build provenance, the `--native-llvm-receipt`
option, and rootless extraction checks.
[Native packaging notices and provenance](p9-native-package-provenance.md).

The exact `8694b55` native CUDA and HIP CLI release profiles both linked.
The CUDA build passed two CPU cases and two physical RTX 3090 CUDA cases
without fallback. All four exports matched the prior native replay hashes.
HIP execution remains unrun.
[Native profile builds and execution receipts](p9-linux-native-evidence/8694b55/README.md).

The unchanged nightly coverage gate at `7551f73` passed all 145 tests
across 19 CPU targets and exited 1 at the strict coverage threshold.
It covered 4,644/6,456 lines, 474/753 functions, 4,766/7,271 regions and
573/762 branch outcomes. The native GUI and mixed-lane counters require
separate evidence.
[Exact strict coverage run](p9-cpu-assurance-evidence/7551f73-nightly/README.md).

The exact `8694b55` supplemental Linux Miri run passed 22/22. Both
randomized-layout seeds passed 142/142 across 19 targets each, including
the real large TIFF and ICC corruption test. The canonical macOS gate
still needs a matching hosted run.
[Linux nightly receipts and artifact identities](p9-cpu-assurance-evidence/8694b55/linux-nightly/README.md).

At `287828a`, local Linux CPU ASan and TSan each passed 137/137
across 19 targets. The unchanged nightly gate ran 137 passing tests and
exited 1 at the strict coverage threshold, with 4,466/6,250 lines,
454/728 functions, 4,551/7,025 regions and 552/686 branch outcomes covered.
The later test-only `e63cadd` run passed 30/30 focused CPU fusion and image
I/O tests, including the weighted-focus window and PNG area limit. No
strict coverage result has been measured for that test delta.
[Exact ASan](p9-cpu-assurance-evidence/287828a/address/README.md).
[Exact TSan](p9-tsan-audit/integrated-287828a/README.md).
[Strict nightly gate](p9-cpu-assurance-evidence/287828a-nightly/README.md).
[Focused CPU tests](p9-cpu-assurance-evidence/e63cadd-focused/README.md).

Native mutation subpartition `255/256` at `287828a` executed all ten
selected mutants and exited 3 with six caught, three missed and one
timeout. The baseline passed 137/137. The other 2,573 mutants in that
exact roster remain unrun. Full mutation acceptance stays open.
[Exact native mutation shard](p9-mutation-audit/287828a-255-256/README.md).

The pinned native mutator repeated those same ten operations at exact
`c1e2cac` and caught all ten after successful mutant builds. The native
baseline passed 146 tests across 19 groups. The current roster contains
2,642 operations, with 2,632 outside this replay. The 165-row next-shard
projection contributes a verified roster and cost estimate. Full native
mutation acceptance remains open.
[Exact historical-operation replay](p9-mutation-audit/c1e2cac-historical-ten/README.md).
[Next-shard projection](p9-mutation-audit/c1e2cac-historical-ten/full-shard-projection.md).

The local `e81079d` recipe specifies a fresh ordered full roster, exact
partition, baseline and native mutant outcomes. A complete shard has
yet to run at that revision.
[Canonical local mutation recipe](p9-mutation-audit/e81079d-canonical-shard-plan/README.md).

The exact `0a9ae85` local mutator listed 2,672 unique candidates and
executed all 167 in partition 9/16. The baseline passed all 148 tests.
The partition failed with 108 caught, 45 missed, one timeout and 13
compiler-unviable operations. Cargo-mutants exited 3 and the strict
verifier exited 1. The semantic audit accounts for every missed or
timed-out row and distinguishes focused CPU and physical follow-ups.
The original verdict stands and final same-source mutation remains open.
[Exact native mutation preflight](p9-mutation-audit/0a9ae85-canonical-preflight/README.md).
[Complete native partition](p9-mutation-audit/0a9ae85-native-9of16/README.md).
[Exact adverse operation map](p9-mutation-audit/0a9ae85-semantic-map/README.md).

Focused CPU replays caught five operators recorded as missed on the
original `0a9ae85` partition. The tests verify actual CPU completion
identity, two-dimensional sharpness, Hybrid detail at two fine scales,
and normalized Depth Map focus in both axes. Each targeted mutant failed
and its restored stock control passed. The test changes leave the
application and the sealed package unchanged. The focused results leave
the original shard outcome and full gate separate.
[CPU completion fingerprint](p9-mutation-audit/runtime-fingerprint/README.md).
[Two sharpness operators](p9-mutation-audit/0a9ae85-sharpness-row/README.md).
[Hybrid fine-band selection](p9-mutation-audit/hybrid-fine-band-0a9ae85/README.md).
[Depth Map box normalization](p9-mutation-audit/0a9ae85-focus-box-normalization/README.md).

The next focused native replays caught the NumPy reader cap, RAW encoded
limit, Previous-reference progression, Breathing scale clamp and stale
worker outcome operators. The NumPy fixture covers reader tolerance at
the declared metadata offset. Independent NumPy accepted the exact limit.
The exact-cap Bayer fixture retained all 53,760 working samples and used
0.22 s and about 259 MiB more peak RSS than the unpadded decode. The
Previous-reference test checks the real +3 and −3 ordered steps. The
Breathing test uses an independently rendered expansion. The worker uses
its real bounded outcome channel and a test-only module.
[NumPy reader cap](p9-mutation-audit/npy-v2-limit/README.md).
[RAW encoded limit](p9-mutation-audit/0a9ae85-raw-encoded-limit/README.md).
[Previous-reference sequence](p9-mutation-audit/previous-reference-sequence/README.md).
[Breathing scale clamp](p9-mutation-audit/breathing-clamp/README.md).
[Native worker generations](p9-mutation-audit/worker-generation-0a9ae85/README.md).

The existing repeated-registration fixture also caught the saved
near-equal peak mutation. Its focused LLVM maps show the real ambiguity
guard taking the identity path on stock and passing through to an
unsupported transform under the mutant. These focused replays retain
the original shard's recorded outcomes.
[Near-equal registration peak replay](p9-mutation-audit/near-equal-registration-peak/README.md).

Five more saved operators failed their focused native tests and passed
after restoration. The Exposure residency test pauses a real CPU job
and confirms one extra 9,216-byte RGB allocation only for slab size zero,
with exact f32 and TIFF outputs in both controls. The geometry tests
check signed shear inversion, the scaled last-pixel crop, Euclidean
rigidity and rejection beyond the full search radius. This tranche
changes test code only, including two guarded unit-test modules.
[Exposure residency](p9-mutation-audit/0a9ae85-resident-rgb-retention/README.md).
[Public P5 geometry](p9-mutation-audit/0a9ae85-p5-geometry/README.md).
[Euclidean and search radius](p9-mutation-audit/estimator-geometry-0a9ae85/README.md).

Two further focused CPU replays caught the original Pearson reference
variance and weak spectral peak operators. The Pearson fixture recovers
a known translation in its textured control and abstains with the
low-variance reference. The pure spectral fixture brackets the same
cutoff using a reachable 64×64 proxy and checks the exact signed shift.
Both original mutations failed, and both restored controls passed. Full
mutation acceptance remains open.
[Pearson variance replay](p9-mutation-audit/pearson-variance-0a9ae85/README.md).
[Weak spectral peak replay](p9-mutation-audit/spectral-weak-peak/README.md).

The CPU Hybrid transpose and blue-detail Laplacian tests caught five
further exact operators. The physical Vulkan ordinal test rejected an
absent adapter on stock and caught the mutant selecting the actual RTX
3090 for that ordinal. The Vulkan result covers physical execution. The
original CPU-only shard verdict retains its own recorded scope.
[Four Hybrid pyramid operators](p9-mutation-audit/pyramid-axis-0a9ae85/README.md).
[Advanced blue luminance](p9-mutation-audit/advanced-blue-luma-0a9ae85/README.md).
[Physical Vulkan ordinal replay](p9-mutation-audit/0a9ae85-vulkan-ordinal/README.md).

Follow-up tests caught all three earlier survivors on their separately
verified revisions. The panorama test exercised the thumbnail budget
boundary. The two GUI tests exercised missing-source Reveal and native
GTK panel failure in a display-free child. The original shard verdicts
remain recorded. The full mutation gate remains open.
[Panorama mutation replay](p9-mutation-audit/2654813-panorama-budget/README.md).
[Native panel and Reveal replays](p9-mutation-audit/native-panel-and-reveal/README.md).

The warp now advances over the checked output extent and validates the
assembled frame dimensions. The exact forced-empty read mutation that
timed out after 60.058 s was caught in 0.254 s at `1ea0c4f`. The new CPU
oracle verifies all 264,579 samples across a partial final tile. The exact
`8460dd1` release passed all 64 P5 IDs with 63 native CPU cases and one
physical RTX 3090 Vulkan case. Its exports were byte-identical to the sealed
`8694b55` release, with exact transform parity. The native cascade probe
retained the same two warp allocation sizes. Full integrated coverage, mutation and
performance acceptance remain open.
[Warp mutation and exact P5 replay](p9-mutation-audit/warp-empty-read/README.md).

The same-source native CUDA and HIP release profiles linked. The CUDA
release passed two native CPU and two physical RTX 3090 CUDA cases with
matching export hashes and zero fallbacks. HIP has a retained build and link
receipt. [Exact native profiles and raw records](p9-linux-native-evidence/287828a/README.md).

### Hosted preview checkpoint

At `d6359e1`, the local Linux CPU ASan and TSan gates each passed 127/127 tests.
The unchanged nightly CPU coverage gate exited 1 with 4,202/5,932 covered
lines and 520/640 covered branch outcomes.
[Exact CPU sanitizer tests](p9-tsan-audit/integrated-d6359e1/README.md).
[Unchanged nightly coverage gate](p9-cpu-assurance-evidence/d6359e1-nightly/README.md).

The same-source optional CUDA and HIP CLI release profiles both linked.
The CUDA release passed two native CPU and two physical RTX 3090 CUDA cases.
The HIP receipt covers the retained build and link.
[Matching native backend builds](p9-linux-native-evidence/d6359e1/README.md).

Hosted Linux ASan passed 127/127. Hosted TSan reported two Fontconfig races
through parallel Slint/fontique test initialization after all 127 tests passed.
Hosted CPU coverage stopped at the uncovered-line gate. The Ubuntu platform
job timed out during release compilation after its fast and debug CPU checks
passed. Local commits `4c260b3` and `216f1f3` address the macOS documentation
lint and raise the native platform job limit to 45 minutes. Local changes
`2fa7c27` and `89fffbb` address the filesystem fixture and offscreen font
ownership. Hosted replay remains pending. The 2,504-candidate mutation roster
at `d6ac366` reconciled exactly across 16 listed shards. A separate native CLI
pilot caught five mutants and recorded one compiler-unviable variant.
Full-workspace mutation execution and strict owned-source assurance remain open.
[Raw hosted ASan](preview-ci-evidence/d6359e1/sanitizer-address.log.gz).
[Raw hosted TSan](preview-ci-evidence/d6359e1/sanitizer-thread.log.gz).
[Raw hosted coverage](preview-ci-evidence/d6359e1/coverage.log.gz).
[Observer path regression](p9-cpu-assurance-evidence/macos-observer/6d25836/README.md).
[GUI test ownership and TSan replay](p9-tsan-audit/hosted-gui-fontconfig-d6359e1.md).
[Full mutation roster](p9-mutation-audit/d6ac366/README.md).
[Executed mutation shards](p9-mutation-audit/f9af478-cli/README.md).

P9 retains the safe Slint interpreter for embedded repository-owned source,
with the upstream Rowan strict-Miri finding documented. Rootless Debian
validation covers the extracted executables, desktop entry, resolved native
libraries and archived dependency notices. Physical CubeCL Vulkan ASan at
`fcf395b` returned `Parent device is lost` during acquisition before any
kernel ran. Minimal wgpu and standalone C controls acquired the device and
reported LeakSanitizer failures.
[Interpreter release decision](p9-slint-boundary.md).
[Exact package verification](p9-linux-package-evidence/287828a/package-verification.json).
[Dependency-notice review](p9-package-notices.md).
[Physical GPU ASan replay](p9-gpu-asan-evidence/fcf395b/README.md).
[Apple packaging and signing prerequisites](p9-release-preparation.md#apple-and-retained-builds).

The packaged `d6359e1` GUI passed all 120 physical lifecycle requests on the
RTX 3090, with 60 pauses and 60 cancellations across the original 64-path
fixture and real D150. Maximum acknowledgements were 150.773 ms for pause
and 151.283 ms for cancel, both below the frozen 250 ms limit. The combined
latency reducer exited 1 with zero viewer observations in this campaign.
[Exact physical lifecycle campaign](../../qualification/p10-performance/evidence/native-lifecycle-d6359e1/README.md).

The exact `e81079d` Debian GUI also passed all 120 physical lifecycle
requests across the same 64-path fixture and real D150. Worst observed
acknowledgements were 211.515 ms for pause and 212.889 ms for cancel,
both within 250 ms. The capture supplied no viewer observations, so the
combined latency gate remains open.
[Exact package lifecycle replay](../../qualification/p10-performance/evidence/native-lifecycle-e81079d/README.md).

The exact packaged `287828a` GUI recorded all 400 physical viewer
observations with independent pixel validation. Idle P95 passed at
21.713240 ms and loaded P95 missed at 33.817853 ms against the 33 ms limit.
The single attempt ended with the frozen reducer at exit 1. All 1,120
presented RGB patches matched within one level per channel. The earlier
scoped `6489af6` pass at 20.793 ms idle and 23.827 ms loaded remains
attributed to that candidate. The exact package's loaded viewer gate and
combined viewer and lifecycle campaign remain open.
[Exact package viewer capture](p10-viewer-package-evidence/287828a/README.md).
[Earlier scoped viewer measurement](p10-viewer-command-evidence/README.md).

A separate packaged trace diagnostic verified 12 actual viewer transitions,
including six loaded Fuse inputs and 36 calibrated presented patches. The
GPU stack completed. The trace had no Vulkan API tables, leaving queue
attribution unverified. The frozen loaded result remains the package replay.
[Vulkan trace attempt and retained witnesses](p10-vulkan-queue-trace-evidence/287828a/README.md).

A directly profiled empty `287828a` GUI captured six Vulkan queue submits,
two image acquisitions and two presents from the exact packaged PID. The
launcher exited 1 after the shutdown wait timed out. This verifies API
transport and leaves loaded queue attribution and clean shutdown open.
[Direct Vulkan transport smoke](p10-vulkan-direct-smoke-evidence/287828a/README.md).

The exact packaged `8694b55` GUI passed idle P95 at 23.811137 ms and
missed loaded P95 at 59.597912 ms against 33 ms. It completed all 400
ordered events, with 1,120 independently validated pixel patches and 33
loaded events over the limit. A separate 12-input instrumented observation
verified Vulkan API capture during load. The frozen viewer result remains
the full uninstrumented package run, and the combined lifecycle campaign
remains open.
[Exact `8694b55` viewer replay](p10-viewer-package-evidence/8694b55/README.md).
[Scoped loaded Vulkan trace](p10-vulkan-loaded-trace-evidence/8694b55/README.md).

The later retained-result trace kept 200 idle and 80 loaded inputs and
verified all 784 observed pixel patches. Four late transitions showed
71.6–100.7 ms between Vulkan submit return and present entry alongside
long semaphore waits on another thread. That API trace established the host
interval and left the Rust wait site unresolved.
[Instrumented retained-result attribution](p10-vulkan-retained-result-evidence/8694b55/README.md).

A read-only audit examined the pinned CubeCL and wgpu source on
`c1e2cac` alongside that earlier `8694b55` trace. A shared-device read
guard can span the poller's native fence wait, and presentation requires
the corresponding write guard. The audit proposed measuring acquisition on
the same guard instance to distinguish lock contention from queue and
presentation delay.
[Shared-device poll and present audit](../../qualification/p10-performance/evidence/viewer-poll-lock-audit/README.md).

The `fb9c399`-based instrumented viewer used a common Linux monotonic
clock across the observer and Rust recorder. Nine uniquely matched loaded
events showed the rendering thread waiting 36.434–182.437 ms for the
same typed device lock held by a CubeCL poller. Its confirmed read
hold covered 36.429–182.416 ms of that wait. Seven late events
overlapped two possible presents and remain unresolved. The original
tight Nsight clock join was inconclusive. The instrumented run
validated all 784 patches and one physical RTX Vulkan completion.
Scheduler behavior, the frozen 33 ms target and the latency verdict
remain unchanged.
[Physical same-lock attribution](p10-vulkan-lock-attribution/fb9c399/README.md).

The instrumented `e81079d` viewer replay captured 200 idle and 80 loaded
actions, verified 784 pixel patches and completed a physical Vulkan job.
Of 17 loaded actions above 33 ms, the conservative join attributed eight
to an older draw and four to a new present waiting on the same device
lock. Five remained ambiguous. A separate drag/release sensitivity check
assigned those five without changing the primary verdict. The frozen
matched viewer gate remains open.
[Stock viewer pipeline attribution](p10-viewer-pipeline-trace/README.md).

The live private RTX 3090 Vulkan surface advertised FIFO and Immediate.
Four startup and resize configurations resolved to FIFO. Mailbox was
unavailable on this surface, and the experiment stopped before a
candidate build. The loaded viewer gate remains open.
[Live presentation mode contract](p10-mailbox-capability-0a9ae85/README.md).

An uninstrumented two-arm viewer experiment on the RTX 3090 measured
52.322024 ms loaded P95 on the exact `c1e2cac` package and 66.942485 ms
on the bounded-poll prototype. Both exceeded 33 ms. The loaded late
count rose from 35 to 45 of 200. Each arm completed 400 ordered viewer
events and verified 1,120 patches. The prototype stays outside the
product. All 52 authenticated T50 TIFFs were rehashed and removed after
both captures.
[Exact stock and experimental viewer pair](p10-shared-poll-evidence/c1e2cac/README.md).
[Shared T50 materialization and cleanup](../../qualification/p10-performance/evidence/t50-shared-c1e2cac/README.md).

The separate nonblocking poll experiment measured 42.208 ms loaded P95
on the stock `fb9c399` package and 68.007 ms on the prototype. Both
missed 33 ms. The two arms completed 400 events each and validated
1,120 patches each with one physical Vulkan completion per arm.
The product retains its existing polling behavior. The T50 owner
rehashed and removed all 52 staged files after both readers finished.
[Nonblocking viewer comparison](p10-shared-poll-evidence/fb9c399/README.md).
[Exact input and cleanup receipt](../../qualification/p10-performance/evidence/t50-viewer-ab-fb9c399/README.md).

### Benchmark baselines and scoped follow-ups

The earlier `12f2151` Linux CUDA and HIP release profiles both linked. The
CUDA build passed four frozen CLI cases, with two CPU completions and two
native CUDA completions on the RTX 3090. The HIP result is a build/link
receipt. Windows build/link was unverified at this revision.
[Native Linux profile evidence](p9-linux-native-evidence/12f2151/README.md).
[Windows build prerequisite](p9-windows-build-evidence/12f2151/README.md).

The host-aware RAW worker and allocation changes are merged at `f54096b`.
The focused release at `b10fbec` matched the merged production source and
lock. It decoded the authenticated first and last R50 originals into
bit-identical working RGB and byte-identical RGB16 exports. Median two-file
decode fell from 1.853 s to 0.782 s, with observed process peak RSS rising
from 630,312 to 639,534 KiB. All 84 P2 cases passed at that focused revision,
including 16 physical RTX 3090 Vulkan comparisons. Full paired R50 and the
post-merge integrated release ledger remain open.
[Exact Canon parity and timings](p10-r50-raw-audit/host-aware-workers/README.md).
[P2 CPU accounting](p10-r50-raw-audit/host-aware-workers/p2/p2-cpu-summary.json).
[P2 GPU accounting](p10-r50-raw-audit/host-aware-workers/p2/p2-gpu-summary.json).

On the 50 authenticated Canon originals, one untraced R50 CPU pair took
50.792 s with the exact packaged `287828a` CLI and 47.811 s with the
corrected pinned reference. Each decoded RGB16 export repeated its earlier
digest, and cross-application RGB mean absolute difference stayed at 506.42
u16 codes. A separate traced package run measured 18.087 s in RAW decode,
17.206 s in registration thumbnails and 14.340 s in frame fusion. This
diagnostic leaves the full warm, cold, process-tree RSS and equal-quality
verdicts open.
[Packaged R50 pair and stage trace](p10-r50-phase-287828a/README.md).

The separate `287828a` versus `07816a2` R50 CPU pair completed in
51.521 s and 42.072 s respectively. The encoded and decoded RGB16 TIFFs
matched, as did all 50 f32 alignment and sequential transforms. Separate
GDB runs measured registration thumbnails at 17.206 s and 7.944 s.
RAW decode and fusion remained the longest stages in the owned-buffer arm.
The comparison leaves frozen timing medians and process-tree RSS open.
The thumbnail lattice remains an experiment plan. The authenticated
originals and temporary outputs were removed.
[Owned-upload R50 evidence](p10-r50-phase-287828a/owned-upload/README.md).
[Thumbnail lattice proposal](p10-r50-phase-287828a/thumbnail-lattice-plan.md).

The `e81079d` R50 audit found a generic finite scan after Rawler
normalization and reproduced a 17.374 s ideal decode-overlap ceiling
from the earlier 50-frame trace. The adopted `c80d59a` change checks that
each developed sample has a U16 code, then checks the packed raster size.
The generic finite check still applies to arbitrary float inputs. The
scoped witness removed one RAW validation scan and retained both EXR
validation calls. All 23 library and 28 native image I/O tests passed.
[R50 coverage audit](p10-r50-e81079d-audit/README.md).
[Native scan and DNG parity evidence](p10-r50-typed-raw/README.md).

Two authenticated Canon CR3 images exercised parallel RAW workers on CPU
and physical RTX 3090 Vulkan. Within each runtime, the stock and candidate
matched exact RGB16 TIFF bytes and every f32 registration transform. A
single 50-image CPU Hybrid pair took 35.642573 s on stock and 34.944663 s
with typed RAW completion. The complete RGB16 exports and both transform
families matched. Direct-child peak RSS rose by 19,353,600 B. The frozen
CPU and GPU profiles and independent detail and color review remain open.
[Positive Canon CPU replay](p10-r50-typed-raw/canon-positive/README.md).
[Positive Canon physical Vulkan replay](p10-r50-typed-raw/physical-canon/README.md).
[Matched full R50 CPU diagnostic](p10-r50-typed-raw/full-r50-diagnostic/README.md).
[Verified removal of owned R50 media](p10-r50-typed-raw/full-r50-diagnostic/cleanup.json).
[Isolated luma disposition](p10-registration-luma-lattice/README.md#full-r50-follow-up).

The five-buffer Hybrid change removes a temporary RGB band while
retaining the existing per-pass storage binding limit. The exact `903836c`
CLI passed all 69 P4 cases, including real RTX 3090 Vulkan work. On each of
CPU, Vulkan and native CUDA, the isolated stock and candidate paths matched
235,962 f32 channel values across 18 fixtures. The earlier seven-tensor
prototype remains experimental.
[P4 and Vulkan receipts](p10-r50-hybrid-band/README.md).
[Exact native CUDA parity](p10-r50-hybrid-band/native-cuda/README.md).

The RAW packing change uses the existing bounded RAW pool on eligible
large frames. Its exact `c8f68cf` CLI passed all 84 P2 IDs, including 16
physical RTX 3090 Vulkan completions. The same source passed 20/20
supplemental Linux Miri library tests. The canonical macOS gate and the
combined package replay remain open.
[RAW packing and exact P2/Miri receipts](p10-r50-raw-packing-residual/README.md).

At the packaged `12f2151` CLI, the aligned real E9 comparison completed 17
launches per application in each CPU and physical NVIDIA lane. CPU warm,
cold and peak RSS ratios against the historical reference were 0.4604,
0.0628 and 0.3188. Vulkan/OpenCL warm, cold and RSS ratios were 0.7050,
0.0952 and 0.7199. GPU-memory sampling gaps and cross-application
output-quality alignment remain open.
[Exact E9 measurement and quality witnesses](../../qualification/p10-performance/evidence/integrated-e9-12f2151/README.md).

The separate `12f2151` native CUDA release completed the matched real E9
campaign on the same RTX 3090. The historical CUDA/OpenCL warm, cold and
peak RSS ratios were 0.5634, 0.0844 and 0.4252. Candidate GPU-memory traces
exceeded the required sampling interval, and equal output quality remains
open.
[Native CUDA E9 measurement](../../qualification/p10-performance/evidence/integrated-e9-cuda-12f2151/README.md).

The completed-job evidence inventory covers 26 frozen profiles. One
profile has passed, E9 exposure fusion on explicit CPU at the exact
`c1e2cac` Debian package. The historical E9 CPU, Vulkan and CUDA schedules used a
reference that reran the legacy exposure combiner over batch outputs. The
frozen contract requires equal averaging of the batch outputs. Those runs
retain their historical ratios as diagnostics.
[Full P10 profile accounting](../../qualification/p10-performance/evidence/release-completeness.md).
[E9 reference eligibility audit](../../qualification/p10-performance/evidence/e9-reference-eligibility.md).

The official corrected-reference E9 CPU campaign completed all 34 primary
jobs and two separate backend witnesses. Pancake/reference ratios were
0.425874 warm median, 0.057144 cold median and 0.318864 peak process-tree
RSS, all within frozen limits. Each RSS stream met the 20 ms cadence cap.
Independent review accepted the representative RGB16 pair and its three
fixed windows. The Pancake image had 364 additional floor samples and
731 additional ceiling samples elsewhere, with no visible texture loss
at native scale. The verdict covers the exact `c1e2cac` package, these
nine E9 inputs and explicit CPU.
[Qualified E9 CPU profile](../../qualification/p10-performance/evidence/integrated-e9-corrected-cpu-c1e2cac/README.md).

The sealed `e81079d` Debian package qualified the same E9 CPU profile
with 34 primary jobs and two CPU witnesses. Warm, cold and process-tree
RSS ratios were 0.421197, 0.058141 and 0.316061. The exact input and
RGB16 hashes match the independently reviewed CPU pair. This remains
one qualified logical profile of 26.
[Exact packaged E9 CPU result](../../qualification/p10-performance/evidence/integrated-e9-corrected-cpu-e81079d/README.md).

The exact notice-bearing `0a9ae85` Debian package qualified the same E9
CPU profile again. Its 34 primary jobs and two CPU witnesses measured
warm, cold and process-tree RSS ratios of 0.409919, 0.054769 and 0.311990.
The exact reviewed RGB16 pair and nine source hashes repeated. The
completed-job aggregate remains one qualified logical profile out of 26.
[Latest exact packaged E9 CPU result](../../qualification/p10-performance/evidence/integrated-e9-corrected-cpu-0a9ae85/README.md).

The corrected E9 physical CUDA/OpenCL campaign also completed all 34
primary jobs and two backend witnesses. The warm ratio of 0.521168
missed 0.50. Cold and process-tree RSS passed at 0.078821 and 0.426292.
GPU observation gaps exceeded 20 ms on both applications. Independent
review accepted sampled representative detail and retained the endpoint
clipping differences. This leaves E9 CPU as the single qualified profile.
[Corrected E9 CUDA/OpenCL result](../../qualification/p10-performance/evidence/integrated-e9-corrected-cuda-c1e2cac/README.md).

The exact integrated `e81079d` native CUDA/OpenCL campaign completed
34 fresh primary jobs and two backend witnesses. Warm median was 0.505452
of the reference, 4.295 ms over the 0.50 limit. Cold median and
process-tree RSS passed at 0.074855 and 0.420741. Attributable GPU
memory remains pending after 17 CUDA and four OpenCL gaps over 20 ms.
The first warm pair reproduced the prior independently reviewed RGB16
images and inputs exactly. The frozen GPU profile remains failed and
the completed-job inventory stays at one qualified profile out of 26.
[Exact integrated E9 CUDA/OpenCL result](../../qualification/p10-performance/evidence/integrated-e9-corrected-cuda-e81079d/README.md).

The adopted registration upload reuse extends the checked, frame-local
RGB handoff to slab-free Exposure Fusion. The exact `481d805` change
passed 48 scoped CPU tests and reproduced f32 channels, TIFF bytes and
transforms across 20 physical RTX 3090 CUDA and Vulkan jobs. The nine-frame
identity fixture eliminated all nine extra fusion uploads. The translated
and mismatched-canvas controls kept the upload each required.
[Physical upload and parity evidence](../../qualification/p10-performance/evidence/e9-exposure-resident/README.md).

The same-source E9 CUDA compiler experiment on the RTX 3090 measured
452.260 ms warm median for LLVM and 737.959 ms for C++, with exact RGB16
and transform parity. The faster LLVM profile remains the release profile.
The frozen E9 reference comparison retains its earlier verdict.
[Native CUDA compiler diagnostic](../../qualification/p10-performance/evidence/e9-cuda-compiler-diagnostic/README.md).

The FFT coefficient table moves trigonometry into the first window
dispatch and reuses the values across both forward transforms and the
inverse. The earlier host-only diagnostic reduced measured first-use
compiler time by 7.77 ms at side 64 and 10.24 ms at side 128.
[Host first-use compiler evidence](../../qualification/p10-performance/evidence/e9-cold-kernel-diagnostic/README.md).

The exact coefficient-table candidate `a919045` matched six native CPU
spectral fixtures and all 380 coefficient-angle bits. All 64 mapped P5
cases passed, including 63 CPU cases and one physical RTX Vulkan case.
Separate stock and candidate runs matched 122,880 f32 channels bit
for bit on each of native Vulkan and CUDA.
[Host parity and native link](../../qualification/p10-performance/evidence/e9-lut-75c-host/README.md).
[Mapped P5 and two physical runtime comparisons](../../qualification/p10-performance/evidence/e9-lut-portability/README.md).

Four paired native CUDA E9 observations on the RTX 3090 measured a
410.183 ms stock warm median and 393.651 ms with the table, a 4.03%
diagnostic reduction. All 12 jobs preserved exact RGB16 exports and
both registration map families. The existing corrected CUDA/OpenCL
warm miss retains its original revision and verdict. The integrated
source needs a fresh full package replay and matched E9 campaign.
[Paired CUDA E9 diagnostic](../../qualification/p10-performance/evidence/e9-cuda-lut-diagnostic/README.md).

The read-only E9 startup audit found two CUDA driver loader opens in the
first discovery path. Historical discovery took 13.075 ms of 116.574 ms
runtime setup at the medians. Loader reuse remains a proposed experiment.
[Native CUDA startup audit](../../qualification/p10-performance/evidence/e9-native-startup-audit/README.md).

The packaged `12f2151` GUI recorded all 400 calibrated visible transitions on
physical RTX 3090, with 200 idle and 200 loaded events. The 41.121 ms idle and
41.363 ms loaded P95 values both missed the 33 ms limit. The unmerged X11
redraw branch `1a0ec8c` also missed at 41.492 ms idle and 41.335 ms loaded.
Both arms matched the independent pixel witnesses.
[Sealed viewer and redraw A/B](p10-viewer-redraw-evidence/README.md).
[Earlier viewer cache measurement](p10-viewer-cache-evidence/README.md).

The packaged `12f2151` D150 Hybrid CPU campaign completed all 34 primary
launches. Warm medians of 43.36 s and 88.30 s gave a 0.491 ratio, and the cold
ratio was 0.447. Both speed gates passed. Process-tree peak RSS remains pending
under the frozen sampling interval. Same-canvas review confirmed softer
specimen detail and doubled edges. The unmerged adjacent-fit prototype
`ff2c037` removed the observed 129.796 px jump and widened the crop from
2130 × 1530 to 2302 × 1726 in a full 150-frame metadata and export check.
Detail remained soft in the same held-out regions. Equal quality remains open.
[D150 paired timings and image review](../../qualification/p10-performance/evidence/integrated-d150-12f2151/README.md).
[Adjacent-pair geometry proof](d150-geometry-audit/prototype-ff2c037/README.md).

The held-out `0cc9159` D150 prototype reduced the largest nearby-frame map
jump to 2.607 px and produced a 2266 × 1660 crop. All four original focus
regions became softer than the sealed package. On a separate 40-frame
control using identical registered pixels, the band-local fusion prototype
degraded signed phase in both held-out regions. These experiments leave
registration and equal output quality open.
[Native motion and warp review](d150-geometry-audit/subpixel-diagnostic/README.md).
[Fixed-input phase results](../../qualification/p10-performance/evidence/d150-band-focus/README.md).

Two later D150 diagnostics retain the `287828a` package as control.
Bounded cubic sampling improved the analytical subpixel witness and kept all
150 transforms and the full-stack crop identical. It slightly softened all
four historical focus regions. The separate Hybrid coherence prototype on
40 identically registered inputs improved signed projection by 10.6% and
4.1% in the two held-out regions. Unsigned detail fell by 9.9% and 13.0%,
and signed detail remained below the pinned reference. Both prototypes
remain separate from the packaged build. Equal-quality acceptance is open.
[Full-stack cubic pair](d150-geometry-audit/breathing-sampling/README.md).
[Fixed-input Hybrid pair](../../qualification/p10-performance/evidence/d150-hybrid-coherence-287828a/README.md).

A nine-original D150 probe found that inverted reverse-anchor proposals
reduced held-out bilateral transfer errors at both selected transitions.
Direction choice, whole-stack geometry and fused quality remain open.
The standalone Hybrid expansion audit measured phase leakage too small in
its analytic witness to attribute the observed D150 quality gap to the
expansion stencil. These findings remain diagnostic.
[Reverse-anchor motion study](d150-geometry-audit/motion-admission/README.md).
[Hybrid expansion audit](../../qualification/p10-performance/evidence/hybrid-math-review-287828a/README.md).

The passive D150 coordinate replay supports a bounded proxy witness
experiment and shows the discontinuity from switching one absolute pose
between forward and reverse history. Actual luma proxy texture, admission,
and full-stack quality remain unverified. The separate reviewed seven-tensor
Hybrid prototype can exceed imported wgpu bind group limits and remains
experimental. A revised five-buffer split addresses that source-level issue.
The split and thumbnail lattice still need physical parity. The reviewed
lattice revision also has a stale source inventory.
[Passive D150 witness design](d150-geometry-audit/witness-design/README.md).
[Hybrid binding and rounding review](../../qualification/p10-performance/evidence/hybrid-fused-kernel-review/README.md).
[Hybrid split and luma lattice review](../../qualification/p10-performance/evidence/hybrid-split-luma-review/README.md).

The experimental D150 CPU witness checked six intervals across nine
authenticated originals. On the genuine step it found inverse-reverse
support in eight of nine cells. The false direct jump was contradicted
where the three hypotheses had comparable losses. The sparse support left
the full-canvas replacement unresolved. The product path retains its current
registration behavior.
[Actual D150 proxy observations](d150-geometry-audit/witness-pilot/README.md).

The separate CPU batching diagnostic checked six authenticated D150 edges
over 42 paired observations. All 3,240 scalar moments matched bit for bit.
Median sampled witness reduction fell from 18.632 ms to 0.597 ms.
The existing maps and supporting cells matched. Full-canvas registration,
crop, fused output, GPU cost and frozen D150 timing remain open.
[Batched witness replay](d150-geometry-audit/witness-batching/README.md).
[Connected pose and required evidence](d150-geometry-audit/connected-pose/README.md).

The anchored D150 review confirmed that the first of 149 consecutive
edges is the only one without four independent witnesses. The measured
short and long spans leave the first anchor and the late skip edges
unresolved. The false jump remains contradicted by nine native matches.
The predeclared recovery probe leaves output quality and performance
open.
[Exact anchor and skip-edge review](d150-geometry-audit/anchored-recovery-review/README.md).

The preregistered D150 CPU trial ran once on 11 authenticated originals.
All 12 fresh directed estimates had zero confidence, leaving all six
fresh edges unresolved. The historical jump failed the hold-out spatial
screen, so the optional packed trial remained unrun. A separate synthetic
native test recorded zero packed dispatches when no cells overlapped and
preserved the following neighbor bit for bit. The diagnostic source and
overlays stay under the evidence path. The private borrowed copies were
rehash-verified and removed.
[D150 anchor and skip-edge evidence](d150-geometry-audit/anchor-skip-prereg/README.md).

One matched generated T50 Hybrid diagnostic pair used the packaged `12f2151`
Vulkan CLI and the reference OpenCL runtime on the same physical RTX 3090.
Pancake took 82.44 s and the reference took 26.74 s. Both exported genuine
3998 × 5998 RGB16. The reference fell back to CPU in its alignment helper
while warping and fusion completed on the GPU. The diagnostic carries no
frozen warm or cold timing credit. Resource and equal-quality gates remain open.
[Generated T50 diagnostic and cleanup](../../qualification/p10-performance/evidence/t50-12f2151-preflight/pair-001/README.md).

The two-scan candidate ran two balanced same-input physical T50 Hybrid
pairs against the exact `fb9c399` package. Stock and candidate took
21.966→19.544 s and 22.046→19.513 s, an 11.0–11.5% diagnostic reduction.
All four RGB16 outputs and both sets of 50 identity maps matched.
RSS gaps met the 20 ms cap. All GPU-memory streams exceeded it.
The owner rehashed and removed all 52 TIFFs after both diagnostics
released their reads. The frozen matched-reference schedule remains open.
[Exact ABBA diagnostic](p10-dual-scan-evidence/e5b6260/README.md).
[Shared input and owner cleanup](../../qualification/p10-performance/evidence/t50-shared-fb9c399/README.md).

The exact `e81079d` package completed one T50 Weighted Average CPU pair
against the pinned reference. Pancake took 43.615742 s and the reference
took 65.857370 s, a ratio of 0.662276. Both used the same 50 authenticated
TIFFs and produced independently reviewed RGB16 outputs. The reference
RSS trace exceeded the 20 ms cadence cap. The full schedule, frozen
quality judgment and paired RSS gate remain open.
[Eight T50 profile priorities](../../qualification/p10-performance/evidence/t50-profile-priority-e810/README.md).
[Exact T50 Weighted Average CPU diagnostic](../../qualification/p10-performance/evidence/t50-weighted-cpu-e81079d/README.md).

A separate Weighted CPU attribution attempt completed its stock control
in 43.383004 s. The runner stopped at an ordered-map schema error before
GDB started. The fixed verifier passed five tests using the actual event
and malformed controls. The attempt supplies no measured phase times.
[Stock control and verifier correction](../../qualification/p10-performance/evidence/t50-weighted-phase-attempt/README.md).

The corrected stock trace completed at exact `e81079d` in 45.932933 s
against its 43.569249 s untraced control. Registration and Weighted add
RGB staging accounted for 18.921 s of traced host-call intervals. The
two RGB16 exports and all maps matched exactly. This attributes time to
staging calls and leaves eventual savings and frozen performance open.
[Measured stock Weighted CPU phases](../../qualification/p10-performance/evidence/t50-weighted-phase-measured/README.md).

The adopted `a49ce69` Weighted Average change reuses the checked
registration RGB allocation for slab-free inputs that match the canvas
and have an exact identity transform. The native CPU cohort passed 49
tests. Six physical Vulkan pairs and six native CUDA pairs preserved
exact f32 samples, RGB16 TIFF bytes and all transforms on the RTX 3090.
The nine-frame identity controls eliminated all nine redundant uploads.
The warp, resize, unaligned and slab controls kept their existing routes.
[Weighted Average host, Vulkan and CUDA evidence](../../qualification/p10-performance/evidence/t50-weighted-resident/README.md).

The authenticated 50-frame CPU T50 triplet took 65.067805 s on the pinned
reference, 43.381687 s on the sealed `0a9ae85` package and 25.506298 s on
the standalone `a49ce69` candidate. Pancake's two exports and both map
families matched exactly. Independent review decoded all three outputs
and checked 50 fixed regions. The candidate took 0.391996 of reference
wall time in this single diagnostic. The reference RSS gap exceeded
20 ms. Full warm, cold, memory and quality qualification remains open.
[Authenticated T50 Weighted Average CPU triplet](../../qualification/p10-performance/evidence/t50-weighted-triplet-0a9ae85-a49ce69/README.md).

A second T50 CPU triplet compared standalone `a49ce69` with the scratch
energy-plane reuse candidate `5f2623a`. All three process-tree RSS traces
met the 20 ms gap limit when sampled at a 3 ms nominal interval. The
scratch candidate used 0.529305 of reference RSS, above 0.50, and used
0.998586 of `a49ce69` RSS while taking 1.008061 of its wall time. Both
Pancake outputs and maps matched exactly. The scratch change remains
isolated. This one triplet leaves the frozen campaign open.
[Measured Weighted scratch calibration](../../qualification/p10-performance/evidence/t50-memory-calibration-a49ce69-5f2623a/README.md).

One generated T50 Hybrid run on the unmerged `64fbdb6` trace build completed
in 73.214 s on physical RTX 3090 Vulkan and matched the earlier sealed TIFF
byte for byte. Decode and the two full-frame upload call spans totalled
63.94 s, about 87.5% of the owner-thread time. These are host intervals
that can include copies, submission backpressure and prior queue work. The
two uploads staged 28.8 GB of logical RGB data across 50 frames. This single
instrumented diagnostic carries no frozen P10 qualification credit.
[Physical T50 phase trace](p10-t50-phase-trace/README.md).
[T50 transfer boundaries](../../qualification/p10-performance/evidence/t50-stage-audit/README.md).

The identity-aligned Hybrid resident RGB path merged at `287828a`. A
same-binary physical Vulkan diagnostic on the RTX 3090 reduced wall time
from 72.661 s to 57.064 s with reuse enabled. It removed 50 repeat RGB
uploads totaling 14.4 GB of logical data and preserved the exact TIFF bytes,
50 transforms and completion events. The separate bounded tiled-TIFF
diagnostic preserved all sample bits across nine paired decode observations.
Frozen end-to-end speed and memory remain open. The shared `287828a` stage
verified all 52 generated file and independently decoded pixel records.
After all three borrowers released the stage, all 52 TIFFs were authenticated
again and removed.
[Physical resident RGB comparison](p10-t50-resident-input/README.md).
[Bounded TIFF decode](p10-tiled-tiff-diagnostic/README.md).
[Verified T50 stage](../../qualification/p10-performance/evidence/t50-shared-287828a/README.md).

The one-copy registration upload at `07816a2` passed all 64 P5 and 72 P6
case IDs. The two disjoint replays used the same locked candidate CLI.
The 73-case continuation audited 101 calls, including 33 native CPU jobs,
37 physical RTX 3090 Vulkan jobs and 31 expected corrupt-input errors.
All five Auto jobs used the physical GPU, and completed calls had zero
fallbacks. The integrated TIFF and registration source passed the later
`8694b55` package replay.
[P5/P6 union and raw device records](p10-registration-owned-upload/device-regression.json).

The two separately built `287828a` and `07816a2` CLIs ran one matched
physical T50 pair on the RTX 3090. Wall time fell from 50.533 s to 37.223 s
and process-tree RSS from 1.548 GB to 0.969 GB, with both RSS traces within
the frozen cadence. The encoded and decoded RGB16 output, 50 identity
transforms and completion records matched. VRAM sample gaps exceeded 20 ms.
The frozen cross-application speed, memory and quality verdicts remain open.
[Registration upload A/B and allocation proof](p10-registration-owned-upload/README.md).

The bounded TIFF row-worker candidate `e228c68` passed all 84 P2 IDs,
with 68 CPU-only IDs and 16 GPU-bearing IDs. The latter recorded 16
physical RTX 3090 Vulkan completions plus their native CPU comparisons.
The independent supplemental Linux Miri harness passed both copied pure
cursor tests. The real TIFF and ICC filesystem checks remain in the native
integration lane. The canonical macOS workspace gate remains open.
[Exact P2 accounting](p10-tiff-row-worker/p2/qualification.json).
[TIFF worker and Miri review](p10-tiff-row-workers-review/README.md).

The separately built `287828a` serial and `e228c68` row-worker CLIs ran one
matched physical T50 diagnostic. Wall time fell from 55.539 s to 38.517 s.
Both observed process-tree RSS peaks were about 1.55 GB with compliant RSS
cadence. The encoded and decoded RGB16 output and all 50 identity transforms
matched. GPU memory sample gaps were 93.134 and 116.456 ms. The launches
started at P8/56 °C and P5/58 °C. Compiler-cache cleanup overlapped at least
1.947 s of the serial opening and finished before the row-worker launch.
The timing impact is unknown. The row-worker and one-copy upload effects
were measured in separate A/B pairs. The combined source passed the
`8694b55` release replay. The frozen cross-application comparison remains open.
[Physical TIFF worker pair](p10-tiff-row-worker/README.md).
[Cleanup overlap bound](p10-tiff-row-worker/physical/cleanup-overlap.json).

The decode overlap audit retains ordered current-frame reads. Its archived
trace model bounds the benefit and identifies the extra memory, input-order
and pause acknowledgement proof needed for speculative decode.
[Decode overlap and ownership audit](p10-decode-overlap-audit/README.md).

The process-tree RSS cadence and attributable GPU memory gates remain open
where the traces exceeded 20 ms. The native RSS probe returned the same
session roster and byte totals as the Python collector in its stable control.
It has not qualified full-job cadence. Independent E9 replay measured individual
NVIDIA compute-process queries lasting 26.972 ms on Vulkan and 30.815 ms on
CUDA. The archived T50 pair also had GPU sample gaps above 20 ms.
[Raw cadence replays and collector controls](../../qualification/p10-performance/evidence/resource-sampling/collector-cadence-followup.md).

The synthetic NVML overlap check let a second complete query pair finish
during an independent stall. A shared lock prevented that progress. Archive
replay and timing controls leave physical query independence and coherent
peak attribution unverified. Frozen cadence remains unchanged.
[NVML overlap control and recorded bounds](../../qualification/p10-performance/evidence/resource-sampling/nvml-overlap.md).

Four physical E9 diagnostic launches compared serial and overlapping NVML
acquisition on the RTX 3090. The Vulkan overlap arm produced zero complete
bypasses, and the CUDA serial control did not reproduce a compute call
longer than 20 ms. Every arm exceeded the memory sampling limit. Frozen
device-memory qualification remains open.
[Physical E9 overlap observations](../../qualification/p10-performance/evidence/resource-sampling/nvml-overlap-runner.md).

The corrected E9 audit reproduced 17 exit-boundary CUDA gaps and four
interior OpenCL gaps above 20 ms from the 34 primary traces. The largest
gaps were 34.085 and 39.221 ms. Terminal per-PID high-water accounting
was disabled on the recorded device. A qualified job peak still needs
verified complete GPU-process ownership and symmetric peak counters,
or a cadence-valid attributed series.
[Corrected E9 GPU memory attribution audit](../../qualification/p10-performance/evidence/resource-sampling/e9-corrected-nvidia-peak-audit.md).

A separate private procfs diagnostic preserved the measured RSS roster and
sum across a cgroup move. The working unprivileged namespace changed
source-visible supplementary groups and `getpid()`. Passive kernel task
events were unavailable under the existing permissions. Process-tree RSS
retains its original definition and cadence.
[PID namespace and lineage audit](../../qualification/p10-performance/evidence/resource-sampling/rss-pid-namespace.md).

Final release acceptance, the final GitHub push of `main`, and required CI
on that final revision remain open while frozen gates are unmet.

### Earlier Linux release and performance receipts

The isolated release replay passed all 20 P7 and 14 P8 interface IDs. It
verified native panels, retained results, GUI and CLI export parity, and
the actual 64-frame pause, resume, cancel, and worker reuse workflow.
[Interface evidence](p7-ui-evidence/5216ba4/README.md) and
[Linux package evidence](p9-linux-package-evidence/README.md).

Apple execution, retained platform builds, mutation/sanitizer assurance,
full owned-source coverage, end-to-end CUDA qualification, genuine driver
fault recovery, and the frozen P10 measurements remain separate exit gates.
[Sealed Linux branch coverage](p9-cpu-assurance-evidence/5216ba4-nightly/README.md).

**The `5216ba4` Linux lifecycle campaign passed. P10 release acceptance
remains open.** The physical RTX 3090 GUI met the frozen 250 ms deadline on
all 120 requests, comprising 60 pauses and 60 cancels across the original
64-path fixture and real D150.
Maximum observed acknowledgement was 155.640 ms for pause and 221.802 ms
for cancellation. The comparison clip shifted source pixels with the divider
on that sealed GUI. The corrected `74dfaa3` build matched independent physical
24 MP witnesses for all viewer operations. Its full 200 idle and 200 loaded
observations both missed the frozen 33 ms P95 limit, at 42.310 ms idle and
54.054 ms loaded.
[Lifecycle measurements](../../qualification/p10-performance/evidence/native-lifecycle-measured/README.md),
[physical comparison evidence](../../qualification/p10-performance/evidence/native-physical-pair-calibration/comparison.md),
and [400-event viewer measurement](../../qualification/p10-performance/evidence/native-viewer-measured/README.md).

The registration-fixed `5e10244` diagnostic matched all 17 E9 CPU launches
per application. Warm median was 1.086 s versus 0.899 s (ratio 1.208, limit
0.50). Cold median was 1.075 s versus 6.535 s (ratio 0.164, limit 1.00).
The sealed `5216ba4` E9 Vulkan/OpenCL diagnostic used the same RTX 3090 for
all 17 completed runs per application. Warm median was 0.733 s versus 0.770 s
(ratio 0.952, limit 0.50). Cold median was 0.931 s versus 7.037 s (ratio 0.132,
limit 1.00). CPU and GPU memory observations exceeded the sampling-gap
specification, and equal output quality remains unresolved. The remaining
completed-job profiles and end-to-end speed, memory and quality acceptance
remain open.
[CPU E9 receipt](../../qualification/p10-performance/evidence/e9-measured-diagnostic/README.md),
[NVIDIA E9 receipt](../../qualification/p10-performance/evidence/e9-vulkan-diagnostic/README.md),
and [real-scale T50/R50/D150 functional evidence](../../qualification/p10-performance/evidence/scale-functional/README.md).

[Consolidated P0 work log, including earlier work](p0-work-log.md) is the evidence
entry point. Historical reports retain their original outcomes; this plan owns
current scope and exit requirements.

## 1. Objective and acceptance boundary

Write each production image-processing kernel once in CubeCL. Execute those
definitions on the native CPU runtime and supported GPU runtimes. Use one job
engine from both GUI and CLI. Establish feasibility of the required Apple/NVIDIA
CPU, wgpu, CUDA and native Metal paths in P0, using the same engine and kernels.
Retain AMD/Intel GPU and Windows builds; their execution is best effort under the
explicit testing policy below.
Later phases implement and validate the complete product. P0 proves selected
architectural mechanisms, not every future algorithm or product requirement.

## 2. Updated stack

| Piece | Selection | Purpose and reason |
|---|---|---|
| Application/build | Rust 2024, existing Rust 1.100.0 pin, Cargo workspace | Preserve scaffold lints, checked arithmetic, tool pins, lock discipline, and assurance gates. |
| UI | Slint 1.19 safe interpreter API, winit, FemtoVG-wgpu, accessibility | Declarative UI and native controls without generated unsafe Rust in first-party code. Shared wgpu 30 device/queue. Startup compiler/size cost and upstream Rowan trust boundary remain explicit; compiled Slint is an optional later optimization requiring generated-glue policy review. Software presentation remains the VM route. |
| Numerical kernels | CubeCL | One implementation of blur, resize, warp, pyramids, focus measures, reductions, FFT, fusion, and preview packing. No parallel WGSL/CUDA/MSL copies. |
| Compute runtimes | CubeCL CPU, wgpu, CUDA and native Metal qualified on required Apple/NVIDIA lanes; HIP retained as a best-effort execution path | Runtime changes compilation, resources, and dispatch, not algorithms. CPU uses LLVM JIT and its own worker pool. AMD/Intel GPU and Windows builds remain required. |
| Graphics/interop | wgpu | Shared device/queue, upload/copy, textures, resource limits, and Slint integration. Numerical kernels remain CubeCL. |
| Standard codecs | `image` with required features; direct `tiff` and `exr` APIs | Real u8/u16/float decode; reusable TIFF chunk buffers; full-precision export independent of previews. |
| RAW | Rawler 0.8.0, owned buffers, full default development | Real Bayer DNG → genuine RGB16 is proven without new application FFI. Freeze recommended crop, metadata WB/calibration and sRGB transfer. Proprietary-camera render/crop parity, including known Canon difference, is P2 work. |
| Color | Explicit encoding metadata; Little CMS through a safe Rust API when needed | Separate source precision, transfer functions, profiles, and display conversion. Do not silently change the processing color space during compatibility port. |
| Small host math | Checked Rust; `nalgebra` when a real solve needs it | Transform composition, small optimizer solves, confidence decisions, and orchestration are shared host code, not duplicate pixel backends. |
| Scheduling | Standard threads and bounded channels; one bounded two-worker Rawler pool | Worker owns compute; UI polls bounded state and latest preview. Rawler nested work uses one explicit local pool. No separate Rayon pixel-algorithm implementation. |
| Public interfaces | `clap`, `serde`, JSONL diagnostics | Shared job configuration, CLI diagnostics, settings, and truthful backend/geometry evidence. |

P0 spikes exercise critical safe APIs and available runtime profiles. The combined
codec/foundation lock is a metadata resolution proof, not a combined compile or
application assurance pass. Build and audit each actual integrated consumer when
introduced. License declarations are inventory only; there is no license allowlist
or license gate. Advisory, source, exact duplicate and expiry rules remain enforced.
An architectural dependency change reopens its affected P0 gate. Do not add Burn
solely to access CubeCL.

### Runtime portability

| Path | Hardware | Kernel source | Execution qualification | Display boundary |
|---|---|---|---|---|
| CubeCL CPU | ARM64/x86-64 CPU | Same definitions | Required on available macOS ARM64 and Linux x86-64 lanes; Windows execution best effort. | Bounded preview upload, or Slint software presentation. |
| CubeCL wgpu | Apple/Metal; NVIDIA/AMD/Intel Vulkan/D3D12 | Same definitions | Required on Apple Metal and NVIDIA Vulkan; AMD/Intel GPU and Windows execution best effort. | Shared Slint device and queue when possible. |
| CubeCL CUDA | NVIDIA | Same definitions | Required on the Linux RTX 3090 lane; Windows execution best effort. | Initially transfer only preview data to wgpu display. |
| CubeCL HIP | AMD/ROCm | Same definitions | Best effort; retain backend source and build profile. | Same conservative preview bridge. |
| CubeCL native Metal | Apple | Same definitions | Required on available Apple hardware. | Explicit bridge until native-resource interop is qualified. |

**wgpu does not execute CUDA or HIP.** NVIDIA via Vulkan/D3D12 is GPU support,
not CUDA execution. Native CPU/CUDA/HIP/Metal require their CubeCL runtimes in
that platform build. One source tree does not mean one binary across operating
systems, universally available instructions, or automatic support for every
accelerator. Retain runtime builds per OS/package; require execution evidence
only for the required testing lanes below.

Discover candidates, check features/memory, and complete a small real kernel
before qualifying a device. `auto` chooses a qualified candidate within budget;
explicit CPU/GPU choices remain reproducible. Add a runtime override for backend
qualification. A software adapter must never count as physical GPU execution.

Use portable f32 samples, checked indices, fixed tie-breaking, deterministic
reductions, and bounded workgroup storage. Do not require tensor cores, fixed
subgroup widths, float atomics, or f64 shader support. Specialize workgroup/tile
sizes without creating vendor-specific algorithms. Evaluate compensated or
fixed-order accumulation where precision needs it; do not silently use f16 or
fast-math. Codecs and small host solves need not execute on an accelerator.

### Testing scope — adopted user decision, 2026-11-17

AMD and Intel **GPUs**, plus Windows **execution/testing**, are best effort.
Their absence is not a P0 or release-test blocker. Build support remains in scope;
this is not permission to remove backends, OS targets or portable kernel code.
The available AMD Ryzen x86-64 **CPU** remains a required CPU test lane. The GPU
vendor decision does not remove x86-64 CPU testing.

| Lane | Build obligation | Runtime/test obligation |
|---|---|---|
| macOS ARM64 / Apple M4 Pro | Required native build/link. | Required CPU, wgpu Metal and native Metal qualification; native GUI through an isolated session/guest as applicable. |
| Linux x86-64 / NVIDIA RTX 3090 | Required native build/link. | Required CPU, wgpu Vulkan, native CUDA and isolated physical-GPU GUI qualification. |
| AMD GPUs: wgpu/HIP; Intel GPUs: wgpu | Retain and build the applicable backend profiles and vendor-neutral GPU code, using the required SDK/toolchain. | Best effort. No mandatory hardware procurement, runtime suite, parity or performance pass without hardware. |
| Windows x86-64 MSVC | Required target/profile build and link checks, including enabled CPU/wgpu/native-runtime code. | Best effort. Windows execution, GUI, driver and performance tests do not block P0 or release acceptance. |

A build check proves target/backend code compiles and links. It does not prove a
particular GPU driver can JIT or execute a kernel. Missing SDKs and failed or
unrun build checks remain visible build obligations; lack of test hardware must
not be disguised as a passing build. Runtime absence alone is not a build failure.

Keep build/static-check jobs separate from hardware execution jobs. Windows CI
must retain build/link checks without requiring physical-device tests. General
application tests remain mandatory on required macOS/Linux lanes. CI now separates hosted CPU assurance and Windows MSVC build/link from physical
GPU execution. Record actual build receipts below; workflow syntax or metadata
resolution alone is not a build pass.

Earlier reports retain their observed results and their then-current scope.
This policy supersedes statements that missing AMD/Intel GPU or Windows test
access alone blocks P0. It does not turn historical unrun tests into passes.

### Adopted P0 scope — user decision, 2026-11-17

P0 answers: **Can this stack use safe authored code, portable real kernels,
bounded resources, native accessible UI and an enforceable assurance design?**
It does not implement the full application before implementation starts.

| Earlier blocker | Required architectural proof in P0 | Required later acceptance |
|---|---|---|
| Linux GPU ASan | Actual CPU ASan/TSan on all owned host source; CPU/native source-universe coverage guard; mandatory real GPU tests. Preserve explicit GPU-sanitizer follow-up, with no suppression. | P9: investigate CubeCL/driver GPU ASan failure and final integrated regression. No GPU sanitizer pass is claimed before one exists. |
| RAW compatibility / T50 | Safe full Bayer DNG development and genuine RGB16 output; selected crop/color policy; real codec precision APIs. | P2: proprietary-camera crop/render parity, all file/error/orientation cases, and regenerate/hash-verify the frozen T50 recipe before matched comparisons. P10: complete unchanged numeric/quality protocol, with generated T50 labelled and other corpora real. |
| Complete numerical/UI/lifecycle/performance slices | Real blur and stable f32 reduction, u16 precision, shape/budget errors, resource reuse; safe shared Slint/wgpu handles; actual native input/pixels and responsive cancellable reusable worker. | P3–P5: all methods/registration; P6–P8: full job/UI/lifecycle; P9: packages/full suite; P10: final speed/memory/latency targets. Start timing the first complete pipeline. |

### T50 input policy — later user instruction

User has no suitable local T50 source and authorizes generation or temporary
open-source download, with cleanup. Use a deterministic **GENERATED** native
6000×4000, genuine RGB16 ZIP-TIFF 50-frame scale workload, with known focus
variation and independent full decode/hash verification. All fifty native frames
plus two truth images pass; every generated image is deleted. The retained
[recipe and receipts](../../qualification/t50-generated/README.md) reproduce the
exact samples; a later benchmark must add full-set preparation with disk preflight. Retain generator, seed,
ordered file/pixel hashes and cleanup receipt; remove generated images after
qualification. This changes T50's earlier real-photo-only input condition before
any matched candidate measurement. It does not change thresholds, case oracles,
precision or the real R50/D150/E9 requirements. Regenerate the exact set for later
benchmarks; do not present synthetic scale results as real-camera quality.

## 3. Ordered P0 gates and progress

P1 starts only after every **active architectural minimum** below passes. A row
marked **TRANSFERRED** has no remaining P0 execution gate; its full requirement
is retained in the named later phase. Keep old gate IDs for roadmap continuity.
Every PASS needs source/lock/device provenance, commands and a real result in the
[work log](p0-work-log.md). Historical failed runs remain failed.

| Gate, in order | Architectural minimum / progress | Later requirement or reopening condition |
|---|---|---|
| **P0.02 — access** | **PASS.** Apple CPU/GPU and Linux NVIDIA CPU/GPU reachable; private Linux Xvfb/AT-SPI can use physical GPU. Dedicated macOS guest SSH/Aqua route qualified. | P7/P9: current guest application AX/Automation identity, full workflows and packages. Never use host desktop input. New required hardware reopens access. |
| **P0.03 — foundation and builds** | **PASS.** Slint 1.19, CubeCL 0.11.4, wgpu 30.0.2; selected safe codec/RAW/color APIs; available native runtime builds. Combined graph resolves, metadata only. Actual Windows MSVC all-target build/link and Linux HIP build pass; receipts in work log. | P2: actual combined consumer build/audit; P9: packaged native dependencies/signing. A missing critical API or incompatible graph reopens P0. |
| **P0.04 — safe launch/resources** | **PASS.** Safe typed handles and checked launch under authored unsafe-forbid; nonzero/length/overflow validation, pre-runtime byte budgets, recoverable readback failures and repeated-job ownership tests. | P2/P3: dtype/decoder bounds and tile halos as consumers appear. New unsafe or unbounded ownership requires review. |
| **P0.05 — assurance** | **PASS for current production root library.** Strict CPU/native zero-uncovered-source comparison, mutation/branch/layout/Miri gates and actual Linux CPU ASan/TSan pass. Standalone UI/codec source has separate qualification, not full source-coverage/sanitizer approval. Physical GPU tests remain mandatory. | P9: external GPU-ASan investigation and complete integrated assurance. No ignored test, sanitizer suppression or production-source exclusion. |
| **P0.06 — portable execution** | **PASS.** Same u16 blur and new centered f32 variance/budget/reuse oracle pass ARM64/x86 CPU, wgpu Metal/Vulkan and native Metal/CUDA. HIP compiles/links without execution. | P3–P5: full primitive/method battery. GPU software adapters cannot satisfy physical execution. Runtime/backend changes reopen qualification. |
| **P0.07 — interop/bounds** | **PASS.** Real Slint/CubeCL shared device/queue, finite cached textures/planes/staging, bounded worker state and explicit presentation-copy boundary. Native close exits cleanly after explicit cancellation/shutdown. | P3/P7: production tile halos, decode/cache pressure, retained full-precision results. RGBA8 preview is not HDR or export precision. No zero-copy claim. |
| **P0.08 — initial I/O** | **PASS.** Full suite-generated Bayer DNG → genuine RGB16 validated independently; safe owned Rawler API, bounded two-worker pool. PNG16/TIFF16/f32 TIFF/EXR and ICC precision proofs retained. | P2: Canon crop/render parity, other cameras, orientation/alpha/grayscale, Unicode, file limits and atomic failed-export retention. No camera-specific crop constants. |
| **P0.09 — numerical risk** | **PASS minimum on all required profiles; see .06.** Exact full-range u16 blur and centered f32 row variance handle bright/flat/narrow/odd inputs; checked budget limits and warm repeated resource reuse. | P3/P5: FFT/resize/warp/pyramid/reconstruction and full per-case determinism. No f16/f64 requirement, vendor algorithm copy or relaxed tolerance. |
| **P0.10 — methods/slabs** | **TRANSFERRED to P3/P4.** No five-method/slab pass claimed. | Weighted/depth, Laplacian/Hybrid/exposure, auto parameters, 4+4+1 exposure tree, slabs and precision acceptance. |
| **P0.11 — registration/crop** | **TRANSFERRED to P5.** No registration pass claimed. | Known-transform oracles, all reference modes, ambiguous motion, breathing, confidence, crop and drift. One CubeCL production pipeline. |
| **P0.12 — worker mechanism** | **PASS.** Actual compute progresses while native Ping responds; cancel stops new work and reused worker starts/cancels second job. | P6: full job/device/error/fallback/queue semantics, generation IDs and source/export preservation. |
| **P0.13 — native UI mechanism** | **PASS.** Unchanged Linux native driver clicks actual Slint controls, reads state and validates changed pixels. Selected safe interpreter avoids generated first-party unsafe. | P7: every required control, menus, panels, viewer/collection/crop and GUI/CLI parity. Upstream Rowan runtime limitation remains explicit. |
| **P0.14 — complete lifecycle** | **TRANSFERRED to P8.** Minimal worker proof belongs to .12; no 64-frame scenario or 250 ms acceptance pass claimed. | P8: unchanged real 64-frame Laplacian/breathing pause/resume/cancel/reuse scenario. P10: 30-request latency campaign. No sleeps or business hooks. |
| **P0.15 — verification route** | **PASS minimum.** Private native Linux input plus physical GPU and task-owned guest access exist; source/contract/hash receipts retained. No full case-accounting pass claimed. | P9: matched packages, launch/argv/settings, guest export, physical-GPU full workflow. |
| **P0.16 — performance** | **TRANSFERRED to first complete pipeline/P10.** Targets and protocol frozen in .01. Historical single-job reference diagnostics retained; no speed/memory/latency improvement claimed. | Begin measurements at first working pipeline. Freeze T50 before comparisons. P10 requires equal quality, matched hardware/settings, warm/cold/RSS/GPU-memory/viewer statistics. |

### Assurance boundary and explicit limits

The current production root library has the complete coverage/sanitizer proof
below. Standalone UI and codec workspaces have their own strict lint, unit and
functional proofs; they do not inherit a root-library coverage or sanitizer pass.
Integrated production consumers must meet all owned-source gates when introduced.

The five current OS-neutral pure-host tests run under strict Miri on the
required canonical macOS CI lane. Linux's transitive tiny-xlib constructor calls
unsupported `dlopen` before test entry. No constructor, dependency feature or
test is disabled: Linux retains native layouts, branches and ASan/TSan.
[Source/lock/nightly-pinned host policy](p0-miri-host-boundary.md) fails closed
when owned source or its dependency graph changes. Linux local `all` prints its
unrun canonical-Miri obligation and cannot substitute for the required Mac result. Actual CPU JIT and GPU kernels run in
native integration tests. Linux CPU ASan/TSan exercises authored allocation,
launch, IR construction and readback. CPU-only and full native LLVM coverage
must both have zero uncovered lines/functions/regions/branches, and the native
production function/region universe must occur in CPU coverage. Unknown source
layouts and unclassified integration targets fail closed. No authored host path
may disappear by moving its test into a GPU target.

Neither ASan nor Miri proves GPU shaders, LLVM-generated machine code, proprietary
drivers or all third-party unsafe sound. The existing GPU-ASan device failure
and external loader leak stay visible for P9, with an executable diagnostic
profile. Slint interpreter adds its compiler/Rowan to runtime trusted dependencies. P0
accepts only repository-owned embedded UI source through the safe API, with this
trust limit explicit. Rowan parser runs at startup; its ARC strict-Miri failure
was reproduced in separate upstream probes, not an end-to-end UI Miri run. The
layout patch does not repair ARC. **P9 release condition:** explicitly review and
record continued interpreter/Rowan trust, or adopt a reviewed generated-bindings
boundary. Compiled glue can remove runtime parser exposure as well as startup
cost; it requires its own narrow policy review. This can stop release, not P1.

P0.01's unchanged [targets](performance-targets.json): at least **2×** warm
completed-job speed, at most **50%** peak memory, non-regressing cold median,
viewer P95 **≤33 ms**, pause/cancel acknowledgement **≤250 ms**. The
[measurement contract](p0-performance-targets.md) requires two warm-ups plus ten
fresh-process cached jobs, five fresh-cache jobs, 200 input-to-present events and
30 lifecycle requests per operation. These are engineering targets, not measured
achievements. Keep all input/output hashes, quality checks and failed runs.

### Stop and reopen rule

Stop only the affected work when a failed architectural minimum invalidates the
chosen stack, safe boundary, required runtime access or assurance design. Finish
independent useful work while resolving it. Do not label missing results PASS.
Dependency/policy exceptions remain exact, justified, expiring and enforced.
New first-party FFI still requires an explicit reviewed design; this plan grants
no unsafe or broad coverage exception.

A later acceptance failure stops that phase's exit; it does not retroactively
pretend P0 tested the full feature. A newly discovered architectural failure
reopens the relevant P0 gate. Finite prototypes do not guarantee unknown future
implementation details.

### Known later conditions that can stop a phase

| Condition | Required owner / timing |
|---|---|
| Expiring dependency reviews | Every milestone: five exact maintenance reviews expire 2026-12-17; duplicate reviews expire 2027-02-15. Replace or explicitly re-review before expiry; gates fail closed. |
| New source/dependency or OS-specific code | P1 onward: re-review pinned Miri host contract and source-universe guards. Current Mac Miri proof does not certify future Linux-only code. Separate CLI/core from UI dependencies in P1. |
| Full product acceptance | P2–P8: camera parity/files, complete methods/geometry and actual native lifecycle/permissions may stop their owning phase. P0 has not passed those cases. No artificial lifecycle delay. |
| Release trust/tool boundary | P9: record interpreter/Rowan runtime trust or adopt reviewed generated bindings; complete integrated host assurance and GPU-ASan investigation. No full UI Miri/GPU sanitizer pass is claimed. |
| Performance / generated-input preparation | First complete pipeline/P10: materialize exact T50 recipe with full-set disk preflight; match real R50/D150/E9 baselines and frozen numeric/quality targets. A failed target stops release. |

## 4. Architecture and ownership

P1 must separate the reusable compute/core crate from the UI dependency graph
before shipping a headless CLI. The P0 single-graph qualification exports Slint;
Linux's eager Xlib constructor is concrete evidence that the final CLI/kernel
crate must not link the UI graph. Keep each kernel definition once in that core.
Start other components as modules; extract further crates only for real build
boundaries: CLI must be window-system independent, kernels must
not depend on Slint, and native runtimes need target-scoped dependencies. Every
new crate inherits workspace lints.

| Boundary | Owns | Excludes |
|---|---|---|
| `job` / `config` | Immutable validated requests, limits, lifecycle, errors | GUI controls and adapter code |
| `image_io` | Metadata, decode, source validation, atomic export | Focus algorithms and authoritative preview pixels |
| `compute` / `kernels` | Typed resources, runtime-neutral launch, CubeCL kernels | Vendor copies of algorithm code |
| `registration` / `fusion` | One graph, transforms, confidence, fusers, slabs | Backend-dependent user semantics |
| `session` / `viewer` / `ui` | Sources/results, viewport, comparison, settings, Slint | Separate GUI processing/encoding implementation |

Provide `pancake-cli` and `pancake-gui`. Both use the same validated job engine.
GUI accepts positional input paths, as passed by the suite. Forced CPU CLI must
not create a window or initialize GPU compute. Product default can remain auto;
adapter pins deterministic CPU defaults for ordinary numerical cases.

Slint thread owns UI objects; worker owns compute submission. Immutable results
retain full precision or lossless backing, source identity, transforms, crop
origin, and settings. Selecting a new result cannot overwrite an old result.
Generation IDs prevent stale worker messages from replacing current state.

For wgpu, pass one setup's device/queue to Slint. Submit CubeCL producer before
copy/render consumer. Keep resources alive until completion; never recycle a
slice still used by pending work or display. Pool scratch/preview storage under
a byte budget. Native CUDA/HIP/CPU results may need explicit preview transfer.

Current Slint convenience texture import accepts RGBA8. Keep f32 processing and
export data separate and generate a display preview. This supports 16-bit export
but is not an HDR viewer. The experiment's 40-byte/pixel eager retention is not
the production cache design. Do not claim universal zero-copy.

## 5. Public boundaries

| Public boundary | Contract |
|---|---|
| CLI | Implement every descriptor flag: method; alignment/reference; kernel/levels; scale; high precision; slabs; auto/crop; depth/quality; GPU ID; reports; legacy rotation-scale. Reject invalid input with useful offending values. |
| `CONFORMANCE_EVENTS_PATH` | Optional JSONL sink. Emit `stack_complete`, `completed: true`, and actual numerical backend only after processing succeeds. Supply finite path-keyed reference-to-source 2x3 matrices and sequential transforms as required. |
| `CONFORMANCE_UI_SETTINGS_DIR` | Optional isolated profile: same namespace persists across restart; a new namespace starts clean. No precomputed workflow state. |
| Native AX | Real IDs/values from the same state used by controls and rendering. Counts, selection, image presence, and enablement reflect observed application state. |
| Files | Sources unchanged; failed jobs preserve existing outputs; publish only complete successfully encoded files. |

Drawing Slint on GPU is not GPU stacking evidence. Add runtime/device/timing IDs
as useful diagnostics, but keep aggregate backend reporting truthful. Decode
corruption is an input error, not a GPU recovery event. Never emit successful
completion or invented fallback for a failed decode.

## 6. Numerical invariants and traps

| Area | Required behavior |
|---|---|
| Precision | Preserve u8/u16 until conversion is needed, then f32. No 8-bit fusion intermediate. Weighted/depth/exposure identity gradients round-trip within one u16 code; PNG16 and TIFF16 decode identically. |
| Imports | RGB public order; defined grayscale/alpha handling; u16 grayscale; float TIFF at 0–1 and 0–255; EXR; mixed formats; Unicode/non-BMP paths; all EXIF orientations applied exactly once. |
| RAW | Decode actual 112x160 Bayer DNG, without embedded thumbnail, into genuine 16-bit RGB for all methods. This does not certify every proprietary camera RAW. |
| Dimensions | First-frame coordinate system; defined resize/pad policy for mixed dimensions and later higher-precision inputs. Odd-size pyramids reconstruct exact requested shape. |
| Flat/zero weights | Preserve constant color. Zero total focus weight selects first source in defined input order; never black, NaN, or divide-by-zero output. |
| Exposure | Four-frame batches with equal final batch weights, including one-frame remainder. Constant 4/9-frame oracle is mean of batch means, not global mean. Retain floating samples through the batch tree. |
| Determinism | Fixed ties and reduction order. Exposure repeat budget is max 1 u8 code, mean 0.001, at most 0.1% changed pixels. Other CPU repeat cases are pixel-exact. |
| Detail | Both tested focus planes retain at least 65% truth Laplacian variance; mean focus error at most 15 u8 values. Selecting one source cannot pass. |
| Geometry | Evidence maps reference to source, not inverse. Corner/center error at most 1 pixel; weighted first-reference blurred interior mean error at most 3 u8 values. |
| Ambiguous motion | Identity with zero confidence when evidence is insufficient. Never extrapolate motion into unsupported pairs. Repeated/nonrepeating slab cases require drift below 0.3 pixels and both focus planes. |
| Crop | Existing symmetric policy plus interpolation margin. Generated translation case requires matrix error at most 0.3 and specified bounded crop. Manual crop preserves an exact rectangular subset. |
| Failure | Any undecodable requested input fails entire job, identifies input, retains existing export, publishes no partial result/completion. Reject invalid dimensions before allocation/decompression. |

Use exact per-case parity limits in `cases/devices.py`; do not introduce one
relaxed global threshold. File rounding/quantization must be identical across
PNG/TIFF. Display quantization never controls export. New color-space or detail
enhancement behavior is separately validated after compatibility.

## 7. Ordered implementation milestones

Start every behavior change with a meaningful failing test. Exercise real public
boundaries through the adapter as soon as available. Retain all previously
passing cases in regression selections. Do not merge placeholders as passes.

| Phase | Implementation and regression exit evidence after P0 closure |
|---|---|
| **P0 — architecture qualification** | Close active P0.01–P0.17 minima above. Retain explicit transfers and unresolved third-party/tool limits. No full application or performance pass. |
| **P1 — CLI integration** | Shared validated config, help/errors, path/glob/deduplication, resource limits and real diagnostics. Build runnable release CLI; test arguments, help, and rejection paths. |
| **P2 — complete I/O** | Integrate selected codecs/Rawler/color into actual audited consumer. Complete DNG, precision, camera crop/render parity, orientation, grayscale/alpha, Unicode, malformed/oversized inputs, atomic export and source immutability. Regenerate/hash-verify approved generated T50 before matched scale comparisons; keep R50/D150/E9 real. |
| **P3 — primitives/basic fusion** | Implement shared resize/pyramids/focus/reduction resources, weighted average and depth map. Independent precision/shape/flat/tie oracles and applicable CPU/GPU device cases. |
| **P4 — all methods/slabs** | Complete Laplacian, Hybrid, exposure and slabs, auto parameters, reports and 4+4+1 batch semantics. Pass full numerical cases before optimization. Start equal-quality full-pipeline timing as soon as executable. |
| **P5 — registration/crop** | Implement/qualify transforms, multiscale FFT/refinement/confidence and all references, breathing, high precision, legacy switch, mixed dimensions, crop and slab drift. Known-transform independent oracles; no divergent OpenCV CPU path. |
| **P6 — devices/resilience** | Complete device qualification/selection, truthful telemetry, bounded queues, pause/cancel, stale messages and errors. Pass all 341 host cases. Keep genuine-driver-trigger test gap explicit. |
| **P7 — complete Slint workflow** | Build all controls, gestures, native panels, collections, retained results, comparison, export, viewport and crop on qualified safe UI/resource boundary. Native workflow and GUI/CLI parity; requalify guest AX identity. |
| **P8 — desktop/lifecycle** | Preferences, history/context, About/Lightroom, unsaved/error workflows. Run unchanged real 64-frame lifecycle with no artificial delay. All 34 VM-compatible GUI cases pass; physical GPU case remains separate. |
| **P9 — packages/required matrix** | Complete matching packages and native libraries, guest/physical-GPU GUI, integrated assurance, GPU-ASan investigation and explicit interpreter/Rowan release-boundary decision. Required Apple/NVIDIA runtime tests plus retained target/backend builds. AMD/Intel GPU and Windows runtime best effort; label unrun lanes unverified. |
| **P10 — performance/release** | Full frozen numeric/quality protocol (generated T50 scale; real R50/D150/E9): 2× warm speed, ≤50% memory, non-regressing cold median and viewer/lifecycle limits. Required assurance/build/package receipts and explicit best-effort labels. No target edits after seeing results. |

Use shared host math for small solves and control. RustFFT/OpenCV may be local
independent test oracles; they must not become separate production CPU algorithms.
Full feature acceptance remains mandatory in the phase above. An architectural
failure reopens P0; routine defects remain in their owning phase.

## 8. Native UI and lifecycle details

| Surface | Required observable state |
|---|---|
| Options | All five methods; off plus five alignment modes; exclusive selection; crop; advanced visibility; parameter values; true enablement. |
| Viewer | `has_image`, `zoom`, `center`, `compare`, `split`, `fit`; actual source index/result method; fixed coordinates across selection, including cropped retained results. |
| Collections | Real frame total, source selection, result count and per-result precision/transforms/crop; no counters inferred from requested actions. |
| Export | Native open/save/folder panels, format/depth, overwrite/error/cancel, batch naming, viewport export, repeatable pixel-preserving manual crop. |
| Desktop | Theme/device preferences, independent profiles, history/context actions, panels, About/version, readable Lightroom status/action state, unsaved close workflow. |

Use canonical semantic IDs or observed descriptor mappings. Native menu roles
and actual committed selection must satisfy the driver, not just custom labels.
Expose read-only viewer metadata from real state, not successful test actions.

Job states: Idle, Running, Pausing, Paused, Cancelling, Failed, Completed.
Acknowledge pause only after submitted work reaches a safe boundary; no new
batches then submit. Cancel stops new decode/compute, settles in-flight resources,
discards partial result, and restores reusable controls. Generation IDs reject
stale preview/completion messages. Keep full-precision retained results immutable.

Lifecycle fixture has 64 unique 1024x1536 paths. Perform normal per-frame decode,
alignment, and fusion. No artificial sleep/barrier/business hook may extend
runtime. Validate the complete native interaction window in P8 without slowing production
or synthesizing a pass. P0 proves the worker mechanism only; it does not claim
that this full workload already passes.

## 9. Verification lanes and full accounting

| Lane | Required evidence |
|---|---|
| Host numerical | 341 IDs: 46 CLI + 223 algorithms + 72 devices. Real CPU/physical GPU, outputs and diagnostics. |
| Dedicated macOS GUI VM | 34 interface IDs. Software presentation/native CubeCL CPU as needed; real native input, AX/screenshot traces, exports, completed host CLI parity. |
| Runtime/OS matrix | Required execution: macOS ARM64 CPU/Apple wgpu/native Metal; Linux x86-64 CPU/NVIDIA wgpu/native CUDA. AMD/Intel GPU and Windows execution are best effort. Builds remain required for retained targets/backend profiles; record build and execution status separately. |

An adapter-owned release verifier may combine IDs from lanes only with matching
source/descriptor/lock/contract provenance. Preserve every raw report/status;
reject missing, duplicate/conflicting verdicts or incomplete runs. Explain the
VM unsupported entry alongside physical evidence; never rewrite it as a pass or
cherry-pick successful retries. Qualify complete accounting in P9; P0 retains raw provenance receipts only. Unrun
required hardware remains unverified and cannot satisfy its P0 execution gate.
Untested best-effort hardware remains labelled unverified without blocking that
gate; its retained build obligations still apply.

Provision matching native binaries/libraries inside the guest before tests.
Workspace sync does not prove ignored `target/` artifacts were copied. Build in
an explicit guest preparation step or stage a matching package under the adapter;
never compile inside a case's process timeout. Validate actual application package preparation/launch in P9 before final GUI
acceptance. P0 proves access and the minimal isolated Linux native route. Virtual GPU is unnecessary
for software UI presentation and native CPU processing.

Target-scope native dependencies and runtime build profiles. Review compatibility
with existing all-features/feature-powerset gates during P0.03/.05. Apply the
explicit best-effort testing policy above; do not silently remove build targets,
omit required execution lanes, weaken checks or add broad coverage/dependency
exclusions. Repeat validation whenever the qualified build graph changes.

## 10. Commands and release evidence

From repository root, after milestones create the real executable targets:

On a provisioned physical-GPU Linux host, the unchanged runner creates a private
desktop automatically; do not pass `--gui-vm`:

The alternative below is for a separately provisioned isolated native macOS test
session only; never execute it against the user's active desktop:

During P0 and after each milestone: run `bash scripts/check.sh all` inside
`pancake/`, accrued external cases, and tests for executable adapter code if
introduced. Use unique
output directories. Record platform/remote checks not run. Never regenerate
baselines to hide failures, relax tolerances/timeouts, or retry defects away.

The recurring local gate order is `fast`, `test`, `coverage`, `deps`, `mutants`,
`nightly`, `branches`, `guardrails`; CI additionally retains platform build/link checks and required-lane native
execution plus Linux address/thread sanitizer checks. Windows execution/testing
is best effort; its build checks remain required. P0.05 must establish that these gates
work for the real chosen architecture. Later runs verify continued compliance;
they are not first-time feasibility experiments.

Measure cold JIT/setup separately from warm complete jobs; decode, registration,
fusion, export, preview, and input-to-present separately. Time completed GPU work,
not dispatch. Bound scratch, decoded lookahead, and preview/result caches by
bytes; use lossless spill when needed. Spatial tiling needs correct halos and
is distinct from frame slabbing. Fix targets in P0.01 and prove performance and
resource mechanisms in P0.04/.07. Begin complete-job measurement at the first
working pipeline; P10 owns final acceptance on the integrated application. Compare generated T50 scale inputs and real R50/D150/E9 stacks with peak memory;
do not double-count Apple unified memory as independent CPU/GPU physical pools.

## 11. Sources

Primary upstream documentation:

- [CubeCL](https://github.com/tracel-ai/cubecl): runtimes, shared kernels,
  capability differences, JIT/cache, and evolving APIs.
- [CubeCL CPU](https://github.com/tracel-ai/cubecl/blob/main/crates/cubecl-cpu/README.md):
  LLVM compilation and worker-pool execution.
- [Slint features](https://docs.slint.dev/latest/docs/rust/slint/docs/cargo_features/):
  winit, wgpu/software rendering, version-specific interop.
- [Slint wgpu integration](https://docs.slint.dev/latest/docs/rust/slint/wgpu_30/):
  shared device/queue and external texture import.
