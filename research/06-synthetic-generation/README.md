# Synthetic focus-stack datasets: research and proposed pipeline

Research cutoff: **2026-11-17**. Four parallel specialist reviews plus an integrated design. This package addresses generation of `N` focused observations and a precisely defined all-in-focus reference, with procedural scene content, optical/sensor modeling and optional capture imperfections. It adds to the earlier benchmark research without replacing its history or acquisition decisions.

**Recommendation:** build a scene-first procedural 3D generator with replaceable camera and sensor modules. Use full-3D finite-aperture rendering for the main evaluation track, a fast layered/PSF approximation for training, and a small compound-lens/wave-optics validation track. The target should have a specified view, time, optical appearance, pixel filter and numerical error budget. Known geometry does not by itself make rendered RGB exact or every target detail recoverable.

Start with the [pipeline design](pipeline-design.md), then the [generator validation and evaluation plan](validation-and-evaluation.md).

## Reading map

| Question | Review | Evidence ledger |
|---|---|---|
| What has already generated synthetic stacks, and what can be reused? | [Datasets and prior art](datasets-and-prior-art.md) | [23 source records](prior-art-sources.json) |
| How much lens physics do the renderers actually model? | [Optics and renderers](optics-and-renderers.md) | [16 source records](optics-sources.json) |
| How should scenes, assets, RAW data and ISP be generated? | [Procedural scenes and sensors](procedural-scenes-and-sensors.md) | [20 source records](procedural-sources.json) |
| What is truth under motion, haze, dust and vibration? | [Motion and imperfections](motion-and-imperfections.md) | [12 source records](motion-sources.json) |
| What should we build and in what order? | [Pipeline design](pipeline-design.md) | Synthesis; proposals labeled |
| How can we trust references and interpret algorithm errors? | [Validation and evaluation](validation-and-evaluation.md) | [4 coordinating-review records](validation-sources.json) |

The ledgers contain **75 evidence records**, with intentional overlap across specialties; this is not a count of 75 distinct papers. [Combined source index](source-index.csv) makes them searchable together. [Review method](review-method.md) states search coverage and evidence rules. [Document validation](validation-report.json) concerns file structure and provenance fields, not optical accuracy.

## Findings that matter most

**The procedural starting point is closer than the earlier catalogue suggested.** FOSSA's Infinigen Defocus describes 200 curated procedural indoor scenes, each with **nine focus distances × five apertures plus AiF**. That is 46 images per scene and nine frames per fixed-aperture sweep. A release is listed, but the modified scene generator and image precision were not established from inspected code/metadata. Macro-scale geometry and qualified reference accuracy still need work. [Paper, appendix A2](https://arxiv.org/html/2603.26658v2#A2)

**Long synthetic sweeps are established prior art.** Neuromorphic focal-stack research reports 480-frame Blender sweeps with AiF targets; MattingMFIF renders multiple photographic planes with DOF-off references; FoD500 points to a Blender generation file. These offer reusable designs, with different geometry, precision, asset and release limitations. None should silently become calibrated photographic truth. [Detailed comparison](datasets-and-prior-art.md#1-direct-3d-generation-precedents)

**Realistic optics and exact scene truth are different components.** CLDefocus provides particularly relevant compound-lens wave-PSF synthesis, but its image recipe uses estimated depth and inpainted hidden regions. Its current repository also withholds generated data pending redistribution clarification. Reuse its optical machinery as a candidate, then supply known procedural geometry and validate the composition. [Primary paper](https://arxiv.org/html/2607.05837v1), [inspected release](https://github.com/lykelee/CLDefocus/tree/06713044b693bc34fe635d26965916e447ee7ce1)

**Renderer labels and names can overpromise.** The reviewed PBRT realistic camera traces compound geometry but has scalar refractive indices and no lens diffraction; the reviewed DeepLens image path needs scene-renderer integration. BlenderProc's inspected defaults include display processing, denoising and limited material bounces that need overriding. Infinigen's current 2.0 alpha does not imply feature parity with its Nature generators. [Optical implementation audit](optics-and-renderers.md#2-renderer-capability-audit), [infrastructure/defaults audit](procedural-scenes-and-sensors.md#1-infrastructure-what-can-actually-be-reused)

**Motion has a useful, explicit ground-truth definition.** Render one instantaneous scene state at a declared reference camera/time, and retain per-frame sharp auxiliary images. Distinguish this from an exposure-integrated target that preserves motion blur. A moving wing can hide target texture throughout the capture; label that unsupported detail instead of treating every mismatch as a fusion failure. Synchronized multiview/multifocus work such as AiDiF changes the acquisition problem and provides adjacent ideas, not a turnkey sequential-stack solution. [Temporal contracts and example](motion-and-imperfections.md#2-make-time-viewpoint-and-desired-degradation-explicit)

**“Pixel-perfect” should mean an auditable contract and error budget.** Start with analytically checked opaque targets; qualify rich renders with independent sampling/convergence and a second implementation on small scenes. Preserve aperture/time mixing, reference uncertainty and selection/mixture diagnostic floors. Keep matched-model correctness, optical-model mismatch and real-camera transfer as separate experiments. [Acceptance suite](validation-and-evaluation.md#4-minimum-generator-acceptance-suite)

## Concrete next step

Implement the contract and eight small analytic fixtures, then a 24-scene procedural feasibility pilot. Freeze geometry across each sweep; save clean linear observations, declared targets, central geometry, support/visibility information and uncertainty. Measure numerical accuracy, acceptance rate, runtime and storage before scaling. Add sensor/ISP and compound optics next; motion, haze and dust remain separate optional tracks. These counts and milestones are proposed planning choices, not validated sample-size recommendations.

No renderer, dataset generator, lens calibration, stacker or training experiment was run for this review. Linked archives were not downloaded or decoded. Public code/data links are evidence of availability with stated limits, not a complete rights or reproducibility certification. The work delivered here is a research package and implementation design.
