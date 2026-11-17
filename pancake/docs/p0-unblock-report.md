# P0 fix execution and remaining conditions

> Historical report. The adopted smaller P0 and subsequent closure evidence are
> recorded in [p0-work-log.md](p0-work-log.md) and the current plan. Original
> failed/unrun results below are preserved, not promoted to passes.

> Current scope update, 2026-11-17: AMD/Intel GPU and Windows execution/testing
> are best effort; build support remains required. Missing access alone no longer
> blocks P0. The [implementation plan](implementation-plan.md#testing-scope--adopted-user-decision-2026-11-17)
> is authoritative. Observed results below are retained as historical evidence.

Date: 2026-11-17. Integration worktree: `codex/p0-unblock`.

## Applied fixes

| Previous blocker | Change | Required verification |
|---|---|---|
| GPL-only dependency allowlist | Removed `[licenses]` allowlist and stopped invoking cargo-deny's license check. License declarations remain an informational inventory. | Actual advisories/bans/sources audit and negative policy tests. |
| Unmaintained helper crates | Five CubeCL manifests use maintained `pastey`; `spin_on` uses standard stack pinning. Exact upstream archives, modified-file hashes and regression tests retained. | Upstream unit, macro, immovable-future and Miri regressions; integrated lock audit. |
| Legitimate transitive version splits | Exact package-version/parent/requirement reviews generate cargo-deny and Clippy configuration. Expiry and graph changes fail closed; GPU/window resource families cannot receive exceptions. | Real dependency graph plus policy regression tests. |
| Linux font/D-Bus maintenance notices | Four exact advisory/version/parent exceptions expire 2026-12-17. Keep Wayland, system title fonts, settings and accessibility. | Other advisories remain enforced; upstream replacements require runtime validation. |
| Dead host DSL function in CubeCL coverage | Narrow macro patch honors function `expand_only`, retaining checked launch and default behavior for other users. No authored source excluded from coverage. | Compile contracts plus unchanged independent CPU/GPU values and strict source coverage. |
| Miri cannot call native JIT/GPU FFI | Pure host library tests run under Miri; real numerical integration remains mandatory in native/debug/release/layout/coverage checks. No ignored tests. | Host Miri, native tests, branch coverage, mutations and guardrails. |

[Dependency policy](dependency-policy.md) defines the enforced exceptions;
[upstream patch review](p0-upstream-patch-review.md) records source provenance.
The [kernel report](p0-kernel-qualification.md) records strict executable assurance.
Integrated `bash scripts/check.sh all` passed on macOS: 6 debug/release tests,
62/62 lines, 7/7 functions, 66/66 regions, 8/8 branches, 35/35 mutants caught,
3 strict host Miri tests and both native layout seeds.
[Linux GPU ASan remains failed](p0-sanitizer-investigation.md); no sanitizer
exception, suppression or test split was applied. Rowan also needed a narrow
explicit-layout patch; its pre-existing upstream ARC Miri limitation is recorded
separately and its unsafe implementation remains unchanged.

## Real runtime evidence

The numerical definition is one CubeCL horizontal `[1,2,1]/4` blur, using safe
TensorHandle resources and checked launch. u16 input widens to u32 storage and
accumulation; no precision is discarded. Independent expected values cover a
5x3 plane, full u16 range, clamped edges, width one and a 257x3 workgroup tail.
Runtime selection changes device/compiler, not kernel source.

| Runtime | Physical hardware | Observed result |
|---|---|---|
| CPU LLVM JIT, ARM64 | Apple M4 Pro | Exact values pass. |
| wgpu Metal | Apple M4 Pro, integrated GPU | Exact values pass; software fallback rejected. |
| Native Metal | Apple M4 Pro | Exact values, borders and tail pass; `msl_Apple M4 Pro`. |
| CPU LLVM JIT, x86-64 | AMD Ryzen 9 7950X3D | Exact values pass; `cpu_x86_64_load-512_f16-chain`. |
| wgpu Vulkan | NVIDIA RTX 3090, discrete GPU | Exact values pass; software fallback rejected. |
| Native CUDA | NVIDIA RTX 3090, CUDA 12.5/driver 580.178.04 | Exact values, borders and tail pass; `ptx_sm86`. Missing-runtime negative test fails as required before enabling `cubecl/cuda`. |

## Corpus and decoder evidence

The [codec probe](p0-codec-qualification.md) proves full sensor decode against
LibRaw, real Bayer DNG, RGB16 PNG/ZIP TIFF and float TIFF/EXR roundtrips. Identical
ICC bytes bypass color conversion without losing a bit; a real transform matches
system LCMS. Rawler recommended crop differs from LibRaw visible output on R50,
so RAW rendering/geometry parity remains blocked. The combined production graph
and the remaining file/error/resource cases are still unqualified.

The [dedicated macOS guest](p0-guest-access-qualification.md) now supports SSH
and an Aqua session through the existing managed key. Current AX/Automation
identity is still unqualified. The task guest was stopped; the unrelated guest
was left running. Linux task container is retained stopped with tools/cache for
reproduction; no host package, driver, permission or desktop setting changed.

## Conditions still required

| Gate family | Condition that can still stop P0 |
|---|---|
| P0.01–P0.03 | T50: 50 distinct native images at least 6000x4000, genuine high-bit-depth input. Exact complete support/build/package matrix and safe codec/RAW/color graph. No upscaling, repeated frames or source-scope reduction. |
| P0.02/P0.06 | Complete the required Apple/NVIDIA kernel battery and native GUI/permission checks. AMD/Intel GPU and Windows execution are now best effort; their absent runtime evidence is no longer a blocker. Retained platform/backend builds remain required. |
| P0.04–P0.09 | Complete resource/interop/budget, decoder/file/color, assurance and numerical battery on promised runtimes. A blur pass does not qualify FFT, warp or memory pressure. |
| P0.10–P0.15 | Real five-method/slab, hard registration/crop, job/device/error, native Slint controls, 64-frame lifecycle and deployment/accounting slices. None can be replaced with mock successful diagnostics. |
| P0.16–P0.17 | Equal-quality full-job warm/cold/memory/viewer measurements against frozen targets, then every gate passed on matching source/lock/contract provenance. |

User confirms there are **no AMD GPU, Intel GPU or Windows SSH/runner lanes**.
The later scope decision makes those execution/test lanes best effort while
retaining builds. Required Apple/NVIDIA work continues; test observations and
all other unresolved conditions remain unchanged.
Dependency policy cannot create missing hardware, supply a real 24 MP corpus or
substitute for unimplemented numerical/UI behavior. P1 remains closed.
