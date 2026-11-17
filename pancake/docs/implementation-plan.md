# Pancake implementation plan

## 1. Objective and acceptance boundary

Write each production image-processing kernel once in CubeCL. Execute those
definitions on the native CPU runtime and supported GPU runtimes. Use one job
engine from both GUI and CLI. Establish feasibility of CPU, wgpu, and every
promised native CUDA/HIP/Metal path in P0, using the same engine and kernels.
Later phases complete and revalidate the product on that already-qualified
matrix; they do not introduce first-time platform or algorithm feasibility gates.

## 2. Updated stack

| Piece | Selection | Purpose and reason |
|---|---|---|
| Application/build | Rust 2024, existing Rust 1.100.0 pin, Cargo workspace | Preserve scaffold lints, checked arithmetic, tool pins, lock discipline, and assurance gates. |
| UI | Slint, winit, FemtoVG-wgpu, accessibility | Declarative controls, menus, native accessibility, and input. Enable software presentation for the GUI VM/no-GPU environment. |
| Numerical kernels | CubeCL | One implementation of blur, resize, warp, pyramids, focus measures, reductions, FFT, fusion, and preview packing. No parallel WGSL/CUDA/MSL copies. |
| Compute runtimes | CubeCL CPU, wgpu, and promised native CUDA/HIP/Metal paths qualified in P0 | Runtime changes compilation, resources, and dispatch, not algorithms. CPU uses LLVM JIT and its own worker pool. |
| Graphics/interop | wgpu | Shared device/queue, upload/copy, textures, resource limits, and Slint integration. Numerical kernels remain CubeCL. |
| Standard codecs | `image` with required features; direct `tiff` and `exr` APIs | Real u8/u16/float decode; reusable TIFF chunk buffers; full-precision export independent of previews. |
| RAW | Safe Rust-facing decoder selected and qualified in P0 | Start with LibRaw to preserve the existing RAW engine; evaluate Rawler if no safe maintained binding meets the gate. Resolve the choice, DNG/metadata behavior, and licenses before P1. |
| Color | Explicit encoding metadata; Little CMS through a safe Rust API when needed | Separate source precision, transfer functions, profiles, and display conversion. Do not silently change the processing color space during compatibility port. |
| Small host math | Checked Rust; `nalgebra` when a real solve needs it | Transform composition, small optimizer solves, confidence decisions, and orchestration are shared host code, not duplicate pixel backends. |
| Scheduling | Standard threads and bounded channels | Bounded decode lookahead, compute ownership, pause/cancel, latest-preview coalescing. Avoid a competing Rayon numerical pool. |
| Public interfaces | `clap`, `serde`, JSONL diagnostics | Shared job configuration, CLI diagnostics, settings, and truthful backend/geometry evidence. |

P0 qualification spikes provide the first real consumers and tests for critical
dependencies. Resolve their complete platform graphs, licenses, and native
requirements then. Later additions must remain within that qualified design;
an architectural dependency change reopens its P0 gate. Do not add Burn solely
to access CubeCL.

### Runtime portability

| Path | Hardware | Kernel source | Display boundary |
|---|---|---|---|
| CubeCL CPU | Supported ARM64/x86-64 CPU | Same definitions | Bounded preview upload, or Slint software presentation. |
| CubeCL wgpu | Apple/Metal; supported NVIDIA/AMD/Intel Vulkan/D3D12 | Same definitions | Shared Slint device and queue when possible. |
| CubeCL CUDA | Supported NVIDIA/driver combinations | Same definitions | Initially transfer only preview data to wgpu display. |
| CubeCL HIP | Supported AMD/ROCm combinations | Same definitions | Same conservative preview bridge. |
| CubeCL native Metal | Supported Apple hardware | Same definitions | Explicit bridge until native-resource interop is qualified. |

**wgpu does not execute CUDA or HIP.** NVIDIA via Vulkan/D3D12 is GPU support,
not CUDA execution. Native CPU/CUDA/HIP/Metal require their CubeCL runtimes in
that platform build. One source tree does not mean one binary across operating
systems, universally available instructions, or automatic support for every
accelerator. Include and qualify runtimes per supported OS/package.

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

## 3. P0: all known feasibility and external-prerequisite gates

**P1 cannot start until every P0 gate passes with recorded evidence.** Unknown,
unavailable, awaiting review, and untested are blocking statuses, not accepted
risks to carry into a later milestone. Resolve a failed gate within the chosen
stack and fixed contract; a necessary scope or policy change must be made
explicit before leaving P0, never silently substituted mid-port.

P0 is now a substantial feasibility phase, not a short dependency smoke test.
It includes bounded, real numerical and native-UI vertical slices, including
enough worker/engine behavior to test lifecycle timing. Reuse qualified code
in later phases. Do not use placeholder kernels, fake executables, injected
successful diagnostics, or artificial waits as evidence. This document update
does not make a documentation-only statement into a passing execution result.
The following progress ledger distinguishes completed reviews from unexecuted
application/kernel work.

### Execution progress — 2026-11-17

| Gate | Current marker | Evidence and remaining condition |
|---|---|---|
| P0.02 | **BLOCKED; available lanes qualified in part** | Apple/NVIDIA shared-kernel execution proven. Linux private desktop/AT-SPI provisioned. [Dedicated macOS guest](p0-guest-access-qualification.md) SSH/Aqua access works via existing managed key; current AX/Automation identity still unqualified. User confirms no AMD/Intel/Windows test access; support matrix unchanged. |
| P0.03 | **BLOCKED; foundation graph passes** | Slint 1.19.0 + CubeCL 0.11.4 + wgpu 30.0.2 adopted. License gate removed; patched helpers and exact expiring reviews pass dependency checks. Native CPU/wgpu/CUDA/Metal blur builds run. Complete combined codec/RAW/color graph, RAW engine parity and all promised package/platform builds remain unqualified. |
| P0.04 | **IN PROGRESS; safe blur launch proven** | Same checked TensorHandle kernel runs under application unsafe-forbid on Apple CPU/Metal and Linux CPU/Vulkan; malformed shapes fail before runtime creation. Remaining capacity/aliasing/decoder/texture/native boundaries require proof. |
| P0.05 | **BLOCKED; Linux GPU ASan failed** | Integrated macOS `check.sh all` passed: 6 debug/release tests, 100% of 62 lines/7 functions/66 regions/8 branches, 35/35 mutants caught, strict host Miri and both layout seeds. [Linux ASan](p0-sanitizer-investigation.md) passes host/CPU tests but CubeCL GPU setup fails; minimal wgpu succeeds with external loader leaks. No suppression, ignored test or target split. Full TSan/Windows checks not run. |
| P0.06 | **BLOCKED; six available execution paths pass blur** | Identical u16 blur passes ARM64 CPU LLVM JIT, Apple Metal/wgpu, x86-64 CPU LLVM JIT and physical RTX 3090 Vulkan/wgpu. Native CUDA and native Metal also pass the blur probe. User confirms no AMD/Intel test access; HIP and full kernel battery remain unqualified. |
| P0.07 | **NOT RUN** | Historical viewer interop experiment exists; production-safe bounded cache/ownership design and its measurements are unqualified. |
| P0.08 | **BLOCKED; safe codec subset proven, RAW parity open** | [Pinned probe](p0-codec-qualification.md) proves CR3/DNG sensor samples, RGB16 PNG/ZIP TIFF and float TIFF/EXR bit-exact; ICC no-op/real-transform tests pass. Rawler crop 5088x3392 differs from LibRaw visible 5118x3412 on real R50 input; safe maintained LibRaw binding/general parity design unresolved. Other orientation/file/error/budget tests remain. |
| P0.09 | **IN PROGRESS; u16 blur subset passes** | Independent exact u16 values, clamped one-pixel borders, odd dimensions and workgroup tails pass on CPU/Metal/Vulkan. Remaining resize/pyramids/warp/variance/reduction/FFT/determinism battery still required. |
| P0.10 | **NOT RUN** | No five-method/slab numerical slice implemented or qualified. |
| P0.11 | **NOT RUN** | Registration/crop numerical feasibility not executed. |
| P0.12 | **NOT RUN** | Real job/device/error mechanisms not implemented or qualified. |
| P0.13 | **NOT RUN** | No actual Slint accessibility replay through current native driver. |
| P0.14 | **NOT RUN** | No actual worker lifecycle replay; no artificial timing substitute introduced. |
| P0.15 | **IN PROGRESS; isolated access works** | Linux private desktop and physical NVIDIA GPU visible together. Dedicated macOS guest SSH/Aqua/tools verified and guest stopped after check. Actual Slint controls/export, current guest AX/Automation identity, physical-GPU GUI stacking and package/accounting route remain unqualified. |
| P0.16 | **NOT RUN; reference diagnostics available** | D150 Hybrid CPU 30.585 s, peak RSS 2,897,117,184 bytes; E9 exposure CPU 0.877 s, peak RSS 237,469,696 bytes, each n=1. These use a reduced/mismatched profile and no matched Pancake job; no target, warm/cold statistic, half-memory or quality-equivalence pass. |
| P0.17 | **BLOCKED** | P0 is not complete; P1 cannot start with these open gates. |

Finite prototypes cannot prove all future implementation details. Full-case
acceptance and regressions remain checked throughout P1-P10. The purpose of
this ordering is to expose every identified policy, hardware, toolchain,
numerical, lifecycle, and performance feasibility risk before the main port.

Execute these gates in order. Every row requires an artifact, a concrete verdict,
and a named condition that would reopen it; documentation-only assertions do
not qualify as execution evidence.

| Gate | Feasibility question and required exit evidence |
|---|---|
| **P0.02 — hardware and test access** | Secure actual ARM64/x86-64 CPU, Apple, NVIDIA, AMD, and Intel test access for every promised combination, including native CUDA/HIP/Metal. Provision supported native lanes and a physical-GPU GUI route: private Linux desktop on a GPU host or isolated physical macOS session; retain the macOS VM lane for that platform's own coverage. Verify permissions, tools and repeatable access. No inaccessible hardware may be deferred to P9. |
| **P0.03 — complete dependency/build graph** | Resolve compatible Slint/CubeCL/wgpu, codec, RAW, color, and host-math dependencies on every promised target. Build/link real runtime profiles; verify LLVM/JIT and native-library distribution, licenses, advisories, duplicate policy, and all-features/feature-powerset compatibility. Resolve package-launch/signing/tool access requirements. No unselected critical dependency remains for P2 or P9. |
| **P0.04 — safe resource and launch boundary** | Test safe `TensorHandle` allocation/`into_arg()` with checked shape, dtype, capacity, aliasing, and lifetimes; test macro-expanded code under `unsafe_code = forbid`. Include decoder, texture, and native integration boundaries. Never copy the experiment's unsafe `BufferArg::from_raw_parts` statements. If safe APIs are insufficient, resolve the upstream API or reviewed policy decision here. |
| **P0.05 — real assurance compatibility** | Run the existing local gates and relevant platform/sanitizer checks against executable qualification code, not only the empty scaffold. Demonstrate full application-owned source coverage, mutation resistance, and a valid Miri/native-JIT/GUI verification design. Miri cannot run a real GPU driver or LLVM JIT; any necessary policy/design decision must be resolved now without silent exclusions. |
| **P0.06 — shared kernels on every runtime** | Run the same representative image-kernel battery on native ARM64/x86-64 CPU, wgpu GPU paths, and each promised native CUDA/HIP/Metal runtime. Exercise odd dimensions, indexing, reductions, workgroup/subgroup differences, memory limits, cold compilation/cache recovery, and GPU-free CPU startup. Record completed execution and exact device/runtime identity. A scalar toy kernel or software wgpu adapter is insufficient. |
| **P0.07 — display interop and bounded resources** | Prove Slint/wgpu device/queue sharing, producer-consumer ordering, resource ownership through completion/eviction, and the explicit native-runtime preview bridge. Demonstrate bounded scratch/decode/cache storage and spatial tiling/halos on a large image. Confirm RGBA8 preview never limits full-precision export; accept or resolve display-color/HDR scope here. Do not defer memory scalability to P10. |
| **P0.08 — codecs, RAW, color, and files** | Select the actual safe decoder and prove real Bayer DNG decode, true RGB16, float TIFF/EXR interpretation, grayscale/alpha, all EXIF orientations, mixed dimensions, Unicode/non-BMP paths, and PNG16/TIFF16 round-trip identity. Exercise malformed/oversized input and failed/atomic export retention. Freeze transfer-function/profile behavior; resolve binding safety or decoder substitution before P1. |
| **P0.09 — numerical precision and determinism** | Qualify real blur/resize/pyramid/warp/variance/reduction/FFT operations across the runtime matrix with independent expected values. Test u16 low-bit retention, fixed ties/reduction order, zero weights, flat/odd/narrow inputs, reconstruction, and portable accumulation. Meet existing per-case precision/parity limits without f16 shortcuts, vendor-specific numerical implementations, or relaxed tolerances. |
| **P0.10 — five-method and slab feasibility** | Build bounded vertical slices of weighted average, depth map, Laplacian, Hybrid, and exposure fusion using the shared primitives. Test both focus planes, detail/error limits, documented enhancement behavior, auto-parameter extremes, aligned/overlapping slabs, and the 2/4/9-frame exposure batch tree including a one-frame remainder. Preserve precision/determinism on all promised runtimes. |
| **P0.11 — registration and crop feasibility** | Exercise hardest generated geometry: translation, Euclidean, similarity, affine, breathing, first/middle/previous reference, high precision, mixed dimensions, narrow cascades, repeated/nonrepeating texture, and ambiguous-motion identity/zero confidence. Validate reference-to-source matrices, corner/interior error, drift, one-resample composition, and symmetric/manual crop. Decide any portable feature/RANSAC seed design now; no first registration feasibility proof waits for P5. |
| **P0.13 — native UI contract mechanisms** | Use the unchanged native driver to operate each required control/menu/dialog/gesture mechanism in Slint, including collection selection/virtualization, viewer metadata, comparison, advanced visibility, native file/export panels, isolated preferences, retained-result/crop coordinates, history, and recoverable dialogs. Verify actual pixels/state rather than action acknowledgements. Prove GUI/CLI use of the same numerical result. |
| **P0.14 — lifecycle timing** | Run the unchanged 64-frame lifecycle scenario through a real worker performing normal Laplacian/breathing decode, alignment, and fusion. Prove native pause/resume/cancel and reuse without sleeps, barriers, or business hooks. Measure the available interaction window and cancellation latency. If optimized work outruns native input, resolve that fixed-contract conflict in P0; recording it as a future P8 problem does not pass this gate. |
| **P0.16 — performance feasibility** | Measure the real numerical/worker/viewer slices against P0.01's frozen targets on agreed workloads/hardware. Include all five methods, representative registration/slabs, RAW/TIFF decode/export, cold JIT, warm completed jobs, memory pressure/cache eviction, and input-to-present latency. An exposure-only kernel or enqueue timing is insufficient. Show the chosen design can meet targets before committing to the main port. |
| **P0.17 — closure review** | Produce the P0 evidence ledger: every gate PASS, reproducible commands, source/lock/contract/device provenance, artifacts, resolved policy/dependency choices, and locked performance targets. No required hardware, unresolved numerical failure, accepted-later risk, missing numeric threshold, or untested launch/verification mechanism remains. P1 starts only after this closure condition is met. |

P0.01 has fixed initial engineering limits in
[performance-targets.json](performance-targets.json): at least 2x warm
completed-job speed, at most 50% peak memory, no cold-job median regression,
viewer P95 at most 33 ms, and pause/cancel acknowledgement at most 250 ms.
These are targets defined during authorized P0 work, not prior user-supplied
figures or measured achievements. The [measurement contract](p0-performance-targets.md)
defines same-device/quality comparisons, separate unified/discrete memory
accounting, ten measured warm jobs after two warm-ups, five fresh-cache jobs,
200 viewer events and 30 lifecycle requests per operation. Real corpus and
matched baseline selection remain pending; P10 cannot change thresholds after
seeing results.

### Evidence ledger and stop rule

At P0 execution, keep per-gate records in adapter-owned verification artifacts,
with status `not_run`, `blocked`, `failed`, or `passed`; all start `not_run`.
Each record identifies the original late-phase risk, tested scenario/runtime,
commands, numerical thresholds, results, artifact hashes, and regression owner.
Dependency changes, new promised hardware, different accumulation/registration
designs, changed memory ownership, or altered targets reopen relevant gates.

If a gate fails, finish only independent work useful to resolving P0. Do not
proceed into P1 with a caveat such as "validate CUDA later" or "fix lifecycle in
P8." Keep the chosen stack, fixed contract, and original requirements unless an
explicit decision changes them. The existing absence of a portable real driver
failure trigger remains a declared scope limit, not a reason to fabricate proof.

If safe upstream APIs cannot satisfy the boundary, stop that milestone with a
concrete decision: obtain a safe upstream API or review a narrow boundary policy
change. Existing AGENTS.md requires reviewed policy changes for future FFI.
This plan grants no exception and does not weaken `forbid`, coverage, or other
gates. Resolve platform/assurance conflicts before the main implementation
phases. P0 evidence must exercise real code; passing empty-scaffold checks is
not sufficient.

## 4. Architecture and ownership

Start with modules in the existing crate. Extract crates only when an actual
build boundary requires it: CLI must be window-system independent, kernels must
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
| **P0 — all blocker qualification** | Execute P0.01-P0.17, including real critical numerical/UI/lifecycle slices, all promised runtimes, physical-GPU GUI evidence, assurance compatibility, packaging route, and measured frozen performance targets. No open blocker enters P1. |
| **P1 — CLI integration** | Complete shared job config, help/errors, path/glob/deduplication, limits, diagnostics/profile interfaces already proven in P0.01/.12/.15. Build release executable and pass full argument/help/rejection selections. |
| **P2 — I/O integration** | Integrate the decoder/codec/color/export choices already qualified in P0.03/.04/.08. Complete error handling, Unicode, orientation, precision, atomic export, and source immutability cases; no first RAW-binding decision here. |
| **P3 — primitives/basic fusion** | Harden/reuse P0.06/.09/.10 shared primitives, resources, weighted average, and depth map. Expand full parameter/shape/error coverage across qualified runtimes and pass matching device cases. |
| **P4 — all methods/slabs** | Integrate P0.10's Laplacian/Hybrid/exposure/slab designs into complete engine, auto parameters, reports, and batch handling. Full numerical cases pass before optimization. |
| **P5 — registration/crop integration** | Complete P0.11's proven geometry/FFT/refinement/confidence/crop design, including all references, breathing, high precision, legacy switch, cascades, and slabs. Run the complete geometry matrix; no first registration feasibility proof here. |
| **P6 — device/resilience integration** | Complete P0.12's device selection, telemetry, queues, cancellation, and recovery mechanisms. All 341 host cases pass; keep the previously declared genuine-driver-trigger evidence gap explicit. |
| **P7 — complete Slint workflow/viewer** | Assemble P0.07/.13's qualified controls, gestures, panels, collections, retained results, comparison, native export, batch/viewport export, and crop. Complete native workflow and GUI/CLI parity checks. |
| **P8 — desktop lifecycle integration** | Complete preferences, history/context, About/Lightroom status, unsaved workflows, and error recovery. Rerun P0.14's already-working real lifecycle scenario against integrated application; all 34 VM-compatible GUI cases pass. |
| **P9 — full matrix regression/package qualification** | Run complete applicable contracts on P0.02/.06/.15's already-available hardware, runtimes, and packaged launch paths, including physical-GPU GUI. No first hardware procurement, native-runtime proof, or test-route discovery here. |

Registration feasibility is resolved in P0.11 using known-transform independent
oracles and multiscale refinement; establish any portable feature/deterministic
RANSAC seed requirement there. Do not maintain separate OpenCV-CPU
and CubeCL-GPU production pipelines. RustFFT may serve as a local test oracle,
not a divergent production CPU algorithm. Small host solves/control stay shared.

P0 proves all identified risky mechanisms in real vertical slices. P1-P10
complete their integration and exhaustive acceptance, and can still find bugs or
regressions. A newly discovered architectural blocker reopens P0 immediately;
do not carry it to a later release milestone or claim P0 prevents unknown defects.

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
runtime. Resolve an insufficient native interaction window in P0.14 without
slowing production or synthesizing a pass. Later integration must preserve the
qualified behavior; a recurrence reopens P0.14 rather than becoming a new P8
feasibility caveat.

## 9. Verification lanes and full accounting

| Lane | Required evidence |
|---|---|
| Host numerical | 341 IDs: 46 CLI + 223 algorithms + 72 devices. Real CPU/physical GPU, outputs and diagnostics. |
| Dedicated macOS GUI VM | 34 interface IDs. Software presentation/native CubeCL CPU as needed; real native input, AX/screenshot traces, exports, completed host CLI parity. |
| Runtime/OS matrix | ARM64/x86-64 CPU; Apple wgpu/Metal; NVIDIA wgpu/CUDA; AMD wgpu/HIP; Intel wgpu, for supported combinations. Record actual OS/driver/runtime versions. |

An adapter-owned release verifier may combine IDs from lanes only with matching
source/descriptor/lock/contract provenance. Preserve every raw report/status;
reject missing, duplicate/conflicting verdicts or incomplete runs. Explain the
VM unsupported entry alongside physical evidence; never rewrite it as a pass or
cherry-pick successful retries. Qualify this accounting route in P0.15. Unrun
hardware remains unverified and cannot satisfy its P0 support-matrix gate.

Provision matching native binaries/libraries inside the guest before tests.
Workspace sync does not prove ignored `target/` artifacts were copied. Build in
an explicit guest preparation step or stage a matching package under the adapter;
never compile inside a case's process timeout. Prove this preparation/launch route
in P0.15, before relying on it for later GUI gates. Virtual GPU is unnecessary
for software UI presentation and native CPU processing.

Target-scope native dependencies and runtime build profiles. Review compatibility
with existing all-features/feature-powerset gates during P0.03/.05. Do not silently
skip platform combinations, weaken checks, or add broad coverage/dependency
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
`nightly`, `branches`, `guardrails`; CI additionally runs native platform and
Linux address/thread sanitizer checks. P0.05 must establish that these gates
work for the real chosen architecture. Later runs verify continued compliance;
they are not first-time feasibility experiments.

Measure cold JIT/setup separately from warm complete jobs; decode, registration,
fusion, export, preview, and input-to-present separately. Time completed GPU work,
not dispatch. Bound scratch, decoded lookahead, and preview/result caches by
bytes; use lossless spill when needed. Spatial tiling needs correct halos and
is distinct from frame slabbing. Fix targets in P0.01 and prove performance and
resource feasibility in P0.07/.16; P10 repeats that acceptance on the integrated
application. Compare real RAW/TIFF stacks and peak memory;
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
