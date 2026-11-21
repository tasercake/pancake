# Motion, imperfect capture, and the meaning of sharp ground truth

Research date: **2026-11-17**. This review extends the existing [synthetic reference contract](../03-measurement/synthetic-ground-truth.md) with dynamic-scene literature and an implementable temporal/degradation policy. It is a literature and design review: **no scenes, degradations, stackers, or metrics were executed**. Source IDs and evidence limits are in [motion-sources.json](motion-sources.json). All numbered benchmark tracks and validation gates below are proposals.

## Main recommendation

For a moving subject, define ground truth as **the sharp scene at one declared time, seen from one declared camera**, rather than “the sharp pieces found anywhere in the burst.” Render that target directly from the simulation. Other frames can help recover its content only where their scene content is compatible with that target. Store per-frame sharp auxiliary renders as well, so reference-frame policies can be changed without silently changing geometry.

A deterministic target can exist even when it is not uniquely recoverable from the inputs. The benchmark must distinguish those statements. If a wing reveals a random code only at the target instant, while every recorded exposure sees another surface or obliterates its detail, knowing the simulator's code does not make recovery a reasonable fusion requirement. This is a central dataset-design issue, not an edge case that more accurate optical flow automatically solves.

Start with static scenes and inter-frame rigid camera movement; then add within-exposure camera movement; then controlled object motion. Keep haze and dust as separate matched ablations. Crossed effects become useful after each component passes its own checks.

## 1. What the most relevant prior art actually supplies

| Source | What can be borrowed | Important boundary |
|---|---|---|
| **WHU-MFM / DSAF-Net (2024)**, Chen et al. [M01](https://www.sciencedirect.com/science/article/pii/S1566253523004414), [generator](https://github.com/PeimingCHEN/WHU-MFM-Dataset/tree/818992fdc05896768eeb6012aa9b74e00eb4a27a) | A particularly relevant public Blender recipe: five differently focused, misaligned images, camera matrices, depth/defocus/object masks, and a separate DOF-off AiF image for every camera pose. README reports 3,000 scenes at each of two resolutions. | Inspected script changes camera pose between frames, with random translation/rotation. It does not implement a continuous shake trajectory during exposure in its Python code. Per-pose AiF targets make target-camera choice explicit. Assets include collected Sketchfab meshes; no repository license file was present in the inspected tree. Do not assume the source models or outputs are freely redistributable. |
| **AiDiF / DYN-MVMF (2026)**, Li, Liu and Gong [M02](https://doi.org/10.1117/1.JEI.35.4.041407), [author repository](https://github.com/North-Li/DYN-MVMF/tree/098217d98d11165931498781fb45299449c243cb) | Direct dynamic multi-focus prior art: the publisher abstract describes simultaneous multi-view, multi-focus acquisition and a Blender dynamic dataset. Simultaneity is a useful counterfactual for separating temporal errors from focus fusion. | This changes the acquisition problem: multiple synchronized views introduce parallax instead of sequential scene change. Full paper and archive contents were not inspected. The public tree contains only a README and Baidu dataset link; its terms prohibit commercial use and request author permission for research/education. Counts, exact target construction and generation code remain unverified. |
| **Dynamic HDR**, Kalantari and Ramamoorthi (2017) [M03](https://cseweb.ucsd.edu/~viscomp/projects/SIG17HDR/PaperData/SIGGRAPH17_HDR_LoRes.pdf) | A useful temporal convention: align the output to a designated medium-exposure reference. The dataset combines a static bracket/reference with moving-subject alternate exposures. | A carefully constructed reference policy is needed even in adjacent fusion tasks. The static-for-dynamic capture recipe is not an exact capture of arbitrary continuous motion, and does not establish focus-stack recoverability. |
| **Deep Dynamic MEF (2023)**, Tan et al. [M04](https://doi.org/10.1109/TIP.2023.3315123), [author code](https://github.com/Tx000/Deep_dynamicMEF) | Another explicit “static-for-dynamic” strategy for references in moving multi-exposure pairs. | Abstract/author README evidence only; do not infer a fully procedural renderer or exact scene truth. Baidu distribution was not downloaded. |
| **RSBlur (ECCV 2022)**, Rim et al. [M05](https://arxiv.org/html/2202.08771v3) | Paired real blurred images and short-exposure sequences support analysis of temporal integration, saturation, noise and ISP. The authors use the center short exposure as the sharp target and show why naive video averaging misses real blur. | A short exposure is a practical sharp reference with residual exposure/noise, not a zero-duration physical measurement. This is deblurring evidence, not a focus-stacking result. |
| **Rolling Shutter Motion Deblurring (CVPR 2015)**, Su and Heidrich [M06](https://www.cs.ubc.ca/labs/imager/tr/2015/RollingShutterMotionDeblurring/) | One continuous camera trajectory yields different exposure segments for different scanlines. | Inspected author project abstract, not code or full methods. It motivates joint simulation of rolling shutter and blur; it does not validate our proposed trajectories. |
| **MPI-Sintel (ECCV 2012)**, Butler et al. [M07](https://files.is.tue.mpg.de/black/papers/ButlerECCV2012.pdf), and **TAP-Vid (NeurIPS 2022)**, Doersch et al. [M08](https://papers.nips.cc/paper_files/paper/2022/file/58168e8a92994655d6da3939e7cc0918-Paper-Datasets_and_Benchmarks.pdf) | Geometry-linked motion labels, explicit occlusion/invalidity, controlled rendering passes, and metrics that distinguish tracking and visibility. | Flow/point tracks are geometry truth. They are not a complete inverse model for pixels integrating several times, surfaces or pupil rays. Sintel also cautions that cinema assets/lighting need not obey natural scene physics. |

The search found direct dynamic-focus work and reusable adjacent protocols, but did **not** establish an openly licensed, complete procedural macro renderer that already combines arbitrary N, realistic optics, dynamic truth, sensor/ISP, imperfections and support labels. This is a scoped search result, not proof that no such system exists.

## 2. Make time, viewpoint and desired degradation explicit

Define a target contract independently of any algorithm:

`target = (camera/reference view, time policy, spatial/pixel filter, optical appearance policy, retained nuisance effects, radiometric encoding)`.

Recommended initial image: noise-free, pixel-filtered central-ray render of opaque diffuse geometry at `t_ref`, with a fixed reference camera. This removes defocus and temporal blur by definition. A finite-aperture or glossy appearance track needs its own contract, as discussed in the earlier reference document.

| Temporal contract | Exact simulation definition | Appropriate use | Consequence |
|---|---|---|---|
| **Frozen instant, reference frame** | Render scene state and reference camera at a specified `t_ref`, without temporal integration. | Main dynamic reconstruction track. Set `t_ref` to the midpoint of a declared reference frame exposure, not an output-selected time. | May require reconstructing unavailable detail; expose support strata. |
| **One target per recorded frame** | Save a sharp instantaneous target at each frame's declared reference time and pose. | Reusable dataset, method with a caller-selected base frame, future video stacking. | The selected output target must be fixed before scoring. Reporting the best score over all times rewards temporal mistakes. |
| **Exposure-integrated, extended focus** | Integrate the declared sharp optical model over the reference exposure/shutter. | Isolating defocus fusion while retaining true motion blur. | “All in focus” here still contains motion blur. It answers a different task from a frozen instant. |
| **Rolling-shutter sharp target** | Zero-duration spatial samples at each row's own scan time. | Isolating defocus while retaining scan distortion. | The target is a temporal mosaic; it does not depict one instant. |
| **Canonical pose/deformation** | Render all subjects in a named rest/canonical configuration. | A separate inverse-graphics or canonicalization study. | Adds pose reconstruction; a conventional focus stacker should not be judged against it without being tasked to do so. |
| **Per-object selected time** | Assign an explicit time to every object and recompute complete scene transport and visibility. | Artistic composite/deghosting study. | Can assemble a scene that never existed. Copy-pasting sharp object pixels leaves inconsistent occlusions, shadows and reflections. |

A stacker allowed to choose a base frame should return that frame/time identity. Evaluate this choice as a distinct policy or score against its corresponding pre-rendered target, with selection cost and support reported. Do not let a scorer infer a favorable base after seeing the output.

### A concrete moving-wing example

At `t_ref` an insect wing is open. At later focus settings it closes and hides a patch of abdomen. The frozen-instant reference contains the open wing and that patch. If earlier frames saw the same patch sharply, surface correspondences can help. If all inputs saw the patch only blurred or partly through a defocused wing, the problem needs reconstruction. If it was hidden for every input exposure, its texture is unobserved. An algorithm that preserves the reference wing but leaves uncertain abdomen detail blurred may be an effective deghosting system, even while it has substantial full-target error.

Conversely, a composite with two wing tips from different times can be sharp and have high local contrast while describing no valid chosen scene. Evaluate geometry/duplicate structures and time consistency, not sharpness alone.

## 3. Simulate one acquisition process rather than independent filters

A useful schematic per sensor pixel `p`, channel `c`, frame `k` is:

`expected_electrons[k,p,c] = ∫ shutter[k,row(p)](t) ∫ pixel_p ∫ pupil ∫ wavelength response_c(λ) × weighted_radiance(ray(k,q,a,t,λ), scene(t)) dλ da dq dt`.

Then draw photon/read/dark/electronic noise under the sensor model, clip/quantize at the appropriate stages, and apply the declared ISP. The time measure is in seconds; trajectories, focus settings and lights share the same clock. This is a proposed implementation contract, not a claim that one stock renderer exposes every integral.

This order matters:

- Subject motion changes ray intersections, silhouette coverage, cast shadows and reflections throughout exposure. A post-render translation of an already blurred photograph does not reproduce arbitrary moving 3D objects.
- Camera translation at macro distances produces depth-dependent parallax and changing visibility. A single image homography is a restricted approximation for pure rotation or a plane.
- Motion and defocus integrate together. Sequentially applying two generic image blurs is not generally equivalent near boundaries or changing depth/pupil visibility.
- Exposure integration operates on linear radiometric quantities. Applying blur to gamma-compressed/clipped RGB loses the behavior of bright moving highlights. RSBlur is experimental evidence that saturation, noise, temporal sampling and ISP choices materially affect synthetic-to-real transfer. [M05](https://arxiv.org/html/2202.08771v3)

### Motion branches to retain separately

1. **Inter-frame pose jitter only:** each exposure freezes a camera pose; no within-frame blur. Reproduce this simple WHU-MFM-style diagnostic before adding dynamics.
2. **Continuous rigid camera trajectory:** smooth translation/rotation through each exposure; record `SE(3)` poses and exact shutter times. Use seeded spline/band-limited trajectories. A damped oscillation after a focus/rail step is a useful proposed vibration model, but measured amplitude/frequency distributions are needed before calling it representative of hardware.
3. **Rigid subject motion:** independently controlled translation/rotation, including motion along the optical axis. The latter changes focus and magnification through the exposure.
4. **Deforming subject:** bones, blend shapes or persistent mesh/material coordinates for bends, antennae, wing flaps, breathing. Persistent identity is essential for truthful correspondences.
5. **Rolling shutter:** row-dependent exposure interval on the same trajectory. Distinguish full-frame scan time from per-row exposure duration. Store direction and scan law. Avoid treating a stock renderer's dimensionless shutter settings as milliseconds without checking frame rate and documentation.

Blender's current Cycles source has shutter timing, motion transforms and rolling-shutter fields; this is evidence of implementation machinery, not a calibration certificate. Pin the tested version and verify settings with a known moving edge before relying on it. [M12](https://github.com/blender/blender/blob/main/intern/cycles/kernel/types.h)

## 4. Support and correspondence labels

For each target surface sample, retain stable object/primitive/material-coordinate identity, location at `t_ref`, its trajectory, and projection into each source at specified times. For opaque non-deforming surfaces this is straightforward; topology changes need explicit identity handling. Central-ray depth/flow remains valuable but is insufficient at mixed pixels.

Recommended labels (proposals):

| Label | How to establish it in the simulator | What it does **not** prove |
|---|---|---|
| Target validity | Finite render estimate, valid target ray/pixel filter, material/task eligibility. | Exact convergence at every pixel. |
| Geometric visibility | Ray-test the corresponding surface at source time; retain out-of-frame and occluded separately. | Surface contributed useful sharp information after shutter and pupil integration. |
| Fractional contribution/support | Estimate contributions from target surface identity over pupil/time/pixel samples; keep fractions or moments near boundaries. | A single binary visibility mask is adequate. |
| Focus/motion/saturation qualification | Record circle-of-confusion or local PSF estimate, projected trajectory, exposure/noise and clipping; declare thresholds. | “Not sharply seen” equals mathematical non-identifiability. Joint deconvolution can recover some such content. |
| Appearance compatibility | Flag material/lighting/normal/view changes, time-dependent emission, and newly appearing texture. | Constant 3D identity implies constant radiance. |
| Unobserved geometry/texture | No target-surface contribution under the traced acquisition model, with numerical tolerance. | No indirect reflection or shadow information exists unless those paths were also checked. |

Expose both all-target error and stratified errors on directly useful support, contaminated/inferable support and missing/ambiguous support. Report coverage fractions and failures. Do not hide hard regions by accepting an algorithm's own crop or predicted validity mask.

For an opaque, Lambertian, illumination-stable diagnostic subset, an oracle surface warp can isolate fusion from estimated registration. Give it a distinct result track and label privileged geometry. A geometry warp cannot in general transform a finite-pupil mixed pixel into its correct target pixel. For the hardest boundary cases, a small audit subset can store ray/surface contribution identities rather than just two-channel flow.

A useful impossibility control is a **paired hidden-code construction**: make two scenes identical except for a randomly chosen texture patch that is visible in the target but has zero effect on all captured rays under the simplified model. Identical inputs with different targets demonstrate non-identifiability in that model. If indirect light could carry the code, disable that path in the diagnostic or prove it absent; do not generalize the witness to an unconstrained physical scene.

## 5. Haze, dust, and other stretch goals

### Haze and participating media

SynFog (CVPR 2024) is strong adjacent evidence for combining volumetric path tracing with optics, sensor and ISP. Its methods use ISETCam, calibrated camera parameters and a diffraction-limited optical model; it is not an arbitrary calibrated multi-element macro lens. The paper contrasts volume transport against depth-only atmospheric blending, particularly for multiple active lights. [M09](https://arxiv.org/html/2403.17094v1)

For this project, expose a seeded medium with extinction/scattering/absorption in inverse metres, phase function and spatial/temporal density. Report optical depth `τ = ∫ σ_t ds`, not just an artistic slider. At millimetre subject scale, the same density as a kilometre-scale fog experiment can be negligible; use actual world units. Separate scene haze from lens flare/veiling glare.

Render two optional targets from the same state:

- **Sharp, haze retained:** evaluates focus reconstruction in a scattering environment.
- **Sharp, medium removed:** evaluates joint restoration and changes the task to include dehazing. Removing the medium can alter incident illumination and indirect transport, so declare whether the target recomputes the clean world's lighting or is only a direct-view counterfactual.

Do not compare methods asked only to stack against a clear-world reference and automatically attribute persistent haze to stacking error. Moving particles treated as resolved geometry can have their own focus and motion; a homogeneous volume is a different approximation.

### Sensor-cover dust and dirty lenses

These need separate implementations. Zhou and Lin's sensor-dust model places dust on the sensor's low-pass filter and derives a projection-dependent attenuation pattern; its scale depends on filter offset and f-number. [M10](https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/dust_cvpr2007.pdf)

Gu et al. model lens dirt/thin occluders through attenuation **and added scattered light**, under stated simplifications. A dark image overlay alone omits dirty-lens bright veiling effects. [M11](https://cave.cs.columbia.edu/Statics/publications/pdfs/Gu_SIGGRAPH_Asia_09.pdf)

Proposed practical hierarchy:

1. Calibrated sensor-fixed attenuation map, applied before photon sampling, with aperture-dependent footprint; label as a simplified sensor-cover model.
2. A separate dirty-lens attenuation-plus-straylight model with dependence on illumination/camera settings.
3. Explicit contaminants in an optical tracing model if the renderer can support their location and scattering reliably.

Keep dust fixed in sensor/lens coordinates while subject and camera trajectories move in world coordinates. Post-registration sensor spots can therefore appear as trails. Preserve defective and defect-free paired renders; specify whether the desired target removes dust. Hot pixels are sensor electronics, not dust, and should use their own seeded fixed-pattern mask/model.

### Other useful imperfections

Exposure/light flicker, white-balance/gain drift, focus positioning error/backlash, insufficient settling, compression and clipping are cheap diagnostic branches if applied at the correct acquisition stage. A focus actuator's error is correlated over a sweep; independently sampled frame errors can be a useful stress test but should not be presented as measured motor behavior. Record commanded and achieved focus/pose separately. Hold fixed the latent scene when comparing nuisances, and use independent noise seeds for repeatability estimates.

## 6. Proposed validation and evaluation sequence

| Gate | Check | Expected failure caught |
|---|---|---|
| Static limit | Set all scene/camera trajectories constant. Match the static generator within sampling uncertainty. | Time convention or shutter normalization bug. |
| Short exposure limit | Reduce exposure duration while preserving radiometric normalization. | Unwanted residual temporal blur. |
| Known motion | Moving coded plane and two-depth bars, with independent projection calculation. | Flow direction, units, occlusion, camera-convention errors. |
| Joint focus/motion | Move a thin foreground wire over coded background and through focus; compare time/pupil sample refinement. | Sequential-filter and temporal undersampling artifacts. |
| Rolling shutter | Moving edge with known row scan times; verify zero scan time reduces to global shutter. | Confusing scan duration and exposure. |
| Support witness | Paired hidden code scenes; verify captured measurements match under the declared model. | Unsupported certainty about missing detail. |
| Degradation identity | Zero density/dust/noise/jitter returns the clean branch. | Confounded nuisance application. |
| Convergence | Independent seed repeats and increasing temporal, aperture and pixel sampling; inspect worst regions, not only global average. | Renderer noise or missed thin/moving structures in truth. |
| Transfer | A small calibrated physical sequence with measured timing/trajectory/lighting where possible. | Synthetic success being mistaken for real-camera fidelity. |

Use motion magnitude relative to feature width and blur footprint, sweep duration, speed along depth, occlusion fraction, and within-exposure path length in pixels as interpretable factors. Cross focal order with motion: reversing or randomizing the same focus settings can change which geometry is ever sharply recorded. For an order ablation retain the same time interval and trajectory and regenerate captures; permuting already rendered images only tests order sensitivity, not acquisition order.

For frozen-instant output report reference-view radiometric error, boundary/filament fidelity, duplicate/ghost structures and support-stratified statistics. For a target sequence additionally report warped temporal residual on truly compatible visible surfaces and visibility/track accuracy when those are outputs. Temporal smoothness alone rewards blur and cannot establish correctness. Report method failures and processing time as separate endpoints. These proposals extend, rather than replace, the existing measurement qualification work.

## Search and access audit

Fresh discovery covered dynamic multi-focus fusion and misaligned stacks; reference-based HDR/MEF/burst reconstruction; rolling shutter plus blur; geometry/visibility benchmarks; sensor dust versus lens contamination; and volumetric haze with camera simulation. Queries explicitly included 2025–2026 dynamic-focus work. AiDiF was retained as a 2026 direct lead with restricted evidence. Papers whose “dynamic” referred only to network receptive fields were excluded as motion evidence.

Full or relevant original text was read for RSBlur, SynFog, dynamic HDR, Sintel, TAP-Vid, sensor dust and dirty lenses; WHU-MFM README/Python and repository tree were inspected at a pinned revision. AiDiF and dynamic MEF remain abstract/README evidence. Several first-choice PDF/doc URLs failed; successful author/arXiv alternatives are recorded in the source ledger. No linked Baidu archive was downloaded and no permission contact was sent.
