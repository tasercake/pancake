# Qualifying the generator and interpreting pixel errors

**Design proposal, 2026-11-17.** This extends the earlier [reference contract](../03-measurement/synthetic-ground-truth.md) with generator-specific acceptance experiments. The tests below have not been implemented or run. They are gates for a future pipeline, not evidence that one already works.

## 1. Three separate accuracy questions

1. **Implementation correctness:** does the code solve its declared image-formation model?
2. **Numerical accuracy:** how much uncertainty remains from sampling, geometry discretization, interpolation, precision and reconstruction filters?
3. **Physical adequacy:** how closely does that model match a particular real camera and acquisition regime?

An exact scene description answers none of these automatically. Ray-sampled images retain integration and aliasing error; PBRT explicitly discusses the distinction between continuous images, discrete samples and reconstruction. Use that distinction when specifying the reference pixel filter. A high-resolution image subsequently resized with an undocumented filter is a different target. [V01](https://pbr-book.org/4ed/Sampling_and_Reconstruction/Sampling_Theory)

The useful deliverable is **a precisely specified target with an error budget**. Analytic cases can sometimes have provable numerical bounds; rich path-traced references normally have empirically estimated uncertainty. No finite sample count justifies an unconditional “every pixel is perfect” claim.

## 2. Define the target before drawing any random seeds

For the first strict track, use opaque diffuse scenes and a fixed central-view pinhole target, integrated over the declared pixel footprint. Define its projection, visibility, illumination, linear color basis, exposure normalization and pixel filter. The target is a computational camera convention. Retain separate targets for:

- **Sampling truth:** ideal central-view image with the declared sensor pixel integration, no noise or ISP artifacts.
- **Delivered-image truth:** the same target passed through a fixed, documented output transform. A nominal camera look and a radiometric reference are separate products.
- **Optical-resolution truth, if needed:** a declared finite-resolution extended-focus operator, such as a controlled spatially varying focus system. Its residual PSF, distortion and visibility must be stated; there is no universal lens-preserving AiF operator.

For glossy/transmissive surfaces, keep pinhole agreement labeled as agreement with that target convention. Different focus/pupil settings can alter appearance legitimately. Spatially-varying autofocus is a real alternative optical construction, but its existence does not turn any arbitrary per-pixel focus composite into a physically realizable camera. [V02](https://imaging.cs.cmu.edu/svaf/)

Classical compositing theory already shows why depth-edge behavior matters: blending source slices can change the requested defocus to avoid halos. It studies an achievable composite under its assumptions, not a promise that any ideal target is contained in the inputs. [V03](https://graphics.stanford.edu/papers/focalstack/focalstack.pdf)

## 3. Separate fusion limitations from missing information

Save the clean noiseless observations in addition to noisy deliverables. In the static common-grid track, let `x[p]` be the target RGB vector and `y_k[p]` the clean input RGB vectors after the **declared oracle geometry mapping**. Define two diagnostic floors:

```text
selection floor[p] = min_k ||y_k[p] - x[p]||²

convex-mixture floor[p] = min_w ||sum_k w_k y_k[p] - x[p]||²
                         subject to w_k >= 0 and sum_k w_k = 1
```

These are our proposed diagnostics. The mixture uses the same weights for all color channels. They bound the performance of those restricted pixelwise operator classes on that representation, with the ground truth available to the oracle. They do **not** bound an algorithm that deconvolves, uses neighboring pixels, changes geometry, or reconstructs a scene. A large floor identifies a selection/mixture mismatch; it does not prove the target is unrecoverable. Optimizing these diagnostics over noisy images can select lucky noise, so label clean and noisy results separately.

For motion, occlusion or aperture mixing, a single dense warp may not exist. Only compute the floors on regions where the prescribed representation and mapping are valid. Keep aperture-resolved visibility/support fractions and motion/deformation labels instead of inventing a one-to-one correspondence for every observation pixel.

Use four distinct labels:

| Label | What establishes it | What it does not establish |
|---|---|---|
| Clean direct support | A declared observation/footprint sees the target surface with sufficiently small blur and contamination | Uniqueness of the complete reconstruction |
| Mixed or indirect support | Target surface contributes through a known aperture/time/pixel mixture | That one source pixel can be copied correctly |
| No direct support found | No qualifying direct sample in the generated stack | Mathematical impossibility of reconstruction |
| Certified ambiguity in a restricted model | Two distinct targets produce identical measurements, or a valid null-space argument applies under stated assumptions | Ambiguity for arbitrary materials, priors or richer sensors |

For small linear calibration scenes with known fixed geometry and a linear unknown, explicitly construct `y = A x` from independent integration. For example, let `x` represent fixed outgoing surface radiance/emission under fixed transport, or Lambertian codes under fixed direct lighting with no interreflection. Full global illumination is not generally linear in unknown surface albedos merely because geometry is fixed. Inspect singular values and response to random coded textures. Noise-scaled singular values diagnose conditioning in that model; they are not a global theorem for nonlinear visibility or unknown depth.

For a certified ambiguity control, use a restricted opaque/direct-illumination model with a target-visible coded patch that is completely hidden throughout every input's **continuous pupil/time domain**. For example, a moving occluder can reveal the patch at the declared target instant in a gap between recorded exposures while covering it during every exposure. Show that the affected columns of the independently derived measurement operator are zero, exclude indirect lighting/shadow influences of the changed code, and verify that two different codes produce different targets. In a static identical-view scene, a patch hidden from all pupil rays is generally hidden from the central-view target too, so that construction alone is not a useful witness. A finite trace merely finding no contributions establishes numerically undetected support, not an exact impossibility proof. Report unsupported inference separately from directly observable fidelity.

## 4. Minimum generator acceptance suite

| Test family | Required comparison | Failure it can reveal |
|---|---|---|
| Projection and units | Independent projection of coded planar points; focal distance, axial depth and Euclidean ray distance checked separately | Metres/mm errors, principal-point shifts, mistaken depth definition |
| Ideal geometric thin-lens limits | Pinhole appearance limit with explicit radiometric normalization; planar focus versus independent ideal camera | Wrong ray construction, normalization or focus convention; physical throughput tends to zero with pupil area, and real aberrated lenses need not focus an entire plane sharply |
| Point/line/edge optics | Analytic disk/edge integrals for the simplified model; PSF centroid/energy/width across field and depth | Radius versus diameter, gather versus scatter, kernel indexing, energy errors |
| Foreground occlusion | Two coded opaque planes, foreground bars, partial pupil visibility and hidden background texture | Masked-blur shortcuts, missing hidden geometry, incorrect compositing order |
| Sampling | Independent seeds and increased pixel/pupil/time/wavelength samples; subpixel phase sweep | Renderer noise, thin-feature dropout, temporal/angular aliasing |
| Photometry | Flat field, linear exposure sweep, bright point, unclipped HDR then sensor saturation | Display transform in linear data, clamp-before-blur, biased PSF normalization |
| Geometry labels | Reproject known surface points; compare independently rendered central-view depth/IDs | Lens-distorted RGB paired with incompatible pinhole labels |
| Sensor | Dark/flat repeats at known electron levels; empirical mean/variance, covariance and quantization | Treating render variance as shot noise, wrong gain units, independent-noise assumption errors |
| Motion | Analytic translating plane/rotating rigid object; shutter trajectory integrated independently | Frame jitter mislabeled as motion blur, wrong shutter time or flow direction |
| Integrity | Same manifest regenerates within declared reproducibility mode; interrupted sample rejected | Seed reuse, stale caches, partial sample publication |

Use **two independent implementations** for small scenes, ideally an analytic/quadrature harness and a ray tracer. Merely exporting the same buggy camera to another renderer is insufficient independence. State shared dependencies. A matched forward/inverse model can be valuable for debugging but optimistic for transfer: the image-restoration literature explicitly discusses this “inverse crime.” Keep matched-model correctness and mismatched-model performance as separate experiments. [V04](https://research.nvidia.com/sites/default/files/publications/NN_ImgProc.pdf)

## 5. Error budget and release gates

Keep uncertainty components separate:

- Stochastic integration: independent high-sample repetitions and per-region uncertainty, including bright/specular and thin-structure regions.
- Discretization: pixel/pupil/time/spectral resolution, tessellation, texture resolution and PSF/depth-table interpolation sweeps.
- Model discrepancy: thin lens versus compound lens, RGB versus spectral, measured versus synthetic PSF and sensor behavior.
- Reference convention: differences between legitimate targets, especially material and temporal policies. This is a task-definition difference, not sampling variance.

A repeat difference estimates repeatability, not bias or guaranteed coverage. Choose reference sampling so uncertainty is small relative to the **smallest difference the benchmark is intended to resolve**. Set that decision threshold after a pilot; do not silently select an arbitrary universal PSNR. Require regional and high-quantile checks as well as mean error. Preserve failures instead of masking their regions after viewing a method's output. Disable denoising, adaptive clamping and learned enhancement in the authoritative reference unless their bias is explicitly qualified.

Store float32/float64 calculations and full-float reference files where supported. EXR does not imply full-float channels: half precision, compression, channel encodings and non-finite values require explicit checks. Geometry IDs must use a lossless integer-compatible representation; filtered IDs are not instance labels. Save a geometrical coverage fraction for antialiased boundary pixels, not just a nearest-surface hard label.

## 6. Evaluation protocol for generated samples

Report the following together; use the earlier [measurement review](../03-measurement/benchmark-design.md) for broader metric/human-study qualification:

1. Linear-light MAE/RMSE and a PSNR with a fixed declared peak and evaluation mask. Do not normalize each result by its own maximum or compute undefined HDR PSNR silently.
2. Regional errors on depth boundaries, thin structures, smooth regions, high-frequency texture, saturation and disocclusion. Include signed edge profiles and known code/line recovery where available.
3. Support-stratified errors, valid output area, failed scenes, and runtime/memory versus resolution and `N`. Cropping and abstention must reduce reported coverage.
4. Appearance/perceptual scores on a fixed delivered-image transform. Keep these distinct from geometry/radiometry correctness.
5. Known-geometry-aligned fusion versus native end-to-end stacking. Alignment can isolate correspondence error on eligible opaque single-surface/common-support regions with a declared resampling filter. At finite-aperture/time mixtures, a geometry warp cannot unmix surfaces or preserve every footprint; label broader results oracle-assisted diagnostics. Native results include registration/cropping. Do not use each candidate's independently optimized warp as an invisible scoring correction.

Motion tracks name a reference time, shutter policy and camera view. Report persistent-visible, newly visible and unsupported target regions separately. Do not replace a moving subject's target with whatever pose best matches each candidate. Haze/dust tracks state whether the defect is preserved in the target or removed as a restoration task.

For statistical comparison, average and resample at independent scene/asset-family level. Frames, crops, noise repeats and focal subsets are repeated observations. Hold out geometry families, materials/textures, illumination, lens/sensor profiles and acquisition regimes for distinct transfer experiments; do not collapse all distribution shifts into one score. Select thresholds on validation data, then freeze them before test evaluation.

## Sources inspected by the coordinating review

| ID | Primary source | Evidence and limit |
|---|---|---|
| V01 | [PBRT 4e, Sampling Theory](https://pbr-book.org/4ed/Sampling_and_Reconstruction/Sampling_Theory) | Sampling/reconstruction/aliasing sections inspected. General rendering reference, not focus-stack validation results. |
| V02 | Qin, Sankaranarayanan, O'Toole, [Spatially-Varying Autofocus](https://imaging.cs.cmu.edu/svaf/), ICCV 2025 | Author project methods, optical construction and dynamic demonstration description inspected. Hardware/source claims not reproduced here. |
| V03 | Jacobs, Baek, Levoy, [Focal Stack Compositing for Depth of Field Control](https://graphics.stanford.edu/papers/focalstack/focalstack.pdf), Stanford technical report 2012-1 | Original PDF, model/compositing and halo discussion inspected. A technical report, not a new empirical benchmark for macro focus stacking. |
| V04 | Zhao et al., [Loss Functions for Image Restoration with Neural Networks](https://research.nvidia.com/sites/default/files/publications/NN_ImgProc.pdf), IEEE TCI 2017 | Original PDF, especially section on inverse crime. Motivates transfer controls; does not validate this proposed generator suite. |
