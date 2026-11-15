# Synthetic and controlled-reference contract

Design proposal, **2026-11-15**. No scenes rendered or physical targets captured in this phase. Sources Q12–Q16 in [sources.md](sources.md) motivate optics, sampling and calibration constraints. A renderer-defined reference is not automatically exact for a real macro camera.

## 1. Three levels of truth

**Level A — analytic/verified targets.** Begin with deliberately simple scenes whose geometry, visibility, codes, color and image formation can be independently checked: flat Lambertian planes; known slanted edges; resolved filaments; two-depth opaque bars over random codes; point lights and flat gradients. For a simple declared aperture/PSF model, derive closed-form pixel/aperture integrals where possible or use verified interval quadrature with rounding bounds. Save the mathematical model and error bounds. This can support a strict correctness claim *under that simplified model*; it does not claim to reproduce all real macro optics.

**Level B — converged physical render estimates.** Use full 3D geometry and ray-traced finite-aperture imaging for rich scenes. Save independent high-sample repeats, convergence curves, per-pixel/region uncertainty and a practical reference-error floor. A finite Monte Carlo render is an estimate, even with a physically based renderer and many samples. Convergence checks do not strictly guarantee every pixel. Independent renderer/analytic-target cross-checks reduce shared generator/scorer mistakes; they do not turn a complex render into a proof.

**Level C — calibrated physical references.** Use measured targets, stable illumination, known geometry, independent repeats and external calibration. Exact code identity or target topology can be known while photographic radiance remains uncertain. Report calibration/manufacturing/lens/noise uncertainty. A carefully retouched composite or small-aperture photo is not automatically an exact all-in-focus image.

Synthetic output from an image-generation model is not a guaranteed reference for physical geometry or factual detail. Photographic cutout layers can be useful controlled boundary cases, but are a special layered scene model; they do not establish arbitrary 3D occlusion or material truth.

## 2. Declare the desired image

A stack does not specify one unique artistic/physical all-in-focus image. Define the benchmark target before making inputs:

| Target convention | Use | Limit |
|---|---|---|
| Central-ray/pinhole target at fixed view, integrated over declared pixel filter | Opaque geometry, line/code fidelity, boundary visibility and controlled diffuse radiance | Finite-aperture specular/transmissive appearance can legitimately differ |
| Explicit virtual extended-focus target with a declared focus/pupil policy | Study a chosen finite-aperture appearance convention | It is a defined computational image, not necessarily a single realizable camera exposure; some cross-depth mixing may remain |
| Subject-sharp/background-policy target | Practical macro preference and subject fidelity | Background sharpness is intentionally not a universal requirement |
| Independent local constraints or admissible target set | Real scenes/materials without one defensible complete reference | Membership/low residual does not certify the exact hidden truth |

Keep target classes separate in results. Do not compare a finite-aperture glossy input stack to a pinhole reference and automatically call every highlight difference a fusion error. Declare whether a surface is opaque, translucent, transparent, refractive, emissive or strongly view-dependent. For the first strict pixel-fidelity track, opaque diffuse targets are the most interpretable; glossy/transmissive scenes require a separate appearance/factual contract and qualified human judgments.

An admissible target set or interval must come from calibration/physics independently of the tested output. A positive lower bound on error to that set can establish inconsistency under its assumptions. A zero lower bound only means the output has not been excluded; broad uncertainty must not become a claim of perfect recovery.

## 3. Generate measurements, not masked blur approximations

The intended measurement model is schematically

`I_k[p,c] = encode_k( integral over wavelength, shutter time, pixel footprint and pupil of sensor_response_c * incoming_radiance(ray_k) * camera_weight + sensor_noise )`.

Specify every implemented approximation: thin lens or calibrated multi-element lens, entrance/exit pupil, aperture shape, focal setting, camera/rail position, pixel filter, spectra/RGB model, exposure, shading, diffraction/chromatic aberration if modeled, sensor noise, clipping, demosaic and transfer function. Thin-lens ray tracing is useful but omits important real-lens effects. PBRT's realistic-camera discussion explicitly distinguishes these models and notes pupil changes when focusing. [Q13–Q15]

For foreground defocus, trace aperture visibility before combining radiance. Blurring each segmented depth region independently and pasting sharp masks can miss asymmetric boundary mixing and hidden background rays. A global 2D blur of a sharp target does not generate general macro stacks with occlusion. Save lens/pixel samples or enough auxiliary ray information to audit mixed pixels on a small calibration subset.

Generate two geometry branches:

- Fixed target camera with known focus changes and modeled breathing/optical-center changes.
- Rail/camera motion with changing viewpoint and known per-frame visibility. A similarity transform cannot be assumed to register away all parallax. Score against the declared target view; preserve non-overlap and disocclusion masks.

Do not implicitly include simulated motion or exposure changes in only one method's input. Randomize nuisances by known factors, keep paired matched inputs, and label complete/incomplete focal coverage separately.

## 4. Preserve ground-truth layers and uncertainty

For each scene save:

- Full linear high-precision stack and delivered-encoding version, camera/focus/pose/time parameters, noise seeds/models, and exact file hashes.
- Declared sharp target(s), target-view depth/normal/object/material/opacity masks and visible filament/code geometry, including partially covered pixel fractions.
- Frame-wise transforms or dense correspondences only where well-defined; per-frame visibility, saturation, coverage and aperture-mixing information.
- Directly observed, contaminated/inferable and unobserved/indeterminate source-support labels, with how each was established. A contamination label is not a mathematical impossibility proof.
- Reference estimator/sampling version, convergence results, uncertainty maps, all asset ancestry, renderer version and configuration. Keep scene groups intact across splits.

Geometry truth and image truth are distinct. A known 0.2-pixel hair exists geometrically, but a sampled sensor may not resolve it. Do not score recovery of its exact photometric width as mandatory without adequate sampling/information. Include subpixel cases as ambiguity/aliasing stress tests and measure false confident invention separately.

## 5. Required scene matrix

Build independent families rather than one showcase scene with many crops:

| Family | Variables | Primary endpoints |
|---|---|---|
| Opaque depth boundaries | Step height, edge contrast, blur radius, foreground/background code, target visibility | Halo profiles, leakage, detail loss, geometry |
| Filaments and overlap | Diameter/orientation/depth/contrast, crossing count, gaps, curved hairs, true holes | Length precision/recall, topology, width, false transparency |
| Flat/ramp and repeating surfaces | Texture amplitude/frequency, color contrast, focus ambiguity, grid repetition | Seams, wrong focus, noise, alignment |
| Material/highlight cases | Roughness, chrome/glass/water, bright-point intensity, saturation, illumination/view changes | Component/energy/clipping and appearance-convention uncertainty |
| Acquisition stress | Focal spacing/range, frame count, exposure/noise, lens/rail movement, dust/hot pixels, drift | Coverage, failure, defect trails, robustness |

Vary nuisance factors independently and include crossed combinations after single-factor calibration. Asset, texture and shape families in metric qualification/final tests must differ from those used to choose detectors. Different random seeds on the same mesh/cutout/scene remain one ancestry family.

## 6. Verify generator before judging algorithms

Check zero aperture and single-depth limits; identical/duplicate frames; known translations/rotations; flat radiance conservation and ramp precision; closed-form edge/circle/line cases; and correct target-view opaque visibility. Compare finite-aperture ray traces with an independent implementation on small scenes. Preserve expected differences when changing aperture or focus affects specular appearance.

Run high-sample independent repeats and increasing sample/pixel resolution to estimate numerical/sampling error. Set score tolerances above reference uncertainty or carry uncertainty intervals; do not let renderer noise rank near-identical outputs. Check rare bright paths/fireflies and thin-geometry sampling specifically, since global mean convergence can hide them. Disable denoising on the truth estimate unless its bias is independently bounded and disclosed.

Use one controlled optics model for development and another calibrated lens/renderer setup for transfer tests. Distinguish known perturbation severity from actual optical fidelity. A detector that succeeds only on its own blur generator does not qualify for real macro scenes.

## 7. Physical target capture

Proposed kit: resolved wires/fibers over random-code planes at measured depths; a 3D slanted-edge/color/ramp target; chrome beads/bright points; glass or droplets; stable textured and low-texture subjects. Capture dark/flat frames, calibration geometry, exposure/noise repeats and enough focal overlap. Keep color, focus, rail position and timestamps.

Use a stable specimen and independently repeated full sweeps for noise/repeatability. For actual targets, external microscopy/metrology can constrain code/line geometry, but a different optical modality is not automatically a pixel-radiance reference. Small-aperture images change diffraction, pupil integration, exposure and possibly material appearance; use them as supplementary evidence, not guaranteed truth.

Existing long-stack data can stress realistic execution while this target corpus is built. The 999-frame source reported by the dataset review is one independent scene, with prefix subsets that change focal range. Matched-range stack-length studies must thin across the full sweep and still record changed spacing/completeness. Its TIFF and RAW active grids differ in supplied metadata, so choose and verify one decode/grid contract before alignment. Dataset-specific facts and acquisition status remain owned by [phase 4](../04-datasets/).
