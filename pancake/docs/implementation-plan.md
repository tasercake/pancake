# Pancake implementation plan

## 1. Objective and acceptance boundary

Write each production image-processing kernel once in CubeCL. Execute those
definitions on the native CPU runtime and supported GPU runtimes. Use one job
engine from both GUI and CLI. Qualify CPU and wgpu first, then native CUDA,
HIP, and Metal using the same engine and kernels.

## 2. Updated stack

| Piece | Selection | Purpose and reason |
|---|---|---|
| Application/build | Rust 2024, existing Rust 1.100.0 pin, Cargo workspace | Preserve scaffold lints, checked arithmetic, tool pins, lock discipline, and assurance gates. |
| UI | Slint, winit, FemtoVG-wgpu, accessibility | Declarative controls, menus, native accessibility, and input. Enable software presentation for the GUI VM/no-GPU environment. |
| Numerical kernels | CubeCL | One implementation of blur, resize, warp, pyramids, focus measures, reductions, FFT, fusion, and preview packing. No parallel WGSL/CUDA/MSL copies. |
| Compute runtimes | CubeCL CPU + wgpu initially; qualified native CUDA/HIP/Metal thereafter | Runtime changes compilation, resources, and dispatch, not algorithms. CPU uses LLVM JIT and its own worker pool. |
| Graphics/interop | wgpu | Shared device/queue, upload/copy, textures, resource limits, and Slint integration. Numerical kernels remain CubeCL. |
| Standard codecs | `image` with required features; direct `tiff` and `exr` APIs | Real u8/u16/float decode; reusable TIFF chunk buffers; full-precision export independent of previews. |
| RAW | Safe Rust-facing LibRaw binding, qualified with actual DNG tests | Preserve the existing RAW engine and broad camera support. Review binding safety/maintenance before selection; evaluate Rawler if no binding meets the gate. Decoder substitution requires recorded output/metadata tests. |
| Color | Explicit encoding metadata; Little CMS through a safe Rust API when needed | Separate source precision, transfer functions, profiles, and display conversion. Do not silently change the processing color space during compatibility port. |
| Small host math | Checked Rust; `nalgebra` when a real solve needs it | Transform composition, small optimizer solves, confidence decisions, and orchestration are shared host code, not duplicate pixel backends. |
| Scheduling | Standard threads and bounded channels | Bounded decode lookahead, compute ownership, pause/cancel, latest-preview coalescing. Avoid a competing Rayon numerical pool. |
| Public interfaces | `clap`, `serde`, JSONL diagnostics | Shared job configuration, CLI diagnostics, settings, and truthful backend/geometry evidence. |

Add dependencies only with their first real consumer and tests. Review their
actual licenses and locked graph. Do not add Burn solely to access CubeCL.

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

## 3. P0 feasibility gates

| Gate | Work and required exit evidence |
|---|---|
| Dependencies | Resolve one compatible Slint/CubeCL/wgpu set; inspect licenses, advisories, duplicates, native toolchains, and runtime distribution on each OS. Retain enforcement; review actual license additions. |
| Safe launches | Experiment uses unsafe `BufferArg::from_raw_parts`. First test CubeCL's safe `TensorHandle` allocation/`into_arg()` path, checking shape, dtype, capacity, aliasing, and lifetime. Verify generated macros under `unsafe_code = forbid`. Do not copy unsafe experiment statements. |
| CPU | Run the same nontrivial kernel on actual ARM64 and x86-64 native CPU runtimes. Qualify LLVM/JIT packaging, concurrency, errors, memory, and GPU-free startup. Software wgpu is not the native CPU runtime. |
| Native UI | Prove real AX IDs, selection/disabled values, native dialogs, menus, and pointer/keyboard input with unchanged VM driver. Screenshots alone do not establish operability. |
| Assurance | Resolve how native Slint/CubeCL integration coexists with Miri, all-features builds, sanitizers, full application-source coverage, and mutation gates. Miri cannot execute a GPU driver or LLVM JIT. Separate host-model and native integration evidence explicitly. |

If safe upstream APIs cannot satisfy the boundary, stop that milestone with a
concrete decision: obtain a safe upstream API or review a narrow boundary policy
change. Existing AGENTS.md requires reviewed policy changes for future FFI.
This plan grants no exception and does not weaken `forbid`, coverage, or other
gates. Resolve platform/assurance conflicts before expanding application code.

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

| Phase | Implementation and exit evidence |
|---|---|
| **P0 — foundation** | Complete all feasibility gates: safe launches, actual CPU/wgpu shared kernel, compatible graph, AX/native dialog proof, and viable assurance/runtime packaging. Record platform gaps. |
| **P1 — CLI** | Shared validated job config, help/errors, paths/globs/deduplication, limits, diagnostics/profile boundaries. Build actual release executable. Argument/help/rejection cases pass; image cases stay honestly red. |
| **P2 — I/O** | Real codecs, orientation, precision metadata, dimensions, RAW, and temporary-file/atomic-publish export. Local codec round-trip/failure tests first; no fake source-copy stacking mode. Prove Unicode, source immutability, and existing-export retention. |
| **P3 — primitives/basic fusion** | Shared CubeCL blur/resize/pyramid/gradient/variance/reduction/warp; checked reusable resources; weighted average and depth map. Start CPU then identical kernels on wgpu. Pass basic/odd/flat/precision/zero-region/determinism/cross-format and matching device cases. |
| **P4 — all methods/slabs** | Laplacian selection/reconstruction and documented tone/detail behavior; Hybrid fine/coarse fusion; full-precision Mertens; auto parameters, quality reports, slab streaming. Pass all methods and exposure batch-tree oracles before optimizing fusion. |
| **P5 — registration/crop** | Portable CubeCL FFT/phase correlation and image reductions; multiscale Euclidean/similarity/affine refinement; shared host solves/confidence. First/middle/previous references, breathing chains, high precision, legacy switch, dimension normalization, conservative ambiguity fallback. Compose transforms and resample once. Pass geometry, aligned/repeated slabs, crop, cascade, corrupt-alignment cases. |
| **P6 — device/resilience** | Qualified automatic selection, explicit CPU/GPU, normalized GPU index, bounded queues, actual completion telemetry, clean cancellation. Entire 341-case host set passes. Genuine driver recovery stays an evidence gap until reproducible; injected faults are local tests only. |
| **P7 — Slint workflow/viewer** | Import/options/advanced controls, sources/results, fit/100%/zoom/pan/navigation/comparison, native export, retained results, menus, batch/viewport export, manual crop. Observed AX mappings. Real GUI output agrees with host CLI. |
| **P8 — desktop lifecycle** | Preferences/profile isolation, recent folders, frame context, panel visibility, About, readable Lightroom status, unsaved close choices, pause/resume/cancel/reuse, missing-source and export-cancel/retry recovery. All 34 nonphysical-GPU GUI cases pass. |
| **P9 — runtime/platform qualification** | Same headless contracts per advertised CPU/wgpu/CUDA/HIP/Metal runtime and OS/hardware lane. Dedicated physical-GPU GUI case. Verify package launch, native libraries, driver versions, actual backend. |

Registration is a major numerical risk. Start with known-transform independent
oracles and multiscale refinement; add portable feature matching/deterministic
RANSAC if real images require coarse seeds. Do not maintain separate OpenCV-CPU
and CubeCL-GPU production pipelines. RustFFT may serve as a local test oracle,
not a divergent production CPU algorithm. Small host solves/control stay shared.

P0 tests AX feasibility early even though full workflows arrive in P7. P3/P4
deliver working vertical slices rather than an entire untestable UI.

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
runtime. If an optimized job outruns native pause/resume/cancel input, record
the testability conflict instead of slowing production or synthesizing a pass.

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
cherry-pick successful retries. Unrun hardware remains unverified.

Provision matching native binaries/libraries inside the guest before tests.
Workspace sync does not prove ignored `target/` artifacts were copied. Build in
an explicit guest preparation step or stage a matching package under the adapter;
never compile inside a case's process timeout. Virtual GPU is unnecessary for
software UI presentation and native CPU processing.

Target-scope native dependencies and runtime build profiles. Review compatibility
with existing all-features/feature-powerset gates. Do not silently skip platform
combinations, weaken checks, or add broad coverage/dependency exclusions.

## 10. Commands and release evidence

From repository root, after milestones create the real executable targets:

On a separately provisioned isolated physical macOS test session only:

After each milestone: run `bash scripts/check.sh all` inside `pancake/`, accrued
external cases, and tests for executable adapter code if introduced. Use unique
output directories. Record platform/remote checks not run. Never regenerate
baselines to hide failures, relax tolerances/timeouts, or retry defects away.

Measure cold JIT/setup separately from warm complete jobs; decode, registration,
fusion, export, preview, and input-to-present separately. Time completed GPU work,
not dispatch. Bound scratch, decoded lookahead, and preview/result caches by
bytes; use lossless spill when needed. Spatial tiling needs correct halos and
is distinct from frame slabbing. Compare real RAW/TIFF stacks and peak memory;
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
