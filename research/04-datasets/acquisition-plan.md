# Dataset acquisition and construction plan

Status: **proposed**. No images or archives were acquired here; small source metadata/manifests were. Dataset intake and benchmark implementation are subsequent work, not completed experiments.

## 1. First intake tranche

Start with Sood's explicitly licensed CR3 stack and the two explicitly licensed Figshare optical-stack releases. Fetch immutable revisions/files, verify provider hashes, decode representative endpoints/middle frames, then inventory all images. The complete Sood RAW sweep is creator-estimated 8.4 GB; TIFFs about 25 GB, so its RAW data is the useful precision source. Use the [remote file manifest](hf-999-file-manifest.csv) to select paths; it is not proof that image bytes were already verified.

Confirm focal order, actual focal coverage, clipping, motion and optical/material mode. Retain RAW, original JPEG/TIFF, all EXIF and untouched provider archives. Identify duplicated spider ZIPs before extraction. Keep confocal CZI separate from ordinary optical focus stacks. Record amber-piece/specimen links and distinguish existing fused outputs from independent references.

Next, resolve LSFD's image-use terms and acquire its original archive when storage is available. Preserve original train/test IDs, reference provenance, native RAW and registered derivatives. Do not train against the pseudo-target as though it were exact. Then intake EDMF and high-resolution pairs for targeted regression, applying the saved missing-pair audit. Learn2Refocus and LearnAF are optional expansion tranches after access/terms/size checks.

No author contact, access-gate agreement, image publication or license acceptance was performed by this review. Unresolved terms are acquisition tasks, not a reason to fabricate a cleared dataset.

## 2. Metric-development pilot: 24 independent groups

This planning target matches the measurement review: **12 controlled scene/asset families + 12 real macro groups**. It is a feasibility and detector-development pilot, not a sample-size proof or final train/validation/test corpus.

Controlled families should vary geometry/materials independently: opaque wire/hair crossings over coded texture, slanted depth ramps, closely spaced opaque edges, isoluminant colored filaments, smooth/noisy color planes, calibrated edge/line-width targets, glossy beads, facet highlights, transparent films/droplets, refractive media, occlusion/focal-coverage gaps, and deliberate motion/viewpoint changes. Use owned or clearly licensed targets/assets. Related layouts or repeated captures remain in one family.

The 12 real macro groups should span specimens/materials and both focus-ring and rail capture. Include fine hairs, overlapping structures, smooth surfaces, reflective and translucent cases. Record magnification, optical path, pupil/aperture, sensor/lens, rail/focal settings, focus spacing, illumination, timing, exposure and RAW response. Include clean and deliberately defective acquisitions; label the latter as robustness challenges. Sample representative everyday captures separately from selected hard cases so challenge frequency is not mistaken for natural prevalence.

Repeat selected captures with unchanged scene/lighting and acquire dark/flat/color/edge calibration frames. Repeats estimate noise/repeatability; they do not increase independent-group count. Keep enough full focal coverage for a clean track, then derive thinning/gap variants within the same group.

## 3. Controlled real reference strategy

Define target viewpoint, intended sharp subject/background, material class and time/pose policy before capture. Use stable independently measured target geometry and printed/coded patterns with known structure. Acquire sharply observed local reference patches through a documented independent capture or measurement method. Estimate registration, blur, color and mask uncertainty; do not assert pixel-perfect natural-scene truth.

Small-aperture photographs can be helpful supporting observations but have different diffraction, noise, exposure and material appearance. Manual composites and commercial stacks can support human inspection but remain pseudo-references. Label direct clean source support, contaminated/inferable support and unobserved/indeterminate regions separately. Absence of a clean source pixel does not establish that all reconstruction is impossible.

For opaque filaments, independently measure silhouette/width and use a coded background to detect false transparency. For highlights, record saturated and unsaturated RAW response separately and keep light/view changes explicit. A real specular highlight is not a diffuse surface landmark. For transparent specimens, choose a material-specific target; do not impose an opaque single-depth label.

## 4. Synthetic construction tranche

Use the measurement review's [synthetic reference contract](../03-measurement/synthetic-ground-truth.md). Build independent 3D scene families from owned procedural geometry and rights-cleared assets. Store mesh/texture/background/HDRI identities before splitting; use disjoint asset pools for train, metric qualification and final test.

Render occlusion-aware aperture integration rather than independently blurring a 2D final image. Save linear floating-point radiance, depth/visibility/opacity/material IDs, camera/pupil/focus geometry, transforms, independent sample/convergence records and focal order. Define exactness as conditional on this model and numerical tolerance. Use several target conventions when needed: central-view pinhole for specified opaque/Lambertian geometry, and separately declared finite-aperture/material counterfactual targets. A pinhole image cannot silently become exact truth for a glossy finite-aperture appearance.

Test renderer convergence with independently sampled higher-quality renders, denoiser-off controls and a second analytic or independent-renderer check on simple scenes. Preserve Monte Carlo uncertainty. Do not use the evaluated stacker's own masks, focus measures or learned depth estimates to create “ground truth.” Validate synthetic-to-real transfer on newly captured targets.

Introduce blur/noise/exposure/ghosting/halo/clipping/crop defect ladders only on metric-development families. Preserve known defect masks, amplitude, extent and other nuisance variables. Keep clean negative controls and mixed defects. Independently created qualification families test whether a metric detects actual failure rather than the injection program's signature.

## 5. Full benchmark release gates

Scale beyond the pilot only after detector feasibility, rater agreement and pipeline reproducibility are measured. Use observed group variability, catastrophic-defect frequency and desired confidence bounds to set final group counts. New algorithm-corpus 60/20/20 allocation remains a proposal; ratios do not substitute for power or rare-case coverage.

Before release: complete rights/provenance review; validate raw/archive checksums and decodes; resolve duplicate/ancestry components; audit focal coverage and masks; preserve original official partitions; freeze test and metric versions; qualify artifact measures on new groups; obtain blinded expert comparisons; publish exclusions and uncertainty. Publish only images/assets whose terms support the intended distribution. Unresolved datasets can remain link-only optional regression panels.
