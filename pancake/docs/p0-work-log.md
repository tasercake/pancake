# P0 work log and evidence register

This document consolidates earlier P0 work and the closure work. Historical
failures are retained, with their resolutions or later acceptance owners.
The plan's adopted smaller P0 supersedes the earlier, larger gate scope; it does
not change an observed failure into a pass. The machine-readable
[`p0-work-log-evidence.json`](p0-work-log-evidence.json) indexes retained source
and evidence by path, byte count and SHA-256. Original photos and large compiler
caches remain ignored; their manifests identify retained locations and hashes.

## 1. Current verdict and ownership

| Item | Verdict / evidence | Owner after P0 |
|---|---|---|
| Dependency and safe API choices | Slint 1.19.0, CubeCL 0.11.4, wgpu 30.0.2; safe TensorHandle kernels; Rawler 0.8.0 plus image/tiff/exr/lcms2. License gate removed. Exact expiring dependency reviews enforced. | Actual integrated consumers must build and pass policy; new architectural choices reopen P0. |
| Numerical / UI proofs | Blur, stable f32 variance, checked budgets and repeated ownership; native Slint controls, actual pixels and cancellable reusable worker. Final required-profile receipts below. | Full numerical methods P3–P5; job/UI/lifecycle P6–P8. |
| Assurance / builds | Local aggregate, Linux CPU ASan/TSan and Windows build receipts pass; hosted CI setup corrections remain under validation. GPU-ASan failure remains an explicit external-runtime follow-up. | Complete application assurance and packages P9. |
| Full product / performance | **Not run.** No Pancake CLI/GUI application or complete stacking job yet. Targets frozen; three real corpora retained; generated T50 qualification/cleanup is in progress after explicit user authorization. | P1–P10, with T50 before matched comparisons. |

## 2. Evidence chronology

Commit IDs identify preserved historical source, even where later integration
uses a cherry-pick with a different commit ID. They are not substitutes for the
source/lock hashes in each receipt.

| Work | Main / source commits | Retained evidence |
|---|---|---|
| Initial plan and front-loaded gate design | `8fcbc52` | Prior plan history; current adopted gate ledger in owning plan. |
| Hardware and dependency candidate audits | `19689ed`, `5370eff`, `65b185f`, `d4b0c96`, `de8940c` | [Hardware lanes](p0-hardware-test-lanes.md), [runtime audit](p0-runtime-feasibility.md), runtime evidence and reconstruction patches. |
| Targets / corpus / preliminary baselines | `b3c3a9a`, `39bb1de`, `19b7f9e`, `e40788d` | [Target contract](p0-performance-targets.md), [baseline report](p0-performance-baseline.md), adapter corpus/RAW/baseline manifests and compressed raw logs. |
| Dependency replacements and enforced exceptions | `936da64`, `202cad0`, `fa2060d`, `7feb615` | [Patch review](p0-upstream-patch-review.md), [policy](dependency-policy.md), vendor source manifest and executable regression tests. |
| Safe blur and six runtime paths | `f3fcd45`, `5f85899` | [Kernel qualification](p0-kernel-qualification.md), kernel evidence, six-path runtime manifest and retained logs/source probes. |
| Codec / RAW sensor / color proof | `48ac5f7` | [Codec qualification](p0-codec-qualification.md), codec evidence and independent Rust/Python oracle source. |
| Guest access and original GPU sanitizer investigation | `2062df8`, `4fa1073` | [Guest evidence](p0-guest-access-qualification.md), [sanitizer investigation](p0-sanitizer-investigation.md), bounded diagnostic source/logs. |
| Best-effort testing scope | `bb0307b` | Plan history; runtime absence no longer blocks AMD/Intel GPU or Windows. Build obligations retained. |
| Bounded variance/resources and assurance design | `572e601`, `44dc3d6`, `65185d9` | Production source, shared independent numerical oracles, [host assurance boundary](p0-host-assurance.md), source-universe and target-selection regression tests. |
| Complete generated DNG development | `6b54aff` (source `7e6fafd`) | [RAW development qualification](p0-raw-development-qualification.md), detailed evidence, unchanged fixture/validator oracle and combined metadata lock. |

The earlier [unblock report](p0-unblock-report.md) records the larger P0 scope
and then-open conditions. Its status is historical. This work log and the current
plan record explicit transfers and final closure status.

## 3. Contract drift and independent acceptance

| Observation | Result |
|---|---|
| Material behavior change | Recovery GUI/CLI comparison now uses actual completed backend. Existing parity limits unchanged. Linux private desktop can execute physical-GPU GUI case. |
| Harness self-tests | macOS: 307 passed, four explained skips. Linux: 309 passed, two live-GUI smoke skips. Ruff passed. These are harness tests, **not Pancake case passes**. |

## 4. Historical Slint/CubeCL viewer experiment

The earlier experiment is archived as small immutable documents under
[`p0-historical-viewer-evidence/`](p0-historical-viewer-evidence/README.md).
`provenance.json` records original source/manifest/lock hashes at `0c83870`.
Raw benchmark samples, rendered PNGs and GUI memory samples are archived under
`p0-historical-viewer-evidence/raw/` with exact hashes. Numerical performance
samples identify `6b9db62`; later callback/smoke changes
must not be silently attributed to that measurement revision.

| Check | Observed result / limit |
|---|---|
| Architecture | Real CubeCL exposure/packing → wgpu buffer-to-texture copy → Slint same-device/queue texture. No application readback in normal viewer update. Uses unsafe raw BufferArg and eager resources; **not application-policy qualification**. |
| Tests / rendering | Four prototype tests; real rendered before/after snapshots after callbacks; failed smoke now propagates exit 1 (`e52df36`). Native CUA attachment timed out. No native input/accessibility or event-to-present latency pass. |
| Measurement protocol | Five serial release processes, 30 iterations per size/process. 257×193, 1920×1080 and 6000×4000 generated images. 150 completed compute and 150 completed copy samples per size, CPU wall clock with GPU waits. |
| Warm 24 MP preview | Compute median 2.224 ms, P95 2.460 ms; copy median 0.581 ms, P95 3.530 ms. Runtime init median 10.32425 ms, P95 11.7565 ms. CPU oracle max preview error zero. These are preview operations, not focus-stack speed or FPS. |
| Memory | 40 logical bytes/pixel eager storage; two 24 MP sources about 1.921 GB before runtime overhead. Whole-sweep RSS about 3.586–3.587 GB, physical footprint about 4.912 GB. Cannot scale this retention to 50 sources or add unified CPU/GPU estimates as independent pools. |

Original experiment lock mixed top-level CubeCL 0.11.3 with permitted internal
0.11.4. It proves only its exact snapshot. Current production selection is
CubeCL 0.11.4. Its RGBA8 display import proves neither HDR nor full-precision
export. Retained benchmark hashes and full limits are in the archived audit.

## 5. Dependency decisions, patches and policy

### Candidate audit

Candidate A used the viewer-derived broader graph: 41 duplicate groups, six
unmaintained notices, one missing-license error and 653 license-allowlist
diagnostics. Minimal required candidate B retained CPU/wgpu, accessibility,
Wayland and software presentation: 30 groups, six notices and 561 license
diagnostics; missing-license issue disappeared. Compatible released refresh C
still had 30 groups and six notices. Exact manifests, paths, counts and lock
reconstruction patches are retained in the runtime audit/evidence. Metadata
resolution alone was never called compilation or execution.

### Applied fixes

| Change | Proof and policy boundary |
|---|---|
| Five CubeCL manifests: `paste` → renamed `pastey =0.2.3` | Original API import names retained. Exact archives and initial 390-file comparison; only seven initial source files changed including spin_on. Upstream core 12, C++ 20 and stdlib 50 unit tests pass; SPIR-V builds with zero unit tests. Two trybuild groups retain two rejected and three accepted programs. |
| `spin_on`: retired `pin-utils` → standard stack pinning | Two upstream unit tests, original doctest and immovable-future regression pass natively and strict Miri. No new unsafe or allocation. |
| CubeCL macro `expand_only` | Narrow three-line patch removes unused host DSL stub while retaining safe checked launcher. Three compiler contracts pass. No owned source excluded from coverage. |
| Rowan `repr(u32)` layout | Fixes layout-seed-42 static assertion. Four upstream tests pass under seeds 1 and 42. Original and patched ARC both fail strict Miri at `arc.rs:264`; unsafe ARC source unchanged. No ARC soundness repair claimed. |
| Duplicate/advisory policies | Initial integrated graph: 787 packages, 32 exact cargo-deny older-version skips across 29 groups; Clippy 58 names/123 exact versions. Parent versions/requirements and protected single-version GPU/window families checked. Expiry fails closed. |

License allowlist **and invocation of license gate** were removed at user request.
Declarations remain informational inventory. Advisories/bans/sources retain
warnings-as-errors. Dependency exceptions cannot hide new security advisories.

Five exact **maintenance-only** reviews expire **2026-12-17**:
`RUSTSEC-2026-0192` ttf-parser 0.25.1; `0370` zvariant 5.15.1; `0371`
zbus_names 4.3.5; `0372` zvariant_derive 5.15.1; `0543` chrono 0.4.45 (all
prefixed `RUSTSEC-2026-`). Font parsing and D-Bus/AT-SPI remain real runtime
exposures; Chrono covers UI date/time and exact dormant Turso parents. Protected
families cannot receive duplicate exceptions. Duplicate reviews expire
**2027-02-15**. [Enforced policy](dependency-policy.md) records exact paths,
owners, expiry and removal conditions; no tree-wide skip or advisory downgrade.

Original unit/macro/Miri logs are archived losslessly under
`p0-historical-upstream-evidence/`, with original sizes and SHA-256 values.

Early cargo-machete printed six metadata errors but exited zero on the empty
scaffold. This was not accepted as a clean integrated pass. Final wrapper checks
actual first-party manifests and fails on those diagnostics. Real regression
tests cover malformed metadata and nested vendor layout. Initial equivalent tail
guard mutant was removed by deleting the redundant guard; checked kernel launch
still bounds access. No surviving-mutant exception was introduced.

## 6. Portable kernel and resource evidence

### Earlier safe blur

The same u16 `[1,2,1]/4` horizontal definition widens to u32 storage and uses safe
TensorHandle resources/checked launch. Independent exact values test 5×3,
full-range u16, clamped borders, width one and width-257 workgroup tails.

| Runtime | Actual device | Result |
|---|---|---|
| CPU LLVM, ARM64 | Apple M4 Pro | Exact blur values pass. |
| wgpu Metal | Apple M4 Pro integrated GPU | Exact values pass; software fallback rejected. |
| Native Metal | Apple M4 Pro, `msl_Apple M4 Pro` | Same values, borders and tails pass. |
| CPU LLVM, x86-64 | AMD Ryzen 9 7950X3D, `cpu_x86_64_load-512_f16-chain` | Same exact values pass. Runtime name is not evidence of f16 sample conversion. |
| wgpu Vulkan | NVIDIA RTX 3090 discrete GPU | Same exact values pass; software fallback rejected. |
| Native CUDA | RTX 3090, `ptx_sm86`, CUDA 12.5, driver 580.178.04 | Same exact values pass. Negative missing-runtime test rejects before enabling CUDA feature. |

### New bounded reduction and reuse

Meaningful failing tests first requested absent bounded blur/variance APIs.
Production default is a **512 MiB logical working-data budget**. Blur checks a
conservative 20 bytes/input pixel; row variance checks 22. Shape, overflow,
sample count and budget checks occur before runtime creation/allocation. A
deliberately unavailable HIP device proves budget rejection precedes setup.
Exact 100/110-byte cases pass; 99/109-byte cases reject.

Centered two-pass f32 row variance retains small differences on bright u16 rows:
independent expectations `[8,0,2]`, bright four-sample variance 1.25, one-pixel
rows zero and 257-wide bright flat rows exactly zero. One kernel definition
serves all backends; no f16/f64 requirement or vendor numerical copy.

Repeated 1025×257 images (263,425 pixels), under an 8 MiB logical request cap,
warm once then run eight blur/variance jobs. Apple CPU live allocation returns
to zero; reserved pool remains 12,582,912 bytes. Apple GPU live six allocations /
384 bytes and reserved 13,748,992 bytes remain unchanged. Pool reserve is not
the request budget, process RSS or GPU peak. Full tiling/halos and integrated
decoder/result-cache pressure remain P2/P3/P7 acceptance.

Strict source coverage initially found one missing region: public variance's
shape-error `?` return. Added shared public invalid-shape oracle (`65185d9`)
instead of excluding the region. One parallel coverage run reused an
instrumented cache and reported another worktree's source paths; that run is
invalid evidence. Final aggregate owns instrumented collection exclusively.

New native runtime receipt: [p0-numerical-qualification.md](p0-numerical-qualification.md)
and [exact evidence](p0-numerical-evidence/evidence.json). Native Metal and CUDA
both complete the unchanged variance/budget/reuse oracle. Metal live allocations
return to zero; CUDA retains eight / 128 bytes, unchanged after eight rounds.
Both reserve 12 MiB before/after. Linux HIP `--no-run` compiles/links an ELF
without ROCm/hipconfig or missing dynamic dependencies. No HIP execution claim.
New Vulkan reduction also passes on physical RTX 3090: eight live allocations /
512 bytes and 13,778,496 reserved bytes remain unchanged. Its numerical and
separate original blur logs are retained with exact adapter identity.

## 7. Assurance: observed failures and adopted boundary

### Earlier integrated local result

Historical `bash scripts/check.sh all` passed on the safe blur snapshot:
six debug/release tests, 62/62 lines, 7/7 functions, 66/66 regions, 8/8 branches,
35/35 mutations caught, three strict host Miri tests, layout seeds 1/42 and ten
real rejecting/accepting guardrail outcomes. These counts belong to that source,
not the larger final numerical implementation.

Miri cannot execute native LLVM JIT/GPU FFI (`mach_host_self` failure). Pure owned
host library contracts therefore run under strict provenance/symbolic alignment;
native integration remains mandatory. This is not evidence that Miri checked
GPU machine code, drivers, LLVM or all third-party unsafe.

### Original Linux GPU-ASan investigation

| Run | Observed outcome |
|---|---|
| Original all-target ASan | Three host and two CPU/shape tests pass; GPU device acquisition fails. Leak report: 440 bytes / three allocations. |
| NVIDIA ICD only | Same CubeCL setup failure. Removing Mesa route removes 256-byte contribution; 184-byte external Vulkan/EGL-path leak remains. |
| Minimal C native / ASan | Native creates device; ASan also creates device but reports same 184-byte loader leak. Thus loader leak is independently reproducible outside Rust. |
| Same safe Rust ASan diagnostic binary | Minimal wgpu device request succeeds; CubeCL descriptor/setup route fails. Features, limits, memory hints and experimental flags were not isolated. No claim of an upstream root-cause fix. |
| EGL vendor restriction / slow unwinding | Recursive `AddressSanitizer:DEADLYSIGNAL` terminated; 465,346,740-byte repetitive log reduced to 20 retained head lines with original size recorded. Three bounded attempts stopped; no blanket exception. |

Bounded source, native-vs-instrumented comparisons, build IDs, tool versions and
gzipped logs are in [sanitizer evidence](p0-sanitizer-evidence/evidence.json).
Old tiny TSan preflight had a standard-library ABI mismatch; that is not a race
or successful runtime test. Final TSan rebuilds standard library correctly.

### Enforced P0 host boundary

[`p0-host-assurance.md`](p0-host-assurance.md) owns exact design. Current production root library plus
every `cpu_*` integration target run under Linux ASan/TSan. Same production
allocation, launch, IR construction and readback execute through CubeCL CPU,
without scalar replacement. Full native GPU targets remain mandatory under
`test`, `gpu` and `all`. Unknown integration target names fail classification.

CPU-only and native coverage each require zero uncovered owned source, including
build.rs when present. Function/region identity comparison requires all native
production source to occur in CPU collection; unknown production layouts fail
closed. Standalone UI/codec workspaces are separate test-only qualification, without
root-library sanitizer/full-source-coverage approval. No owned production root
source exclusion, ignored test, sanitizer suppression or
GPU-only host path loophole was added. Nine focused guards cover these rules.
`sanitizer-gpu` preserves explicit P9 diagnostic route.

Container TSan initially exits 66 before tests: it cannot disable ASLR under
default seccomp. An exec-scoped privileged attempt has the same result and is
not a passing test. An isolated CPU-only container now passes all nine tests using the same three
TSan binaries and five runtime libraries. It runs as UID 65534 with no network,
GPU or capabilities; read-only staged inputs, no-new-privileges and seccomp
unconfined let TSan change its own personality. Full provenance is retained in
the final assurance receipt. No host
sysctl, ASLR policy or driver change is used.

Final Linux [receipt](p0-host-assurance-evidence/receipt.json) and bounded logs
retain both actual nine-test ASan and nine-test TSan passes, source/standard-library
identity and exact closure hashes. This proof applies to the current production
root library, not all standalone qualification source.

## 8. RAW, codec, color and precision evidence

| Proof | Actual result |
|---|---|
| Real Canon CR3 sensor | frame_000 SHA-256 `4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca`; Rawler owned-buffer decode 5376×3574 u16 = all 19,213,824 LibRaw samples bit-exact. CRX sensor is 14-bit; u16 storage is not a 16-bit ADC claim. |
| Camera visible area | Same full sensor; LibRaw visible origin 258/162, size 5118×3412; file/Rawler recommended origin 272/172, size 5088×3392. No sensor loss or camera hardcode. Full render/crop parity remains P2. |
| PNG16 / ZIP TIFF16 | Exact low-bit samples including 1,255,256,257,258,32769. Independent OpenCV/tifffile checks; TIFF ADOBE_DEFLATE tag. |
| Float TIFF / EXR | Independent bit identity for negative, >1, 65504 and `0x3f000001` f32 samples. One-ULP negative control first failed; no relaxed tolerance or f16 intermediate. |
| ICC | Serialize/reload initially changed samples +8/−1. Final validated byte-identical profiles bypass conversion over all 65,536 u16 values. Real sRGB→linear-sRGB safe typed conversion matches independent system LCMS across probe samples. |

LibRaw binding alternatives were reviewed, not assumed safe: libraw-rs allocation
and image-slice guards; rsraw retained-buffer lifetime/mutable native ownership;
rawlib handwritten ABI/macOS native linkage; older libraw package's native
requirements. No memory-unsafe demonstration was executed. Selection avoids
introducing new application FFI. Rawler itself and codec dependencies contain
third-party unsafe; authored Rust remains unsafe-forbid.

### Full unchanged Bayer DNG development

Output shape `[112,160,3]`, uint16, range **65535** (required 10000), **28,206**
unique values, **52,723** samples not divisible by 257. Independent promoted-8-bit
negative control is rejected with `Image lost 16-bit precision`. TIFF SHA-256
`1b7afc3b05d0cde5ee61b4ce2de7c3363cce19a923bc7e68b83dcaaa10ebda6c`;
pixel SHA-256 `523d4b0e2c883f6661b82d6b269c832e1340001f13040abedbac9142ac14f396`.
Fixture/validator and source input hashes are unchanged.

One process-wide two-worker local Rayon pool owns all RAW decode/develop work;
observed names `pancake-raw-0` / `pancake-raw-1`. No global pool/environment
mutation or per-image pool. Three Rust tests, strict lints and both independent
oracle environments pass. This bounds RAW workers, not every native/driver
service thread or whole-process memory.

Combined foundation/codec metadata resolves 839 packages, one wgpu 30.0.2 family,
CubeCL 0.11.4, Slint 1.19.0 and eight reviewed vendor patches; no paste/pin-utils.
Committed combined manifest/lock are **metadata-only**, not a combined build,
decode→CubeCL→export pipeline, policy approval or shipping application coverage.
P2 must integrate and audit the real consumer graph.

## 9. Hardware and non-interfering execution

| Route | Evidence / limits |
|---|---|
| Local Apple | M4 Pro, 14 CPU / 20 GPU cores, 48 GiB unified memory, macOS 26.2 build 25C56. Rust 1.100.0; pinned nightly 2026-11-15. SDK metadata reports 26.4. Actual runtime reports distinguish native Metal and wgpu Metal. |
| Linux physical GPU | Existing SSH alias `krishna-desktop`; Ryzen 9 7950X3D, RTX 3090 24 GB, driver 580.178.04, CUDA 12.5, host Linux 6.8. Task-owned Debian 12 container from existing Node 22 image; no host package/driver changes. |

Earlier `orb list` unexpectedly attempted VM startup and timed out; it was not a
passive inventory success. Subsequent work uses known existing SSH/container
routes. Container lifecycle is coordinated between worktrees. Task-only derived
compiler objects may be reclaimed to preserve 15 GiB free disk; source, photos,
evidence and active caches are preserved. No personal photo library was scanned.

## 10. Native Slint, interop and worker proof

Final safe UI qualification and detailed receipts are retained in
[p0-ui-qualification.md](p0-ui-qualification.md) and
[p0-ui-evidence/manifest.json](p0-ui-evidence/manifest.json). The actual native Linux oracle observes Slint controls, uses real
AT-SPI/XTest input, checks rendered samples and exercises a real compute worker.
It does not add Pancake business operations to the driver.

Selected UI boundary is Slint interpreter `ComponentInstance`, not generated
unsafe bindings. Minimal compiled `slint::slint!` failed E0453 because generated
`allow(unsafe_code)` conflicts with authored `forbid`. Safe interpreter preserves
first-party policy and same Slint/wgpu renderer. Startup compiler overhead and
Rowan runtime trust are explicit costs; compiled UI remains a separately reviewed
future optimization, not an unrecorded safety exception.

Slint's Rowan parser executes during runtime UI compilation. Separate original
and layout-patched upstream probes reproduce the ARC strict-Miri error at
`arc.rs:264`. This is not an end-to-end UI Miri run or proof that the embedded
component triggers that same operation. The error reports an invalid Stacked
Borrows retag; that model is experimental. Native success is not a proof of
soundness. No upstream ARC
rewrite, Miri exclusion or claim of full UI Miri coverage was introduced. P0
records trusted repository-owned embedded-source use only. P9 must explicitly
review/record the release interpreter/Rowan boundary or adopt reviewed generated
bindings; this is a visible release condition, not a deferred P0 proof.

Resource bounds: two 512×384 RGBA textures (786,432 bytes each), two
2048×1536 u16 planes (12,582,912 bytes total), RGBA staging observed peak one
786,432-byte buffer with structural maximum three (producer/queued/presenter),
one command slot and one pending preview.
Root blur separately uses a 25,165,824-byte logical u32 input/output-buffer bound;
Its complete conservative working-data admission includes other storage too.
Actual CubeCL blur runs on worker; explicit CPU packing/upload to shared texture is conservative bridge,
not zero-copy. These counts are not total RSS or GPU peak. Full caches, retained
results, image tiling and 64-frame lifecycle remain P3/P7/P8 acceptance.

Final one-run compile+instantiate observation: 252 ms. Final debug harness
binary 1,001,110,304 bytes includes LLVM, Slint interpreter and debug symbols;
there is no release or interpreter-only size-delta claim. These values do not
establish final cold-start or viewer-latency targets.

## 11. Real corpora, baselines and frozen performance targets

| Corpus | Frozen evidence |
|---|---|
| D150 | 150 distinct optical-focus amber-mite frames, 2304×1728 original JPEG8, selected from 152. Source/archive/file hashes and CC BY4.0 evidence retained. |
| E9 | Nine distinct Balloons exposure brackets, 512×339 original PNG8; actual exposure domain, not focus frames or repeated sources. Rights evidence retained. |
| R50 | First 50 consecutive pinned Canon CR3 files, original hashes all distinct; visible 5118×3412, sensor 14-bit stored u16. Full LibRaw decode of all 50 complete. Below T50 resolution. |
| T50 | Earlier real-photo-only slot is explicitly superseded by later user permission to generate temporary data. Deterministic 50-frame native 6000×4000 genuine RGB16 ZIP-TIFF scale qualification/cleanup is in progress. Retain recipe/hashes; no real-camera quality or full-job timing claim. |
| Acquisition | Initial unique payload 214,404,651 bytes plus R50 442,098,996 = 656,503,647 bytes, below unchanged 2,147,483,648-byte cap. D01 disabled/untouched. D05 acquired but unused in benchmarks; no photos redistributed. |

Original data stay in retained ignored `p0-performance` and `p0-unblock` worktrees;
metadata integration does not copy them. Adapter manifests give precise file
paths, order, source pins, rights, conversion recipes and hashes. No original
photograph is committed or edited. Full source R50 dataset is not downloaded.

Frozen [performance-targets.json](performance-targets.json) and
[measurement rules](p0-performance-targets.md): at least 2× warm full-job speed,
≤50% peak memory, non-regressing cold median, input-to-present P95≤33ms and
pause/cancel acknowledgement≤250ms. Two warm-ups plus 10 fresh-process cached
jobs; five fresh-cache jobs; 200 actual viewer events; 30 requests/operation.
Same hardware/input/settings/output-quality, completed exports, actual backend,
source/lock/driver/cache provenance. Apple unified memory must not be double
counted. Complete-job comparisons start with first working pipeline; P10 owns
final acceptance. P0 does not pass these targets by measuring one preview kernel.

Two check-infrastructure defects were found during final execution. Existing
GitHub run [44054403642](https://github.com/tasercake/focus-stacking/actions/runs/44054403642)
failed before checks because install-action treated Go actionlint as a Rust
crate (`actionlint is not found`). Workflow now installs the same pinned
v1.7.12 from its Go module; no version/gate changed. Native macOS Bash 3.2 then
rejected an empty `assurance_args` array under `set -u` after all 79 mutations
and five strict Miri tests had passed. This is a script failure, not a failing
kernel/layout test; final command must rerun after compatible argument handling.

## 12. Final execution receipts and closure

The complete local aggregate passes at source snapshot
`85ae9433f35220761d390e5aa4a90eb3363114c3` through the unmodified pre-push hook.
[Exact receipt](p0-close-evidence/local-aggregate.json) and complete gzip logs,
four raw coverage reports and mutation outcomes are retained. Both CPU/native
reports name this worktree's root production source: **139/139 lines, 14/14
functions, 143/143 regions, 14/14 branches**; **79/79** mutations caught, zero
missed/unviable/timeouts. Five strict Miri tests, 11 native debug/release tests,
all 11 tests under both layout seeds and ten guardrail outcomes pass.

[CI run 44090076402](https://github.com/tasercake/focus-stacking/actions/runs/44090076402)
checks the same application/lock/script/workflow snapshot on the isolated
`codex/p0-close` branch. Windows MSVC all-target build/link **passes** (job `134062600663`), with
[full log/receipt](p0-close-evidence/windows-msvc-build.json); it never runs
Windows device/UI tests. Other jobs in that first run fail runner setup: missing
fontconfig headers, absent Go on macOS and Go-built actionlint's `v1.7.12`
version field. These are not passing CI results. Shared native-package setup,
checksum-pinned release actionlint and hosted CPU mutation routing are being fixed. Subsequent evidence-only integration must retain exact
production-source/lock equivalence rather than pretending CI tested new code.

Reproduction entry points, from repository root:

```sh
cd pancake
bash scripts/check.sh all
bash scripts/check.sh gpu
# In qualified Linux x86-64 environment:
bash scripts/check.sh sanitizer address
bash scripts/check.sh sanitizer thread
# Explicit unresolved GPU-ASan diagnostic profile:
bash scripts/check.sh sanitizer-gpu address
```

Standalone proof commands and exact environments are in
[`qualification/p0-codecs/README.md`](../../qualification/p0-codecs/README.md),
host assurance notes and retained runtime manifests. Do not run native input on
the user's active desktop. Do not run performance measurements concurrently
with compilation, instrumentation or other GPU work.
