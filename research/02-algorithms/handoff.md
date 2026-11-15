# Phase 2 handoff: algorithm literature

Completed **2026-11-15**, after phase 1. Files: `review.md`, `method-matrix.csv` (29 rows, 14 columns), `sources.md` (26 evidence IDs), `base-comparison.md`, and `source-snapshots/`.

## Decisions supported by current evidence

- Keep focus-stack as an independent complex-wavelet baseline.
- Whole-stack and pairwise methods need distinct applicability labels. FocusDeep has real 30-frame evidence but configured frame-dependent fusion channels. StackMFF/V2 address whole-stack fusion; V2 requires ordered focal planes. Many recent high-resolution and diffusion methods still use near/far pairs.
- Selection, convex blending, coefficient fusion, and reconstruction have different failure constraints. Observed-value output does not guarantee correct focus, opacity, or geometry. Absence of a clean source pixel does not prove all reconstruction impossible. Generated detail in an unidentifiable region does not establish factual recovery.
- Current StackMFF V3 is a code release with manuscript in preparation. Its pinned November 2026 license explicitly disallows general integration/redistribution without permission; do not treat it as current open-source implementation material. Prior MIT releases are different artifacts requiring provenance checks. Several other papers' public code has no verified license or research-only terms.

## Measurement review requirements

1. Preserve target-view, visibility, intended focus domain/background blur, material, and temporal conventions. Finite-aperture focus changes and rail changes cannot be reduced to brightness-constant planar alignment in all cases.
2. Separate delivered pipeline and supplied-registration fusion-core tracks. Keep decode, photometric correction, crop, interpolation, tone mapping, sharpening/denoising, device path, and export in the run record. Native precision is distinct from linear radiometry, HDR headroom, and color management.
3. Test order according to method claims. Ordered methods receive valid ordered input; reversal/thinning can be meaningful acquisition tests. Arbitrary permutation is a claimed-invariance test, not an automatic defect for an ordinal method. Pairwise reductions need a fixed declared reduction tree; slabs/batches are parameters.
4. Avoid rewarding the exact objective a method optimizes without independent checks. MFF-SSIM optimization is relevant prior art, but its own score alone is not independent evidence. FocusDeep PSNR/SSIM against Helicon targets measures pseudo-teacher agreement. Conventional gradient/information metrics may favor noise/contrast, and global averages can hide small serious defects.
5. Qualify detail, halo/ringing, color/highlight, noise, geometry, coverage and unsupported structures separately, then validate any summary against held-out expert judgments. Seeds, stopping/sampling policy, parameter search budget, compute/memory limits, checkpoint hashes and failure cases need explicit reporting.

These are constraints and proposed controls for phase 3, not a finished or validated measurement suite.

## Dataset review leads and provenance risks

- **LSFD / FocusDeep:** real raw 30-frame stacks and pseudo-targets. Original paper uses 84 training and 10 test bursts, with two lenses. Macro focal coverage is sometimes incomplete. Preserve scene/burst grouping before crops and audit reference flaws. No license file observed in code tree; dataset terms need independent verification.
- **StackMFF series:** synthetic whole-stack data generated from images plus measured/estimated depth; benchmarks include Mobile Depth, Middlebury, FlyingThings3D and Road-MF. Check which are physically captured focus stacks versus synthetically refocused derivatives. Monocular depth predictions are not exact scene truth. Current V3 benchmark archives are author-produced outputs, not clean optical ground truth.
- **UHD-MFF:** original 2026 paper and author repository supply synthetic and real 4K pair data links. Useful for resolution stress, not a substitute for long macro bursts. Full data access and terms remain for phase 4.
- **PDRFuse HRMFF:** author README links a high-resolution test corpus; paired scope. **VAEEDOF MattingMFIF:** Blender-generated photographic cutout layers, up to seven inputs. “Blender” does not make such references exact for arbitrary 3D macro optics; retain asset identities and renderer/blur assumptions.
- **MFFW, Lytro, Real-MFF, MFI-WHU, Road-MF:** common short-pair comparison sets. MFFW includes manually fused references; other reference types vary and require source-level audit. Preserve all source-image ancestry, repeated subjects, crops, masks and synthesis assets across a single split group. Check both dataset and pretrained-checkpoint contamination before calling a test unseen.

## Code facts that affect benchmark adapters

## Remaining uncertainty

No algorithm, checkpoint, dataset download, visual output comparison, precision round trip, speed benchmark or detector validation was run. Several publisher full texts were inaccessible; the source ledger marks exactly what was read. GPU parity, physical material correctness, license compatibility, and hundred-frame high-resolution generalization remain unverified. Next phase may now begin: measurement and dataset agents can run together.
