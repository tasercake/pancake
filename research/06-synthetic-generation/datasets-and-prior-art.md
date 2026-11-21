# Synthetic focus-stack generation: datasets and adjacent prior art

Research date: **2026-11-17**. This is an independent literature review and implementation recommendation, not a dataset-generation experiment. It extends the [earlier dataset catalogue](../04-datasets/dataset-review.md); existing real-stack acquisition decisions remain there. Sources and evidence levels are recorded in [prior-art-sources.json](prior-art-sources.json). Counts below are source-reported unless explicitly described as remotely listed; archives were not downloaded or decoded.

## Findings that change the starting point

**There is now direct procedural prior art worth adapting:** Infinigen Defocus renders multiple focal settings and an all-in-focus image from a single procedural 3D scene. **There is also unusually relevant lens-diverse simulation:** CLDefocus supplies a compound-lens PSF and radiometric pipeline, but its published image-generation recipe uses estimated depth and inpainted hidden content. The two solve different parts of this project's problem. Neither release establishes a calibrated macro benchmark with pixelwise radiometric error bounds.

The strongest architecture suggested by this literature is a shared procedural 3D scene representation with separate camera/sensor backends, plus a fast RGBD/layered approximation for training and ablations. Preserve the distinction between that approximate branch and the 3D reference branch. This is our synthesis, not a demonstrated combined system.

## 1. Direct 3D generation precedents

### Infinigen Defocus / FOSSA, 2026

The *Zero-Shot Depth from Defocus* appendix describes 500 generated indoor scenes, curated down to 200. Each retained scene has nine focus distances at five apertures plus an AiF render: 46 images. The focus range is 0.8–8 m; apertures are f/1.4–5.6. This is the closest inspected precedent to procedural N-frame stacks with a separately rendered sharp target. The thin-lens Blender path traces scene geometry rather than applying a blur to one RGBD view. Its indoor, meter-scale content still needs substantial redesign for macro structures. [Paper, Appendix A2](https://arxiv.org/html/2603.26658v2#A2)

The author's [dataset release](https://huggingface.co/datasets/venkatsubra/InfinigenDefocus) lists a 9,092,311,161-byte ZIP and includes a BSD-3-Clause license. The archive contents and image precision remain uninspected. The [FOSSA repository](https://github.com/princeton-vl/FOSSA/tree/97ed839fd74c1d785fa7855fb7c2cedc4d0e6182) exposes a loader; the tree search did not establish release of the modified scene-generation pipeline. Its separate RGBD training route uses randomized PSFs; it should not be mistaken for this ray-traced benchmark. **Adapt the design; verify generator availability before adopting it as a dependency.**

### FoD500 / Focus on Defocus, CVPR 2020

The [official release](https://github.com/dvl-tum/defocus-net/tree/418601f5f8f9cb37477e5d71923b68bcaf744edd) describes 500 samples, first 400 for training, each with five focus images and depth. Focus distances are 0.1, 0.15, 0.3, 0.7 and 1.5 m, focal length 2.9 mm, f/1. The README provides a Blender generation-file link. This makes it a useful small end-to-end template, but camera scale and five-frame sampling do not establish photographic macro realism. MIT code does not automatically clear the generation scene's upstream assets. A complete AiF-target/export contract was not verified.

The [DEReD author page](https://sihaozhe.work/DEReD/) explicitly reports regenerating a 1,000-stack version with better distributed focus distances and using a 500/500 split. This is a different generated version, not evidence that the original release had 1,000 samples. Its simultaneous RGBD/PSF experiments should likewise remain a distinct recipe.

### MattingMFIF, 2025 preprint

The [paper's generation algorithm](https://arxiv.org/html/2510.19581v1) places photographic alpha-cutout planes at 1–9.5 m with a background at 10 m, randomly rotating/scaling them. It samples 90–120 mm focal length and f/0.1–1.5, renders each plane's focus distance, then disables DOF for the target. It reports 4K output, 10,000 training samples, 200 test samples and ten lighting HDRIs. This is already multi-plane/N-frame generation, not necessarily just a pair generator.

The useful ingredient is genuine aperture sampling over layered geometry. The limitation is scene content: cutouts carry baked lighting and possible source blur, depth is planar, and extreme apertures are not evidence of a plausible macro lens. The paper's phrase “perfect GT” refers to its DOF-off render convention; it does not bound Monte Carlo error or establish real optical truth. No complete generator-and-asset package was verified during this review. Use a small version as a **boundary-control track**, keeping cutout, background and HDRI ancestry together across splits.

### Neuromorphic focal stacks, TPAMI 2024

*Teng et al., Hybrid All-in-Focus Imaging From Neuromorphic Focal Stack* reports 200 random Blender scenes with scattered geometric objects textured from MS-COCO. Each gets 480 focus frames at 100 mm, f/1.2, and an AiF reference; DVS-Voltmeter/Spikesim subsequently generate neuromorphic streams. It is relevant even without events: long synthetic focus sweeps are established, and the temporal event simulator is a separate downstream stage. The inspected implementation section does not establish independent sub-exposure scene dynamics, exact target radiometry or a ready licensed generator release. [Author-hosted paper, §IV-F](https://www.jdl.link/doc/2011/20241219_Hybrid_All-in-Focus_Imaging_From_Neuromorphic_Focal_Stack.pdf)

### Finite Aperture Stereo, IJCV 2022

This extends the task to multiview focus stacks: the paper describes Blender renders of Stanford Armadillo/Bunny/Dragon from 24 views, five focus settings per view chosen from ground-truth depth. It is useful for testing target-view/parallax policy and geometry reconstruction, rather than assuming every focus stack shares a perspective. The [official download page](https://www.cvssp.org/data/fas_data/) requires accepting noncommercial-research conditions. The generation scene and AiF radiance reference were not verified. [Primary paper](https://doi.org/10.1007/s11263-022-01658-w)

## 2. Lens-diverse and layered defocus synthesis

### CLDefocus, 2026 preprint: borrow optics, replace the scene truth

[Lee et al.](https://arxiv.org/html/2607.05837v1) combine compound-lens wave-optics PSFs, linear radiometry, layered occlusion handling and an ISP. They report 40k/1k/1k patch pairs, disjoint source-image and lens splits, and 700 filtered lenses. The scene recipe uses DPDD sharp RAWs, Depth Pro geometry, depth quantization and hidden-region inpainting. Better PSFs therefore do **not** make its inferred hidden scene factual ground truth.

Crucially, the [pinned repository README](https://github.com/lykelee/CLDefocus/tree/06713044b693bc34fe635d26965916e447ee7ce1) withholds the image dataset while redistribution terms are unresolved, despite the paper's availability statement. MIT synthesis code is present. [Synthesis instructions](https://github.com/lykelee/CLDefocus/blob/06713044b693bc34fe635d26965916e447ee7ce1/datasyn/README.md) expose resumable stages, external lens prescriptions, RAW/depth preparation, and an approximately 90 GB training-generation footprint. They also document seven omitted converted lens tables in the uploaded collection. For this project, retain one scene across a focus sweep and inject known procedural geometry rather than resampling/inpainting each frame. Validate any layered approximation against aperture-ray 3D renders.

### BokehMe / BLB, CVPR 2022

The [author repository](https://github.com/JuewenPeng/BokehMe/tree/8b3ed556dc146207c9bb39fccd078a113f8d1bd8) describes BLB: ten Blender 2.93 scenes, each with AiF, disparity, five blur amounts × ten refocus disparities, parameter files and corrupted-depth variants. EXRs can exceed one; their published evaluation clips to [0,1]. Preserve unclipped linear radiance when adapting this dataset. Do not inherit the displayed gamma/clip procedure as the benchmark truth contract. The repository is Apache-2.0; complete data and asset rights were not audited.

The [paper](https://openaccess.thecvf.com/content/CVPR2022/papers/Peng_BokehMe_When_Neural_Rendering_Meets_Classical_Rendering_CVPR_2022_paper.pdf) distinguishes a 150-scene simplified-ray-tracing training source from the Blender test source. Its hybrid neural renderer repairs classical-renderer errors. Such repairs can be useful training augmentation, but their outputs cannot certify exact aperture-visible content.

### MPIB, 2022, and Dr.Bokeh, CVPR 2024

[MPIB](https://arxiv.org/abs/2207.08403) directly identifies the missing-background problem: a single AiF view omits regions that become partly visible through a defocused foreground. It combines multiplane representation, background inpainting and a ray-traced training generator. **Inference:** storing only target RGB+depth is insufficient for reusable exact scene simulation; retain full geometry or complete layered textures.

[Dr.Bokeh's author project](https://shengcn.github.io/DrBokeh/) proposes a filtering equation and a differentiable occlusion-aware layered renderer. It is a useful independently implemented comparison for boundary tests and optimization. It still preprocesses RGBD into layers, so unknown hidden content remains a scene-representation limitation. The public code [tree](https://github.com/ShengCN/DrBokeh-Src/tree/24d96e4fb76a698494e86835c84a7c45ae2ebc10) had no GitHub-recognized license in the API check; resolve the applicable terms before redistribution. Differentiability does not establish physical completeness.

### SYNDOF, CVPR 2019

The [official MATLAB generator](https://github.com/codeslake/SYNDOF/tree/7215b1faa5282ea9bc370357ca40b2109d6976ee) consumes synthetic depth/image datasets and writes blur/defocus labels. The code repository is AGPL-3.0. Its map encoding rounds to tenths and then normalizes by a documented scale: a useful reminder that stored label values need a precise unit/quantization contract. This is a cheap legacy approximation baseline, not a substitute for full 3D pupil visibility. Preserve upstream image rights separately.

### Sdirt / dual-pixel ray simulation, 2025

[Simulating Dual-Pixel Images From Ray Tracing For Depth Estimation](https://arxiv.org/html/2503.11213) traces a lens and simplified sensor microlens, trains an MLP on PSFs, then convolves RGBD with predicted local kernels. Its DP119 physical test includes measured planar scenes and LiDAR/structured-light depth for others. Useful lessons are field/depth-dependent PSF calibration and keeping the hardware setup consistent. The final image synthesis is still RGBD kernel rendering, not full scene-path tracing; the sensor structure is simplified and the optical comparison uses a 550 nm assumption. Do not treat “ray tracing” in a PSF generator as proof of full-scene occlusion correctness.

## 3. New rendered bokeh corpora and physical validation

### UnrealBokeh / AnyBokeh, 2026

[The paper](https://arxiv.org/html/2606.31959v1) renders fixed scenes while varying focus/aperture and retains depth, sensor and camera metadata. It reports 11,477 images and 382,888 editing pairs. Its thin-lens CoC conversion explicitly distinguishes millimeters from pixels; its optical-fingerprint approximation assumes non-macro distances.

The current [author data card](https://huggingface.co/datasets/itsmag11/UnrealBokeh) resolves an important scale distinction: **two environments, 78 views, 221 shared depth maps**, 2016×1120 JPEGs; 71 views train and seven validation. The paper's 71 camera setups should not silently replace the current release description. These are neither 11,477 independent scenes nor 382,888 independent physical samples. Academic-research permission covers released images; underlying Fab assets remain the creator's property. [AnyBokeh code](https://github.com/itsmag11/AnyBokeh/tree/1a38ec9beeee1bfdd47f4cab40b07ec2d2ebd94a) uses the noncommercial S-Lab license. This corpus is useful for metadata and focus/aperture controls, but its split can share environment assets, and its render backend must be audited before a pixel-accuracy claim.

### RealDefocus / RealBokeh, 2026

[RealDefocus](https://arxiv.org/html/2607.21078v1) reverses a bokeh-rendering dataset: shallow-DOF images become deblurring inputs and f/22 images become references. It reports 23k pairs across 4.4k scenes, nominal 6000×4000 capture, and a wide input-aperture range. This tests optical transfer and realistic highlights; it is not a fixed-aperture focus sweep. A stopped-down exposure retains diffraction and possible motion/radiometry differences, so it is an approximate physical sharp reference.

The inspected [benchmark repository](https://github.com/TimSeizinger/RealDefocus-Benchmark/tree/b3e948be700dc56831a7940ea2b980850201af91) was a minimal README, not a verified complete data pipeline. Separately, AnyBokeh's table flags a difference between RealBokeh's nominal capture and downloadable resolution. Verify actual files before planning a resolution-matched experiment.

### BokehDiff, ICCV 2025: photorealistic synthesis is a separate track

The [author release](https://github.com/FreeButUselessSoul/bokehdiff/tree/38ff9372ab146ab20468832a0424fa0b2888dffd) combines generated transparent foregrounds, backgrounds, layered rendering and a one-step diffusion model. Training data is generated on the fly. It offers useful diversity and stress testing of visually plausible blur. Generated or diffusion-refined images do not provide independent factual scene truth; using them for evaluation can reward matching the generator's invented detail. Keep this branch optional for training and perceptual challenge sets, with inherited image/model licenses recorded.

## 4. What to reuse, and what to build

These are proposed engineering choices based on the review, not measured quality rankings.

| Component | Recommended starting point | Required adaptation before evaluation |
|---|---|---|
| Procedural fixed-view N-frame + AiF design | Infinigen Defocus design | Macro geometry families, deterministic generation, raw/linear export, sample convergence and full provenance. |
| Small reproducible scene prototype | FoD500 generation-file lead | Verify actual generation file, version, scene assets and target pass; remove implicit phone-camera assumptions. |
| Controlled occlusion/alpha tests | MattingMFIF and BLB designs | Procedural known textures, explicit alpha/material semantics, unclipped linear target, ancestry split. |
| Lens and radiometric extension | CLDefocus modules | Replace estimated/inpainted scene data with known geometry; preserve one lens/scene realization across N frames. |
| Fast training approximation | RGBD PSF, layered MPI/Dr.Bokeh-style renderer | Quantify deviation from the 3D reference in boundary/highlight/large-blur cases. Label it as approximate. |
| Real optics transfer | Calibrated targets plus eligible real corpora | Never promote small-aperture/teacher/inpainted images to exact radiometric truth. |
| Learned bokeh generator | Optional BokehDiff/AnyBokeh comparison | No use as sole truth source; separate model and upstream asset terms. |

### A useful distinction for every sample

1. **Known scene state:** procedural geometry, material, lighting and time functions.
2. **Declared target image:** a specified camera/time/appearance operator applied to that state.
3. **Numerical estimate:** actual samples, reconstruction filter and uncertainty of the target render.
4. **Input measurements:** optical/sensor/temporal variants derived from the same state.

A dataset can have exact geometry labels but approximate RGB, or a perfectly aligned image pair generated from incorrect estimated depth. Neither condition should silently be called pixel-perfect physical truth. The older [reference contract](../03-measurement/synthetic-ground-truth.md) supplies the uncertainty and appearance-policy requirements; the contribution here is identifying reusable generation precedents and current release limitations.

### Research gaps worth testing first

- Does a Blender thin-lens macro stack predict measured foreground-edge spread and hidden-background contributions? Test simple opaque planes before complex materials.
- At equal focal coverage, how do errors change with frame count, inverse-depth sampling and out-of-range depth? Most pair corpora cannot answer this.
- Can the same procedural scene be rendered through both an aperture-ray backend and a layered/PSF backend? Publish the discrepancy by boundary distance, CoC, field position and material.
- How much apparent improvement disappears when test geometry families, lens prescriptions and texture ancestry are held out independently?
- Is the declared sharp target actually observable from the stack? Full scene knowledge lets the generator create a target that no input supports. Keep a recoverability/visibility analysis alongside it.

## Search and evidence scope

Fresh searches covered synthetic focal stacks, AiF/fusion generation, depth from focus/defocus, partial occlusion, lens-diverse deblurring, rendered bokeh and 2024–2026 updates. Primary papers, author repositories and author-hosted dataset cards support the claims above. Searches found no inspected release that combines arbitrary long procedural macro stacks, calibrated compound optics, full nuisance simulation and bounded target error in one ready package; this is a scoped finding, not proof that none exists. Paper rankings were not reproduced. No renderer, model, dataset ZIP or camera was executed in this review.
