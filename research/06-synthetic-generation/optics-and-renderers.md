# Optical simulation and renderer choices

Research date: **2026-11-17**. This is a literature and implementation review, not a renderer accuracy or throughput experiment. Source records, inspected evidence and revision pins are in [optics-sources.json](optics-sources.json). Existing reference-contract research was read only to identify overlap; this review adds current implementation checks, recent optical synthesis research and a concrete camera-model selection strategy.

## Recommendation

Use **procedural 3D finite-aperture rendering as the benchmark's main image-formation path**, with a separately declared all-focus target. Add a second, faster **depth-layer/PSF path for training and parameter sweeps**, and a smaller **compound-lens optical audit track**. These paths test different approximations and should retain separate labels. This architecture is our proposal; no reviewed package was established to supply the whole pipeline unchanged.

The main engineering distinction is between **scene light transport** and **camera optics**. Blender can supply rich geometry and animation while a dedicated optical model supplies camera rays or PSFs. A sophisticated PSF generator does not restore hidden 3D surfaces absent from its source RGB-D image. Conversely, a fully ray-traced scene rendered through a thin lens does not automatically model diffraction, lens dispersion or internal focusing.

## 1. What the strongest directly relevant optics work provides

**CLDefocus — Lee, Kim and Cho, July 2026 preprint.** A particularly relevant synthesis framework combines compound-lens wavefront tracing, Zernike fitting and Debye diffraction accelerated by CZT. Its depth-layer renderer composites PSF-blurred color/opacity in linear space and includes an ISP. The released pipeline uses DPDD RAW photographs and Depth Pro estimates; occluded layer regions are inpainted. The paper explicitly identifies severe off-axis fields, complex pupils, depth errors and simplified ISP as limitations. Thus its **PSF computation is a strong reuse candidate**, while its RGB-D examples are not exact procedural 3D truth. Published counts and deblurring comparisons are covered by the dataset review; they do not establish long-stack macro accuracy. [O07](https://arxiv.org/html/2607.05837v1)

The pinned CLDefocus README exposes standalone PSF demos, Zemax-to-tabular conversion, staged resumable synthesis and customizable sources/lenses. This is unusually useful implementation evidence. Its MIT repository license does not resolve terms for upstream DPDD images, imported lens prescriptions, vendored ISP code or every dependency. The uploaded lens tables also differ by seven omissions from one conversion route; use an immutable table manifest. [O08](https://github.com/lykelee/CLDefocus/tree/06713044b693bc34fe635d26965916e447ee7ce1/datasyn)

**Efficient Depth- and Spatially-Varying Image Simulation for Defocus Deblur — Yang et al., July 2025 preprint.** The method uses unprocessing, pseudo-depth, depth-dependent PSF bases and image-space interpolation. It treats small training patches as locally invariant in sensor position, then conditions restoration on field position and ISO. This supplies a useful scalable training recipe, with reported transfer to a specific 12 MP camera. It does not establish general aperture visibility: mixing blurred full images according to output depth is a different approximation from tracing pupil-dependent occlusion. Use it for efficiency experiments, not the strict boundary reference. [O09](https://arxiv.org/html/2507.00372v1)

**DeepLens and ray–wave design.** The current repository offers geometric, coherent/pupil-propagation and Huygens PSF routes, image simulation and lens analysis. Inspection of `GeoLensRender` finds image-plane rendering at a supplied scalar object depth, not a general arbitrary-mesh scene renderer. It is therefore an optical component to connect to scene generation. Units include millimeters and wavelength-specific parameters; adapters must convert explicitly. Apache-2.0 applies to the repository, subject to its notices and dependencies. [O06](https://github.com/vccimaging/DeepLens/tree/911bb9d334fce7e20a4407c4506ac683b52ff246)

The underlying SIGGRAPH Asia 2024 ray–wave work propagates an aberrated wavefront through a refractive/diffractive camera and reports theoretical, simulation and physical validation. Its abstract supports the hybrid modeling direction, not a claim that every current library option has that validation. [O10](https://arxiv.org/abs/2406.00834)

## 2. Renderer capability audit

The following is a proposed division of labor, based on manuals/code rather than measured performance.

| Component | Verified relevant capability | Material limit | Suggested role |
|---|---|---|---|
| Blender / Cycles | Standard perspective camera samples disk/polygon pupil rays toward a focus plane; aperture-ratio scaling; camera-time transforms in inspected source. | Standard camera is geometric; this code path contains no multi-element prescription or wave propagation. | Procedural authoring and animation; first finite-pupil scene generator. |
| Mitsuba 3 | `thinlens` exposes circular aperture radius and focus distance in scene units; spectral variants support sensor response functions. | Focal-length argument is 35 mm equivalent; stock sensor reference is not a compound-lens model. | Scriptable independent thin-lens implementation and custom sensor research. |
| PBRT-v4 | Built-in `realistic` camera traces a prescription, has aperture-shape controls and radiometric ray weights. | See source audit below. | Compound-lens geometric reference/audit subset. |
| DeepLens | Differentiable optical analysis and multiple PSF models. | Scene transport/occlusion adapter required. | Lens calibration experiments and PSF-bank generation. |
| CLDefocus | Compound-lens wave PSFs plus staged depth-layer image synthesis. | Layered source scene; estimated hidden/depth content. | PSF reuse and fast surrogate comparison. |

Blender's camera source at the reviewed revision directly samples aperture points and aims rays at a focal plane. That justifies calling the ordinary perspective model a thin-lens-style model; compositor glare or lens-distortion nodes should not be interpreted as extra optical physics merely because they look plausible. Some newer custom-camera facilities exist in the inspected source, but no complete physically calibrated camera integration was audited here. The Blender manual failed direct retrieval; code evidence is used instead. [O05](https://github.com/blender/blender/blob/2e8d044d74c4f60964c41c6ba427305d95b1bfb9/intern/cycles/kernel/camera/camera.h)

For Mitsuba, derive actual FOV from physical sensor dimensions and use `fov`/`fov_axis` explicitly. Do not pass a real macro lens's millimeter marking as though the documented 35 mm equivalent field setting independently fixes magnification. Spectral light transport alone does not add lens dispersion. The source license is BSD-3-Clause text, despite GitHub's metadata returning `NOASSERTION`. [O04](https://mitsuba.readthedocs.io/en/latest/src/generated/plugins_sensors.html), [license](https://github.com/mitsuba-renderer/mitsuba3/blob/aa01500f57770a4ef0b067693cae08b3e8a2ac73/LICENSE)

### PBRT-v4: useful but narrower than the name “realistic” suggests

The file format distinguishes perspective `lensradius`/`focaldistance` from realistic-camera `aperturediameter` in **mm** and `focusdistance` in **m**. It also offers independent random seeds and error-versus-sample reporting. [O02](https://pbrt.org/fileformat-v4)

At the pinned revision, `RealisticCamera` stores spherical curvature, spacing, scalar `eta` and aperture radius. Refraction does not use wavelength; focusing assigns the final lens–film spacing via a thick-lens approximation. **Inference from this implementation:** the stock path omits lens chromatic dispersion, diffraction and internal group-motion mechanics. It should not be presented as an exact commercial internal-focus macro lens. Prescription-dependent geometric aberrations, vignetting and pupil clipping remain useful. [O03](https://github.com/mmp/pbrt-v4/blob/20e41fd83789f12550e2c31b2047898c9e20e46c/src/pbrt/cameras.cpp)

Kolb, Mitchell and Hanrahan's SIGGRAPH 1995 model remains the foundational distinction: a camera must map scene radiance through lens geometry onto film irradiance, rather than merely project points. Their primary abstract describes sequential lens surfaces, stops and distributed ray tracing. This gives the conceptual basis for a camera module replaceable independently of the scene integrator. [O01](https://graphics.stanford.edu/papers/camera/)

## 3. A minimal optical contract and useful derivations

The equations in this section are **our paraxial derivations/design guidance**, under an ideal thin lens, positive real object distance and no aberrations. They are not a claim about any renderer's exact parameterization.

Let `f` be focal length, `z` object distance from the thin lens, `s` object distance at the selected focus setting, `D` physical pupil diameter and `p` sensor pixel pitch, all in a consistent length unit. The ideal image distance is

`v(z) = f*z/(z-f)`.

With the sensor at `v(s)`, similar triangles give the defocus-circle diameter

`c(z;s) = D * abs(v(s)-v(z))/v(z)`

`         = D*f*s/(s-f) * abs(1/z - 1/s)`.

The pixel diameter is `c_px = c/p`. This makes **inverse-depth spacing** a useful initial schedule: for a fixed `s` the paraxial circle size changes linearly with inverse depth. Across a macro sweep the prefactor also changes, so choose focus locations numerically from the actual camera model and enforce a desired maximum minimum blur over the target depth range. Do not assume evenly spaced focus-motor commands correspond to evenly spaced optical depths.

A candidate generation rule is: select the smallest `N` for which every eligible target surface sample satisfies `min_k c_px(z;s_k) <= c_max`, then generate intentional incomplete-coverage variants separately. The eligibility/support test must include occlusion and clipping; satisfying a blur formula alone does not prove observable detail. Preserve the full sweep for controlled thinning studies.

At close distances, magnification in this ideal model is `m=f/(s-f)`. Thus the sensor-to-lens distance, perspective scaling and effective image-side cone differ substantially from the infinity-focus approximation. For a symmetric unit-pupil-magnification lens, the familiar approximation `F_eff ≈ F*(1+m)` illustrates why macro exposure/diffraction changes cannot be captured solely by nominal `F=f/D`. It is not valid for every internal-focus design; pupil magnification and calibrated numerical aperture are safer physical quantities.

For an ideal circular pupil, scalar paraxial diffraction has a first-zero Airy **diameter** approximately `2.44*lambda*F_eff`. For example, at `lambda=0.55 µm`, `F_eff=8`, it is about `10.7 µm`, or `2.7` pixels at `p=4 µm`. This is a scale illustration, not a perceptual-resolution threshold. **Shrinking the pupil to zero is an abstract geometric target convention, not the diffraction-free limit of a physical camera.**

A camera profile should therefore include: physical sensor size/pitch; pupil geometry and placement; focus mechanism; lens-to-sensor pose per setting; wavelength-dependent model where present; shutter timing; pixel integration filter; radiometric response/throughput; and an explicit list of omitted physics. Track all of these independently from display tone mapping.

## 4. Diffraction and aberration: where to spend complexity

**Start with finite-aperture geometry**, because thin structures and depth-edge visibility already challenge fusion. Next measure aberrations with a field × depth × focus × aperture × wavelength PSF bank. Keep PSF centroids or corresponding chief-ray mappings: recentering every PSF silently removes distortion/lateral color and can conceal breathing. Validate interpolation at held-out optical configurations. These are implementation recommendations, not a published accuracy guarantee.

Use wave optics when blur/detail is near the diffraction scale, when testing small apertures, or when geometry-only point spread differs materially from the measured lens. Miora et al.'s 2024 PSF study shows why “FFT diffraction” is not a correctness certificate: finite windows, sampling and wraparound can violate energy behavior; high-NA microscopy also requires vector/polarization-aware models. Its microscopy setting is adjacent evidence, not direct validation for ordinary macro cameras. [O11](https://arxiv.org/html/2301.13515v2)

A global Airy convolution after finite-aperture scene rendering can serve as an explicitly approximate baseline. It should not be the sole truth model for strongly vignetted, aberrated or pupil-occluded boundaries. For the strongest audit subset, compare wave/PSF approximations with an independent propagation method and convergence in wavelength, pupil samples, propagation window and sensor grid.

**Flare is distinct from defocus.** Hullin et al.'s SIGGRAPH 2011 work models unintended lens paths including ghost reflections, aberrations and coatings. Treat it as a separate optical module when needed; standard scene bloom is not equivalent. The author page's PDF redistribution restrictions also illustrate why public access to a paper does not supply a software/data license. [O13](https://resources.mpi-inf.mpg.de/lensflareRendering/)

## 5. Focus changes, rail motion and target view

Li and Nguyen's 2019 fixed-lens study explicitly links moving-lens stacks to perspective inconsistency and proposes moving the camera body relative to a fixed lens, followed by scale/shift calibration. Its experiments concern small-object reconstruction, not universal stacking superiority. [O12](https://arxiv.org/html/1912.03005v1)

**Proposed tracks:**

1. Fixed projection, ideal focus-only sweep: isolates fusion and defocus boundaries.
2. Fixed camera housing, calibrated internal focus changes: vary optical mappings, pupils, aberrations and breathing coherently.
3. Rigid camera/lens rail translation: hold lens setting fixed, move physical pose and recalculate visibility.
4. Fixed lens, moving sensor: separate reference case motivated by the fixed-lens literature.

Do not combine these into one generic random zoom augmentation. Geometry, throughput and visibility consequences differ. A single homography cannot generally correct rail parallax in a nonplanar scene. Store frame-wise central-ray projection mappings, visible surface IDs and explicit invalid/disoccluded regions; finite-pupil mixed pixels require richer labels than one 2D flow vector.

## 6. Recent directions to watch, with evidence limits

- **Spatially-Varying Autofocus, ICCV 2025:** Lohmann optics and a spatial light modulator permit local optical focus control. It motivates a finite-pupil extended-focus target convention, but does not make all AiF definitions equivalent. Main synthesis covers target implications. Project page inspected. [O14](https://imaging.cs.cmu.edu/svaf/)
- **DiffRayve, September 2026 preprint:** the abstract proposes differentiable shooting/bouncing rays with geometric and physical fields, including polarized compound refractive/diffractive optics. A promising research direction; full implementation, license and practical scene coupling were not audited. [O15](https://arxiv.org/abs/2609.16404)
- **Non-contact lens metrology from optical flow and differentiable ray tracing, October 2026 preprint:** fits refractive surfaces from patterned-mask distortion and reports commercial-lens validation. Potential calibration support; abstract-only evidence does not establish non-invasive recovery of an assembled proprietary macro lens. [O16](https://arxiv.org/abs/2610.17515)

## 7. Concrete decisions before implementation

- Build a camera adapter with a shared physical-unit schema. Record projection conventions and center-of-perspective definitions; reproduce a plane, point grid and two-depth occluder across backends before rendering rich scenes.
- Keep scene content identical when comparing camera backends, but do not assume BSDFs and texture filters transfer identically. Use simple diffuse calibration scenes for cross-renderer comparisons first.
- Pin code **and** prescriptions/material catalogs; preserve focus-specific configuration. Do not infer that the same nominal focal length/f-number means the same PSF.
- Evaluate optical error separately from fusion error. Store central-ray target, noiseless finite-aperture stack, delivered sensor stack, depth/visibility and uncertainty products.
- Require measured lens profiles for a claim about a named real camera. Public patent prescriptions and synthetic designs support controlled optical diversity, not an exact profile of an off-the-shelf lens.
- Benchmark runtime and convergence on a small representative matrix before choosing the production backend. No runtime recommendation in this review is based on a local benchmark.
