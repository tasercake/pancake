# Pancake implementation plan

| Roadmap phase | Passed | Host evidence |
|---|---:|---|
| P5 motion, crop and registration | 64/64 | [P5 replay](p5-registration.md) |

The release uses the reviewed Slint interpreter with repository-owned
source. The known upstream Rowan strict-Miri finding remains in the
assurance record. [Interpreter release decision](p9-slint-boundary.md).

The matching optional Linux CUDA and HIP release profiles both linked. The
CUDA build passed four frozen CLI cases, with two CPU completions and two
native CUDA completions on the RTX 3090. The HIP result is a build/link
receipt. Apple execution and signed packaging, as well as the Windows
native build, remain open.
[Native Linux profile evidence](p9-linux-native-evidence/12f2151/README.md).
[Windows build prerequisite](p9-windows-build-evidence/12f2151/README.md).

At the packaged `12f2151` CLI, the aligned real E9 comparison completed 17
launches per application in each CPU and physical NVIDIA lane. CPU warm,
cold and peak RSS ratios against the reference were 0.4604, 0.0628 and 0.3188,
meeting their numeric limits. Vulkan/OpenCL warm and RSS ratios of 0.7050
and 0.7199 missed their 0.50 limits. The cold ratio of 0.0952 met its 1.00
limit. GPU-memory sampling gaps and cross-application output-quality
alignment remain open.
[Exact E9 measurement and quality witnesses](../../qualification/p10-performance/evidence/integrated-e9-12f2151/README.md).

The separate `12f2151` native CUDA release completed the matched real E9
campaign on the same RTX 3090. The CUDA/OpenCL warm ratio of 0.5634 missed
the 0.50 limit. Cold and peak RSS ratios of 0.0844 and 0.4252 met their limits.
The candidate GPU-memory traces exceeded the required sampling interval, and
equal output quality remains open.
[Native CUDA E9 measurement](../../qualification/p10-performance/evidence/integrated-e9-cuda-12f2151/README.md).

The earlier native viewer-cache replay at `89c6a22` measured 43.274 ms idle
and 43.355 ms loaded P95 against the 33 ms limit. These viewer and prior
lifecycle measurements apply to their recorded revisions.
[Viewer cache measurement](p10-viewer-cache-evidence/README.md).

The isolated release replay passed all 20 P7 and 14 P8 interface IDs. It
verified native panels, retained results, GUI and CLI export parity, and
the actual 64-frame pause, resume, cancel, and worker reuse workflow.
[Interface evidence](p7-ui-evidence/5216ba4/README.md) and
[Linux package evidence](p9-linux-package-evidence/README.md).

Apple execution, retained platform builds, mutation/sanitizer assurance,
full owned-source coverage, end-to-end CUDA qualification, genuine driver
fault recovery, and the frozen P10 measurements remain separate exit gates.
[Sealed Linux branch coverage](p9-cpu-assurance-evidence/5216ba4-nightly/README.md).

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
