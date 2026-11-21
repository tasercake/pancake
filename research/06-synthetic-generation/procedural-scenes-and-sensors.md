# Procedural scenes, sensors, and transfer to real cameras

Reviewed **2026-11-17**. This review examines generation infrastructure rather than re-cataloguing existing focus-fusion datasets. Existing research was read to identify overlap, then primary papers, current official documentation and selected source files were searched independently. Source records, inspection limits and immutable software revisions are in [procedural-sources.json](procedural-sources.json). No renderer, sensor simulation or training experiment was run.

**Recommendation:** use a small deterministic scene-description layer, with Blender/BlenderProc as the first scene-authoring backend and selected Infinigen Nature generators as optional assets. Keep sensor formation and delivery processing separate. Add an ISET3d/ISETCam spectral calibration branch when camera fidelity becomes a research question. Neither a general scene generator nor an RGB renderer alone supplies a validated RAW camera simulator.

## 1. Infrastructure: what can actually be reused?

| System | Verified contribution | Work still needed for this project |
|---|---|---|
| **BlenderProc2** (JOSS 2023) | Python orchestration for loading/generating scenes, pose sampling, collisions, materials/lights, camera intrinsics, RGB/depth/normal/segmentation output. Official camera utility exposes DOF controls. | Focus scheduling, canonical target definition, aperture-aware labels, precise sensor/ISP formation, convergence checks and macro validation are project-specific. [P03–P06] |
| **Infinigen Nature / 1.x** (CVPR 2023) | Procedural shape/material rules include plants and beetles; underlying geometry is available, making this a strong natural-asset supplier. | Tune assets to macro image scale; add task-specific filament and boundary controls; freeze geometry across a sweep. Full generated landscapes are unnecessary overhead for a specimen crop. [P01, P02] |
| **Infinigen 2.0 preview** | Current official documentation advertises controllable composition via `procfunc`. | The same page says it is alpha and currently covers limited indoor assets/materials and simple rooms. Do **not** assume Nature feature parity or migrate the project to it just because it is newer. [P02] |
| **Kubric** (CVPR 2022) | Modular Blender/PyBullet workers, physics, image/video annotations and reusable dataset patterns. | Its object-video abstraction is useful for later motion work; confirm camera and label semantics. The paper's historical unsupported-feature list is not a reliable inventory of current code. [P07, P08] |
| **ISET3d + ISETCam** | PBRT-based 3D rendering connects to spectral sensor and image-processing structures; source repositories are MATLAB-based. ISETCam represents CFA, spatial sampling, well capacity, noise and fixed-pattern terms. | Requires compatible renderer, assets, MATLAB environment and camera calibration. It is a reference architecture/possible second backend, not a drop-in Blender image postprocessor that recovers missing spectra. [P09–P12] |

These tools occupy different layers: Infinigen supplies procedural content; BlenderProc supplies scene/camera orchestration; Kubric adds dynamic-scene dataset infrastructure; ISET supplies quantitative imaging stages. Combining all four by default would add substantial integration risk. Begin with one host and explicit backend interfaces.

### Current-version and default-setting traps

BlenderProc's inspected `DefaultConfig.py` at `76bd8b82ee19489843578ff045298642197bf883` sets **8-bit PNG, Filmic, Intel denoising, and zero glossy, transmission and volume bounces**. These are software defaults, not recommended truth settings. Override and record output precision/color encoding, denoising, bounce limits, sampling, clamping and geometry simplification. Save linear floating-point render data before display transforms. Changing a single configuration file does not prove that a returned NumPy array or written EXR follows the intended color contract; verify it with constant/ramp scenes. [Pinned code](https://github.com/DLR-RM/BlenderProc/blob/76bd8b82ee19489843578ff045298642197bf883/blenderproc/python/utility/DefaultConfig.py) [P04]

BlenderProc documents different depth and distance outputs: camera-axis depth and Euclidean range are distinct, with different antialiasing options. Its renderer API exposes motion blur and rolling shutter. The presence of these APIs does not establish the accuracy of a particular animated camera/geometry configuration. [Renderer documentation](https://dlr-rm.github.io/BlenderProc/_modules/blenderproc/python/renderer/RendererUtility.html), [camera documentation](https://dlr-rm.github.io/BlenderProc/blenderproc.python.camera.CameraUtility.html) [P05, P06]

Kubric's 2022 paper lists depth of field and motion blur among unsupported features. However, inspected 2026 renderer code includes a `motion_blur` argument; the inspected core camera file has no dedicated focus/aperture parameters. This is evidence for partial evolution, **not** a completed DOF-camera audit. Integrating custom Blender cameras may be possible but remains untested here. [Paper](https://arxiv.org/html/2203.03570v1), [pinned renderer](https://github.com/google-research/kubric/blob/61f2422c84bab75006df33c6989e0b483db3ccfe/kubric/renderer/blender.py), [pinned cameras](https://github.com/google-research/kubric/blob/61f2422c84bab75006df33c6989e0b483db3ccfe/kubric/core/cameras.py) [P07, P08]

## 2. Procedural content should expose the failure variables

The following scene grammar is a **proposal**, not a published dataset recipe or an assertion that a tool implements it already.

| Family | Procedural representation | Parameters worth retaining |
|---|---|---|
| Controlled targets | Analytic planes, solids, apertures, coded textures and slanted edges | Physical depth, edge contrast, spatial frequency, relative phase to pixel grid, exact topology |
| Hairs, fibers, antennae | Explicit curves with physical cross-sections or resolved tube meshes | Radius, taper, curvature, crossings, root attachments, absorption/roughness; include circular and flattened cross-sections |
| Rough natural surfaces | Displaced meshes and repeated procedural structures | Feature height/spacing relative to depth of field, roughness, color correlation, holes/undercuts |
| Leaves, petals, insect-like parts | Parametric surfaces plus attached fine geometry | Thickness, edge serration, venation, local curvature, appendage density; separate opaque and transmissive material branches |
| Artificial detail | Wires, screws, textile loops, printed codes and micro-grids | Repetition, shallow slopes, low contrast, reflectance and known line widths |
| Difficult appearance | Smooth specular beads, rough metal, glass, droplets, thin sheets | Roughness, refractive index, transmission/absorption and lighting geometry; require separate target semantics |

Parameterize sizes in metres and record their **projected** widths in pixels as well. Arbitrarily scaling a human-scale asset into a macro scene does not preserve physical roughness, wavelength-scale structure or manufacturing detail. A visually plausible chitin shader is not measured insect reflectance.

Use procedural geometry for features whose existence or topology is being scored. Normal maps can change shading while leaving geometric depth and silhouette flat; photographic alpha cards can encode existing blur and unknown hidden geometry. They remain useful texture/appearance stressors when explicitly labeled as such. Keep an independent simple mathematical generator for lines, circles and two-depth boundaries so the evaluator is not tested exclusively on one scene backend.

Freeze tessellation and camera-dependent level of detail across all N frames and the sharp target. Infinigen's paper describes camera-aware resolution scaling, instancing and separate geometry-label extraction. For a focus sweep, regenerating those decisions per frame would be an avoidable confound. Its paper also cautions that ordinary Blender render-pass labels become problematic under defocus, motion and translucent/volumetric appearance. [Original paper, method and Appendix C](https://arxiv.org/html/2306.09310v1) [P01]

A geometry map answers “what surface intersects this declared reference ray?” A finite-aperture pixel can receive rays from several surfaces and depths. Therefore preserve both central-view geometry labels and separate aperture/time contribution data on a calibration subset. Never interpret one depth/instance integer as a complete explanation of a defocused pixel. This is a deduction from the image-formation model, not a capability claim about Infinigen.

## 3. Make a sensor measurement before making a TIFF

ISET's useful architectural distinction is **scene radiance → optical-image irradiance → sensor response → image processor/display**. Current ISETCam documentation treats CFA, pixel integration, well capacity, shot/read noise, dark signal and pixel-response nonuniformity as explicit stages; its noise documentation separates temporal noise from spatially fixed DSNU/PRNU. [Sensor model](https://github.com/ISET/isetcam/wiki/Sensor), [noise model](https://github.com/ISET/isetcam/wiki/Sensor-noise-model) [P10–P12]

A proposed minimal generative model, using electrons before conversion, is:

`mu[p] = integral_pixel integral_time integral_lambda E(x,t,lambda) * QE[p,lambda] * lambda/(h*c) d(lambda) dt dA`

`n[p,k] ~ Poisson(mu[p,k] + dark_rate[p] * exposure[k])`

`raw[p,k] = quantize(clip(gain[k] * (n[p,k] + read_noise[p,k]) + bias[p] + row_noise[row(p),k]))`

Here `E` is spectral irradiance in W m^-2 per wavelength unit; the integral must use consistent wavelength units, `QE` includes the declared CFA response, and pixel area is in m². `mu` is an expected electron count. This simplified model omits many possible electronics effects; its exact gain/ADC placement must match the profile. Fixed sensor maps are reused across the burst, while temporal random draws change per frame. Black offset subtraction and clipping order affect the noise distribution and must be explicit.

For an initial RGB renderer, use an explicitly approximate linear RGB-to-camera response and heteroscedastic RAW model. Three RGB channels cannot uniquely recover a spectral radiance distribution or predict arbitrary CFA/illuminant metamerism. Do not describe this branch as full spectral camera simulation. A later spectral backend should provide wavelength-resolved sensor irradiance directly rather than re-interpreting display RGB as physical sensor data.

Generate three deliverables from the same latent scene:

1. **Clean optical linear stack and declared sharp target:** isolates focus fusion from sensor processing.
2. **Noisy mosaiced sensor stack:** evaluates RAW workflows; save clean expected sensor values too.
3. **Delivered RGB stack and target through a fixed declared ISP:** evaluates existing stackers that accept TIFF/JPEG. Preserve CFA phase, crop, black/white levels, color matrices, demosaic, white balance, tone curve, sharpening and compression settings.

The target's noise policy is part of the task: a clean target asks a noisy-input method to denoise as well as fuse. For strict fusion-only tests, use clean input or explicitly quantify expected noise. Clean target RGB must be produced in the same evaluation color space/grid, with a declared target ISP; comparing noisy mosaics to an arbitrary rendered sRGB target mixes several tasks.

Brooks et al.'s **Unprocessing** models how gain, white balance, color correction, gamma, tone mapping and demosaicing affect synthetic RAW training. This is a useful ISP prior. Its inversion of Internet photographs does not recover exact original RAW, unclipped highlights, scene spectra or focus geometry. Starting from procedural linear radiance avoids needing that inversion. [CVPR 2019 paper](https://arxiv.org/html/1811.11127v1) [P13]

### Which noise literature is useful now?

| Source | Reusable idea | Appropriate limit |
|---|---|---|
| **ELD**, CVPR 2020 / TPAMI extension | Shot noise plus heavy-tailed read noise, row and quantization components; flat/bias-frame characterization. | Denoising success does not validate macro stacks. The README explicitly withholds the full model/calibration under a business-license restriction, although baseline models and parameters were later released. Audit the actual reused files. [P14, P15] |
| **Noise Modeling in One Hour**, CVPR 2025 | Hypothesized shot-noise gain plus sampled dark frames after dark-shading correction; useful lower-effort training augmentation. | Its robustness argument targets denoiser training. Hypothesized gain is not a measured camera profile and cannot certify photometric fidelity for an evaluation benchmark. [P16] |
| **SNIC**, Dec. 2025 preprint | Calibrated per-channel mean–variance behavior and dark-frame options; RAW and TIFF synthesis; reports scene-dependent phone RAW processing. | Claims are author-reported; no replication here. Some analysis/formula wording needs independent checking; use it as a calibration lead, not an authoritative sensor standard. [P17] |
| **EMVA 1288 Release 4.0 Linear**, 2021 | Formal measurement vocabulary/procedures for linear response, temporal noise, dark current, nonuniformity and defect pixels. | Its assumptions exclude some processing and noise correlations; following selected ideas is not a claim of standards conformance. [P18] |

[ELD paper](https://arxiv.org/html/2003.12751v2), [ELD release caveat](https://github.com/Vandermode/ELD#quick-start), [2025 method](https://arxiv.org/html/2505.00045v1), [SNIC methods](https://arxiv.org/html/2512.15905v1), [EMVA official PDF](https://www.emva.org/wp-content/uploads/EMVA1288Linear_4.0Release.pdf).

For calibrated evaluation, capture paired flats over exposure levels plus dark/bias series at relevant gains, exposure times and temperatures. Estimate mean–variance curves from frame differences so fixed texture does not inflate temporal variance. Inspect residual histograms, row/column covariance, spatial power spectra, persistent defects and temporal correlations, then validate on textured scenes. Do not infer all noise from the ISO number. Retain calibration captures and uncertainty alongside the fitted profile. This protocol is a project proposal informed by P14/P16/P18.

## 4. Imperfections belong at their physical stage

| Perturbation | Proposed insertion point | Useful saved truth |
|---|---|---|
| Dust/hot pixels on the sensor | Persistent sensor map, possibly with exposure-dependent dark response | Physical sensor coordinates and affected fraction |
| Dust/scratches near pupil/optics | Aperture transmission/scattering model, not a pasted opaque speck | Position, size, opacity/scatter and focal/aperture dependence |
| Atmospheric haze | Participating medium before optics | Extinction/scattering, depth, lighting, and a separate declared haze-preserving or dehazed target |
| Subject dirt/dust | Scene geometry/material | Surface coordinates and appearance included in the subject target |
| Shake/vibration | Camera pose as a function of shutter time | Trajectory, exposure interval, reference pose and overlap |
| Inter-frame drift/breathing | Per-frame optical/camera parameters | Known geometry and correspondences where visible |
| Moving/deforming subject | Time-dependent geometry and shutter integration | Reference-time state, visibility and correspondence validity |
| Exposure flicker/auto processing | Light/exposure or ISP stage, according to cause | Frame-wise factors and which target convention applies |

ISETCam's README mentions wavefront-based optics with apertures, scratches and dust; this is a promising specialist path, but no general 3D lens-contamination implementation was audited or exercised here. [Official README](https://github.com/ISET/isetcam) [P10]

Keep paired corruption-free and corrupted versions where practical. Dust, haze and motion can be scene content, degradation or reconstruction targets depending on the declared task. Save policies rather than silently removing all imperfections from the target. Dynamic target definitions are treated separately in the motion review.

## 5. Sampling, reproducibility and transfer

**Proposed seed hierarchy:** dataset split → scene family → geometry/material/illumination → optics/acquisition → sensor profile → temporal noise → renderer sampling. This makes it possible to change one factor while holding others fixed. Record all seeds, procedural-generator versions, resolved parameters, asset hashes, simulation caches, renderer build/device, spectral/RGB basis and color configuration. A seed by itself is not a portable dataset specification.

Separate Monte Carlo rendering variance from simulated shot/read noise. First render a sufficiently converged latent image, assess independent repeats, then sample sensor noise. Otherwise an algorithm may learn or be penalized for renderer artifacts presented as camera noise. Optional shared random numbers can improve paired ablations, but independent repeated renders are still needed to estimate the reference error floor.

Hold out more than random seeds. Evaluation should include unseen generator templates/topology families, materials, illuminants and camera profiles, plus an independently authored/renderer-checked challenge subset. Sample variations within a family to measure controlled effects; do not claim thousands of seeds are thousands of independent real-world subjects. Keep realistic-prior tests separate from deliberately adversarial stress tests.

Domain randomization has foundational evidence for transfer in **object localization**, including Tobin et al. (2017). That evidence does not establish transfer for exact-pixel macro fusion. Use plausible calibrated ranges for representative tests, broad randomization for training robustness, and independent real stacks for transfer validation. [Original paper abstract](https://arxiv.org/abs/1703.06907) [P19]

## 6. Assets and licensing

The inspected Infinigen core is BSD-3-Clause, but its `.gitmodules` references an `infinigen_gpl` submodule. BlenderProc is GPL-3.0; Kubric, Apache-2.0; ISET3d and ISETCam, MIT. These statements describe inspected repository labels/license files, not a complete dependency or generated-output rights audit. Keep code licensing separate from external textures, meshes, pretrained weights, camera data and calibration captures. [Infinigen license](https://github.com/princeton-vl/infinigen/blob/53033dd55c7191cbc436be6ffbb63c026935a509/LICENSE), [submodules](https://github.com/princeton-vl/infinigen/blob/53033dd55c7191cbc436be6ffbb63c026935a509/.gitmodules), [BlenderProc](https://github.com/DLR-RM/BlenderProc), [Kubric](https://github.com/google-research/kubric), [ISET3d](https://github.com/ISET/iset3d), [ISETCam](https://github.com/ISET/isetcam). [P02–P03, P08–P10]

Poly Haven's asset license page explicitly places its HDRIs/textures/models under CC0, making a small pinned seed-asset collection attractive. Its website/API access terms are separate; use an authorized acquisition route and retain source/license/hash records. Assets can still contain baked appearance or finite spatial resolution inappropriate for macro. An HDRI can diversify lighting without serving as a close-up geometric subject. [Official license and terms](https://polyhaven.com/license) [P20]

## 7. Suggested implementation sequence

1. **Deterministic micro-scenes:** planes, coded boundaries, explicit fibers, opaque diffuse materials; linear floating-point optical data and a fixed canonical view.
2. **One production scene backend:** a pinned Blender/BlenderProc configuration with documented thin-lens capabilities; extract geometry labels separately from optical observations. Add only selected stable Infinigen Nature assets after macro-scale inspection.
3. **Sensor branch:** calibrated or explicitly approximate RAW formation, persistent defect maps and fixed ISP. Retain clean latent products and noise repeats.
4. **Transfer branch:** independent lens/spectral backend on small shared scenes, held-out scene families and measured camera/real-stack checks.
5. **Motion/volume/contamination:** add individually, then cross selected factors after each model has independent tests.

This is an engineering recommendation. None of the reviewed frameworks eliminates the need to validate correspondence, sampling error, optics, materials and sensor profiles for this particular benchmark.
