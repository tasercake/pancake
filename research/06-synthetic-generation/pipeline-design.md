# Proposed procedural focus-stack generator

**Synthesis and engineering proposal, 2026-11-17.** No generator has been implemented or benchmarked in this review. The architecture below is the recommended next experiment, based on the four topical reviews, not a claim that one software stack has already won a comparison.

## 1. Recommended architecture

Build a **scene-first generator with replaceable camera and sensor modules**. The authoritative inputs are geometry, materials, illumination, animation and acquisition parameters. Generate each observation and reference from that shared state. A sharp photograph plus estimated depth is a useful training source, but cannot be the authority for hidden geometry or exact macro visibility.

Use three explicitly labeled generation lanes:

| Lane | Construction | Intended use | Main unresolved limitation |
|---|---|---|---|
| Full 3D geometric optics | Procedural scene, finite-pupil rays, physical camera pose and focus schedule | Main controlled evaluation; boundary/filament/motion fidelity | A stock thin lens omits compound-lens aberrations, dispersion and diffraction |
| Optical audit | Small shared scenes through compound-lens rays and separately qualified wave/PSF models | Measure which optical omissions matter; calibrated camera-profile challenges | Lens prescription/focus calibration, wave/visibility coupling and cost |
| Fast surrogate | Known rendered RGB/depth/layers plus PSF bank and explicit approximate compositing | Training scale, ablations and large parameter sweeps | Approximation error at occlusions, hidden surfaces, view-dependent materials |

These are complementary lanes. A more elaborate PSF does not make a single-depth image into complete 3D, and a path tracer does not make a thin lens into a real commercial macro lens. See the [optics comparison](optics-and-renderers.md) and [prior-art comparison](datasets-and-prior-art.md).

**First implementation choice:** Blender/Cycles for scene authoring and the first full-3D lane; selective pinned Infinigen nature generators plus purpose-built analytic/geometry-node targets; an independent small Mitsuba or analytic camera harness; PBRT realistic-camera experiments for a compound-lens subset; DeepLens/CLDefocus as optical components to investigate. Keep spectral/RAW work behind a camera/sensor interface so an ISET-based audit remains possible. This minimizes new scene-authoring work while preserving a path to stronger optics; production-backend selection still needs measured convergence and throughput.

There is unusually direct prior art: FOSSA's **Infinigen Defocus** uses procedural scenes and Cycles-rendered focal stacks. Its appendix describes **nine focus distances at five apertures**, plus AiF: 46 images per scene, with nine frames in each fixed-aperture sweep. It is a useful reproduction target and baseline, rather than a reason to abandon a macro-specific generator. Its indoor scale, thin-lens imaging and reference precision still need qualification for this task. [Paper, appendix A2](https://arxiv.org/html/2603.26658v2#A2)

## 2. A precise sample contract

At minimum, a sample should contain:

```text
sample/
  manifest.json
  scene/                 procedural specification + asset/material dependencies
  observations/clean/    N noiseless, finite-DOF linear observations
  observations/delivered/ N RAW-like and/or fixed-ISP deliverables
  targets/               canonical AiF target(s), each with its own contract
  geometry/              target/frame central-view depth, normals, surface/instance IDs
  correspondence/        defined surface mappings, visibility and invalid masks
  support/               pupil/time contamination, clipping and focal-coverage labels
  uncertainty/           reference repeats, variance/error estimates, convergence metadata
  qa/                    acceptance results, logs and artifact hashes
```

This is a proposed layout, not an existing on-disk format or schema. Define the manifest before writing consumers:

| Field group | Required content |
|---|---|
| Identity | Schema version, sample ID, scene ancestry/family, split role, immutable specification hash |
| Randomness | Named geometry/material/light/motion/optics/sensor seeds; renderer sample seed and repeat index |
| Units/conventions | World length unit, camera coordinate system, axial versus ray depth, pixel centers, matrix direction |
| Scene | Generator revision/configuration, procedural parameter realization, exact imported asset/material/HDRI hashes and rights |
| Acquisition | `N`, frame order, focus values and mechanism, camera/lens/sensor pose, time/exposure/shutter policy |
| Optics | Physical sensor dimensions/pitch, aperture/pupil, lens prescription/profile, field mapping, modeled/omitted effects |
| Sensor/encoding | Spectral/RGB basis, exposure scale, photon/electron conversion where modeled, gain/noise/ISP, channel dtype and file encoding |
| Reference | Target camera/view/time, visibility/material convention, pixel filter, focus/blur operator, retained/removed nuisances |
| Labels | Per-product semantics, coordinate frame, hard labels versus fractional coverage, validity and uncertainty |
| Reproduction | Renderer/plugins/dependencies, hardware/backend mode, complete settings, logs, file hashes, reproducibility tolerance |
| Qualification | Acceptance-suite revision, results and exclusions fixed before evaluating algorithms |

The source stack and target need **consistent coordinate conventions**, not necessarily identical apparent geometry: breathing, rail motion and subject motion are intentional acquisition effects. Save their mappings. Central-ray depth is target-view geometry, not the single true depth of every defocused/translucent observation pixel.

## 3. Scene content and sampling

Start with geometry that makes errors interpretable, then add appearance realism. Parameterize sizes in object space **and projected sensor pixels** so the same scene can stress different magnifications.

| Family | Procedural controls | Why it matters |
|---|---|---|
| Coded depth steps | Foreground bars, random background code, depth gap, texture contrast, pupil-visible hidden surfaces | Boundary leakage and information support |
| Fibers/hairs | Curves with real radius, crossings, gaps, tilt, branching, motion and material roughness | Thin-feature loss, false transparency and topology |
| Relief and repeated texture | Corrugation, printed/etched codes, ridges, grid orientation, low-texture patches | Wrong focus, aliasing and registration ambiguity |
| Biological macro analogues | Procedural leaves, petals, cuticle-like surfaces, bristles and clustered forms | Rich but controlled natural geometry |
| Industrial macro analogues | Threads, pins, PCB-like structures, scratches and layered assemblies | Distinct shape/material domain and depth discontinuities |
| Material challenges | Metallic beads, droplets, transparent shells, subsurface scattering, emissive points | Multi-path/view-dependent appearance; separate target convention |

Use actual geometry for silhouette-critical hairs, holes and fine relief. Bump/normal maps change shading but do not supply the corresponding occlusion geometry. Photographic cutouts and scanned assets are optional appearance supplements, with inherited blur/lighting and rights recorded. Neural/generated scene representations may help training diversity, but inferred/inpainted structure is not an independent factual oracle.

Factor controls into subject structure, background, illumination, magnification, optical profile, acquisition and sensor/degradation. Begin with single-factor matched comparisons, then sample interactions. Use a balanced challenge set to diagnose failure modes and a separately declared distribution for representative performance; an adversarial scene matrix does not estimate real-world prevalence.

**Proposed feasibility pilot:** eight small analytic fixtures and 24 independent procedural base scenes spanning the six families. Use a low-resolution correctness pass, then a small high-resolution convergence subset before expanding. These are budget-planning numbers, not a powered benchmark or literature-derived minimum. Retain difficult failed scenes in the generator QA record.

## 4. Focus schedule and stack length

Treat `N` as acquisition metadata, not a fixed property of the generator. The [optics review](optics-and-renderers.md#3-a-minimal-optical-contract-and-useful-derivations) derives a paraxial starting schedule. Choose focus settings from scene depth and a declared blur tolerance, then verify actual rendered support, especially at occlusions. Real motor positions and rail steps need their own calibrated mapping.

Generate a dense master sweep for a static scene and derive matched-range subsets for `N` ablations. Save exact selected indices and focus distances. A prefix shortens the depth range; it does not isolate stack length. Thin-lens inverse-depth spacing is a starting approximation, not a prescription for internal-focus macro lenses.

Separate experiments for:

- Equal focal range with different spacing/`N`.
- Equal total photon/time budget versus equal per-frame exposure. More frames otherwise provide more light or more acquisition time.
- Complete versus intentionally missing focal coverage.
- Input permutation tests for API/order sensitivity, separately from physical sweep direction. For monotonic, reverse or randomized focus acquisition under motion, reassign focus to capture times and rerender the same scene trajectory. Permuting an existing stack does not simulate a different capture order.
- Static master-sweep thinning versus independently rendered dynamic capture schedules. Subsampling a moving sequence changes the time/focus relationship.

## 5. Image formation and radiometric discipline

The intended forward process is:

```text
scene state at time t
  -> incoming spectral radiance through pupil/lens and scene visibility
  -> shutter-time and sensor-footprint integration
  -> CFA/pixel spectral response and expected photoelectrons
  -> sensor fixed-pattern effects, stochastic noise, full-well/clipping and ADC
  -> mosaiced RAW packaging with recorded CFA phase
  -> fixed versioned ISP and delivery encoding
```

Only claim the stages actually implemented. An RGB Cycles render plus noise is an RGB camera approximation; it is not automatically spectral radiance or calibrated RAW. A renderer's Monte Carlo noise is an integration error, not the camera's photon noise. Denoising a reference and calling it noise-free hides that distinction. [Scene/sensor review](procedural-scenes-and-sensors.md)

Keep exposures/throughput explicit. Record whether a sweep is radiometrically normalized or retains physical brightness changes; an equalized stack and a real camera sweep answer different questions. Simulate defocus and clipping in the correct order: clipping a sharp source first destroys the highlight energy needed to form a later blur. Disable undocumented tone mapping, auto-exposure, sharpening, clamping and denoising on authoritative linear products.

Maintain both noise-free and perturbed observations. Stable dust/fixed-pattern response should share an identity across frames; shot/read noise draws normally vary. Preserve the same underlying scene/trajectory when making paired degradation ablations. Spatially correlated or exposure-dependent noise is not equivalent to independent Gaussian RGB noise.

## 6. Motion and imperfections: explicit target tracks

The [dynamics review](motion-and-imperfections.md) develops the temporal policies in detail. Recommended default: a canonical camera and an **instantaneous scene state at declared reference time `t*`**, rendered with the selected AiF operator. Reference-frame mid-exposure is a convenient choice; it is a convention, not the unique true answer.

For a moving hair crossing a background code, the background may be visible only at times when it is defocused. Some target-state pixels may never have direct sharp support. Keep that fact in the labels. Do not define truth as a mosaic of unrelated subject poses or evaluate every output against whichever pose is closest.

| Extension | Model placement | Target choice to declare |
|---|---|---|
| Camera vibration | Continuous pose during each shutter, separately from between-frame pose drift | Stabilized canonical instantaneous target, or declared integrated-shutter target |
| Rigid/deforming subject motion | Continuous object transforms/geometry with persistent surface identities where valid | One reference time, per-frame AiF sequence, or a separately defined trajectory image |
| Rolling shutter | Row-dependent shutter times integrated with focus/pose trajectories | Canonical global-time reconstruction or preserved rolling-shutter geometry |
| Haze | Scene volume with transmittance/scattering, or explicitly simplified atmosphere model | Hazy AiF fusion versus clean AiF restoration |
| Dust | Sensor-near/lens/scene location, shared geometry or optical transmission | Defect-preserving fusion versus defect removal |

Start with static lens/sensor defects, then camera motion, then rigid subject motion, then deformation/volumes. Keep each stretch track optional and separately reported; none is needed to establish the static generator's value.

## 7. Reproducible generation and splits

Derive named seeds from a stable sample identity so adding a sensor-noise variant does not accidentally regenerate geometry. Record realized parameters as well as seeds. Pin procedural source, renderer, plugins, material definitions and binary dependencies. Reproducibility has two levels: identical bytes in a qualified fixed environment and bounded numerical differences across supported backends/hardware. Do not promise the former from a seed alone.

Use atomic per-sample publication: build in a temporary directory, check artifacts/QA, write hashes, then publish the manifest. Cache scene, clean renders and sensor variants by their full dependency signatures. A partially completed focus sweep is a failed sample until explicitly labeled as an incomplete-coverage challenge.

Split at base-scene/asset ancestry before generating crops or variants. Avoid overclaiming independence:

- New random seeds on the same generator family measure **within-family generalization**.
- Held-out shape generators/asset identities measure stronger **content transfer**.
- Held-out lens/sensor/illumination profiles measure separate **acquisition transfer**.
- Shared primitive code and generic mathematical operations can be declared common infrastructure; shared distinctive subjects, photographs, textures and environments need ancestry grouping or explicit overlap labels.

A blanket rule joining every scene that uses the same HDRI/library can make one giant group; solve that by assigning distinctive assets to partitions in advance and documenting shared infrastructure. Publish both the intended split policy and the actual dependency graph. Keep generator/metric development scenes distinct from final held-out evaluation scenes. Training only on the cheap surrogate and testing on full-3D/optical-audit scenes is a useful model-mismatch experiment.

## 8. Implementation sequence and decision gates

| Milestone | Concrete output | Gate before proceeding |
|---|---|---|
| P0: contract and analytic harness | Manifest draft, camera conventions, eight tiny fixtures, expected integrals | Independent projection/energy/occlusion agreement |
| P1: static 3D pilot | Procedural families, `N` sweep, clean stack, canonical target and central labels | Reference uncertainty, focal coverage and label consistency qualified |
| P2: practical data product | RAW-like/RGB delivery, sensor variants, atomic runner and reproducibility checks | Noise/encoding/throughput measured; no silent color transforms |
| P3: optical audit | Shared calibration scenes through a compound-lens backend and PSF/wave approximations | Identify significant model discrepancy; calibrate or limit claims |
| P4: stretch tracks | Explicit reference time, continuous shutter and motion/volume support labels | Temporal visibility/correspondence tests pass |
| P5: scale and training study | Frozen held-out families/profiles, reproducible corpus card, transfer evaluation | No ancestry leakage; matched/mismatched/real validation reported separately |

Choose production sample counts/resolution only after measuring acceptance rate, renderer variance, wall time, peak memory and storage on the pilot. For scale: **45 frames at 3840×2160, float32 RGB take about 4.48 GB decimal uncompressed for one stack alone**; targets, masks, repeats and RAW products add to it. This is arithmetic (`N*W*H*3*4` bytes), not a runtime/storage measurement. Chunking and compression policies must preserve precision.

The detailed [validation and evaluation plan](validation-and-evaluation.md) specifies numerical gates, selection/mixture oracle floors, support labels and transfer tests. The result should be described as a **model-defined, uncertainty-qualified synthetic benchmark**, with real-camera calibration claims only where measured evidence supports them.
