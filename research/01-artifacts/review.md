# Focus-stacking artifacts: evidence and benchmark implications

Research date: 2026-11-15. Phase 1. Scope: long, high-resolution macro stacks. Evidence IDs resolve in [sources.md](sources.md). Proposed tests are hypotheses, not validated detectors. No algorithm was run and no image dataset was downloaded in this phase.

## Main conclusion

Quality is a vector: retained real detail, boundary integrity, geometry, radiometry, material appearance, and cleanliness. A sharper output can be worse. A clean output can hide missing hairs or texture. A plausible output can contain invented content. A benchmark must distinguish these outcomes.

Three questions precede scoring:

1. What image is intended: a defined viewpoint with all visible surfaces sharp, or a subject-only stack with intentionally blurred background?
2. Which desired content was observed sharply and without contamination in at least one frame?
3. Which changes came from acquisition, registration, fusion, or later rendering/export?

The artifact names below describe appearance, not unique causes. The same bright rim can come from optical defocus, a wrong source mask, registration, sharpening, or clipping. Output-only detection cannot in general identify cause.

## 1. Scope boundaries

**Long macro stacks differ from pairwise multi-focus fusion.** Araujo et al. explicitly contrast 2–4-frame benchmarks with macro/micro sequences requiring tens or hundreds of frames. Their real-burst work also documents magnification changes, motion rejection, residual registration problems, incomplete coverage, and fallible software-generated references. These are useful failure evidence, not proof that any commercial output is correct. [S11](https://arxiv.org/html/2311.17846v1)

**Microscopy differs from reflective macro photography.** A transmitted-light microscopic image can include real transparent layers; an opaque insect should not become transparent. A single surface-depth label is unsuitable for every material. Controlled targets can separate opaque, translucent, transparent, reflective, and refractive cases; real scenes require material labels and uncertainty.

**A target image is a convention with physical assumptions.** A small-aperture exposure has different diffraction, noise, exposure, and often material appearance. It is not an exact all-in-focus reference by default. A synthetic pinhole render is exact only for its declared renderer, geometry, visibility, sampling, and radiometric model. Separate truth under a model from faithfulness to a real lens.

## 2. The central optical difficulty: boundaries and visibility

When a nearby object is defocused, its footprint spreads across a more distant surface. The distant texture can therefore be sharp yet veiled. In another slice, the nearby boundary is sharp while that distant texture is blurred. Xu et al. describe regions unclear in both source images; Ma et al. model the asymmetric foreground/background boundary effect. Neither work establishes that every such region is unrecoverable from every densely sampled stack. [S08](https://arxiv.org/html/2012.14678) [S09](https://arxiv.org/html/1910.13136)

Use three availability labels rather than a binary recoverable flag:

| Label | Meaning | Consequence |
|---|---|---|
| Directly observed | Target content is sharply and cleanly present in a suitable source after valid registration. | Loss can be charged to pipeline. |
| Contaminated/inferable | Content contributes to observations but is mixed, blurred, or seen through only part of aperture. | Reconstruction may help; uncertainty and model assumptions matter. |
| Unobserved/indeterminate | Target rays/content are absent, clipped, outside frame, or not identifiable from available data. | Do not claim verified recovery from cosmetic plausibility. |

These are proposed annotation classes. Identifiability is scene- and model-dependent. Defocus is not automatically information destruction; however, copying one source pixel cannot recover a clean value that is absent from all sources. Zerene's leaf/petal example demonstrates this practical distinction. [S01](https://zerenesystems.com/cms/stacker/docs/tutorials/tutorial003)

### A01: broad halos and veiling near depth boundaries

Appearance: pale/dark/colored band; soft margin; missing texture beside sharp foreground. Candidate measures: contamination residual in boundary bands, halo width, signed color error and detail loss versus distance from boundary. Evaluate foreground side and background side separately. Wide bands matter even if whole-image error is small.

Confounders: real haze, flare, transparent edge, soft shadow, background blur selected by design. Ground truth or well-characterized source regions are needed for reliable attribution.

### A02: false transparency / background leakage

An opaque foreground can contain sharp background detail. Zerene's antenna example attributes this to finite-aperture rays seeing around foreground; retaining both structures yields false translucency. This differs from genuine transparent material. [S04](https://zerenesystems.com/cms/stacker/docs/tutorials/stackselectedfortransparentforeground)

Proposed test: render opaque filaments above a coded background, then measure background-code correlation inside ground-truth opaque masks. Count leaked detail only where intended viewpoint says foreground is opaque. For real transparent wings, use a separate category; never suppress all background detail by rule.

## 3. Fusion errors and detail integrity

### A03–A05: softness, missing fine structures, wrong focus

Softness includes excess edge width, lost texture contrast, absent hairs, and sharp-to-soft patches across a surface. A wrong selection map can also produce crisp but incorrect content. Helicon documents a detail/halo tradeoff with radius, plus sharper transitions versus smoother/blurry output with smoothing. [S07](https://www.heliconsoft.com/focus/help9/english/HeliconFocus.html)

Measure both detail retention and false structure. A detector should not reward sharpening a wrong patch. Include thin-line recall and precision, line-width bias, broken-line length, high-frequency coherence with a valid source, and edge-spread measurements on calibrated targets. Arbitrary natural textures need confidence in source correspondence.

Zerene documents a practical contrast: its pyramid approach preserves overlapping fine detail but can change contrast/noise/color, while its depth-map approach better preserves smoothness/color but can lose detail. This is an author-documented tradeoff, not an independent comparative ranking. [S02](https://zerenesystems.com/cms/stacker/docs/howtouseit)

### A06: ringing and overshoot

Ringing is oscillatory edge-adjacent error; a halo is not necessarily ringing. Vanmali et al. analyze wavelet fusion and relate ringing to disturbed scale dependencies. Their structural-dissimilarity measure is relevant prior art; its validation scope does not establish a universal macro-stack detector. [S14](https://www.sciencedirect.com/science/article/pii/S1566253517304748)

Proposed measurements: signed overshoot/undershoot relative to calibrated plateaus; number and energy of extra extrema along edge normals; decay length. Keep real periodic texture, demosaicing artifacts, JPEG ringing, and source sharpening as confounders. Report pre-existing source error separately from added error.

### A07: seams, blotches, and low-texture focus instability

Textureless regions can carry little evidence of focus. Reconstruction literature identifies focus ambiguity and boundary seams as separate problems; a seam can preserve each source accurately yet join them inconsistently. [S15](https://www.sciencedirect.com/science/article/abs/pii/S092523121400441X)

Candidate tests: flat-color planes crossing depth; gentle ramps; repeated grids; mixed texture/no-texture surfaces. Score added luminance/chroma steps, low-frequency patchiness and line continuity. A smooth depth map is not sufficient: smoothing can erase real depth discontinuities. Repeating patterns also demand explicit registration checks.

### A08: noise, grit, and false detail

Maximizing a focus/spatial-frequency response is sensitive to noise; Pertuz et al. demonstrate this in all-in-focus fusion and introduce selective fusion to reduce it. [S12](https://fusiello.github.io/papers/tip13.pdf) Focus-operator evaluation also reports strong noise sensitivity of Laplacian operators and a window-size/detail tradeoff. [S13](https://isp-utb.github.io/seminario/papers/Pattern_Recognition_Pertuz_2013.pdf)

Use repeated captures or calibrated noise simulations for output noise power and spatial correlation. Pair these with texture retention. Low noise alone rewards blur; high high-frequency power alone rewards grit. A noise model fitted on raw data is more defensible than treating gamma-encoded RGB noise as uniform.

## 4. Geometry, registration, and time

### A09–A11: double edges, distortion, motion ghosts

Misregistration can make doubled structures, jagged hairs, seams, local stretch, or geometrically inconsistent but sharp mosaics. Moving camera and lens on a rail changes viewpoint. Li and Nguyen show that scale/shift correction does not generally turn such a composite into one perspective projection; their setting is small-object photogrammetry. [S10](https://arxiv.org/html/1912.03005)

A practical rail-versus-focus-ring example shows smeared edges and radial background streaking from perspective change. This is one documented setup, not a rule that a focus ring always wins. Internal focusing can also shift optical geometry. [S05](https://zerenesystems.com/cms/stacker/docs/troubleshooting/ringversusrail)

Separate registration from fusion with two tracks: supplied validated transforms and full raw/unregistered pipeline. On controlled targets measure landmark error, duplicate-edge count, local scale/shape error and visibility consistency. Do not allow a powerful per-output warp to align away geometric failure before scoring. When the capture has real viewpoint change, declare a target view and score only its defined visible domain.

Scene motion is different from camera motion. Moving antennae can be copied from several instants; Helicon shows this explicitly. [S07](https://www.heliconsoft.com/focus/help9/english/HeliconFocus.html) Require a temporal policy: freeze a reference instant, choose one consistent pose, or label unavoidable motion separately. No single static reference can silently resolve mutually incompatible poses.

## 5. Radiometry and material appearance

### A12–A14: color shifts, highlights, and illumination seams

Color, contrast, clipping and glare require separate measurements. The Zerene FAQ documents internal out-of-range values, clipping at export, and tone compression when preserving range. It also documents dust/hot-pixel trails and warns that preview scaling can create apparent moiré absent from saved files. [S03](https://zerenesystems.com/cms/stacker/docs/faqlist)

An original mineral-photography example reports many bright pinpoints surrounded by blur circles from defocused specular highlights. This is primary practical evidence from one scene, not a measured prevalence claim. [S18](https://diglloyd.com/articles/lenspire/lenspire-20171004-GettingStartedWithFocusStacking.html)

Proposed highlight suite: diffuse white patches, chrome beads, water droplets, glossy facets and subpixel bright points; vary saturation, focus, exposure and viewpoint independently. Score novel clipped area, highlight-count/shape changes, extra blur disks and retained unsaturated signal. When light/viewpoint changes, genuine highlight movement is possible. A highlight is not necessarily attached to a fixed diffuse surface point. Do not enforce brightness constancy as truth.

Exposure mismatch can yield excellent common fusion metrics despite visible artifacts: the EDMF authors state this discrepancy directly. Their evidence concerns smartphone image pairs, so it motivates a stress test rather than validating a macro metric. [S16](https://www.mdpi.com/1424-8220/24/22/7287)

Real wet/transmissive subjects need another track. Braker's specimen-imaging protocol uses immersion to reduce glare, illustrating that acquisition changes material appearance. It does not establish a ground truth for dry appearance or prove any fusion algorithm's material fidelity. [S17](https://pmc.ncbi.nlm.nih.gov/articles/PMC9836466/)

### A15: fabricated or unsupported content

Learned reconstruction can produce plausible details; classical multiscale fusion can also create novel combinations. This is a benchmark risk, not a claim that every learned method hallucinates. Source consistency is useful only after accounting for visibility, blur, noise, and radiometry. A result sharper than every source is not automatically wrong: reconstruction may combine complementary information.

Use controlled hidden codes, repeat captures and known geometry to detect objectively incorrect structure. Natural-image source residuals alone cannot prove factual correctness or error in contaminated/indeterminate regions. Quan et al.'s explicit blur-modelling work confirms that all-defocused and focus-boundary pixels need special treatment; it is not a physical correctness guarantee. [S19](https://ojs.aaai.org/index.php/AAAI/article/view/32714)

## 6. Acquisition limits and nuisance artifacts

### A16–A20: coverage gaps, optical limits, defect trails, invalid borders, encoding errors

Focus gaps can make depth bands soft even under perfect fusion. Aperture trades depth coverage against diffraction; step size depends on resolution requirements and setup. Zerene's controlled every-Nth-frame experiment is a useful way to expose this sensitivity. [S06](https://zerenesystems.com/cms/stacker/docs/tables/macromicrodof)

A future benchmark should label lens blur, diffraction, chromatic aberration, motion blur, source clipping and demosaicing before fusion. Preserve raw inputs when licensed. Add intentionally defective captures to a robustness track, and keep a clean capture track for judging achievable fusion quality.

Sensor-fixed dust/hot pixels can become trails after scene alignment. Border regions can lack focal coverage because rescaling/shifts move content outside frames. Output cropping is therefore part of quality accounting: report scored area and coverage loss. A method must not win by deleting hard regions. [S03](https://zerenesystems.com/cms/stacker/docs/faqlist)

## 7. Why common scores can fail

| Counterexample | Evidence or derivation | Required check |
|---|---|---|
| Add noise; apparent sharpness rises. | S12 studies focus-maximization sensitivity; S13 analyzes operator noise response. | Noise/detail separation, repeated capture. |
| Increase contrast; gradient score rises without new information. | Our derivation: for unclipped `F'=aF+b`, `mean(abs(gradient(F')))=abs(a)*mean(abs(gradient(F)))`. | Radiometric fidelity, clipping and overshoot. |
| Smooth aggressively; halos disappear but hairs vanish. | S01/S02/S07 document smoothing/radius/detail tradeoffs. | Detail recall and false-edge precision alongside halo score. |
| Copy source regions faithfully; exposure seams remain. | S16 reports strong conventional metric scores with visible artifacts. | Explicit seam/color tests plus blinded inspection. |
| Match a commercial reference; inherit its mistakes. | S11 explicitly calls reference pseudo truth and discusses learned errors. | Independent references, disagreement audit, human review. |
| Maximize boundary sharpness; create sharp false transparency. | S04 shows opaque antenna with leaked background detail. | Occlusion/material-aware correctness. |
| Crop away hard boundaries; mean score improves. | Proposed adversarial benchmark test. | Common evaluation domain plus coverage penalty. |

No source establishes one metric as sufficient for all cases. Artifact prevalence or numerical acceptance thresholds cannot be inferred from illustrative examples.

## 8. Handoff requirements for measurement and dataset reviews

The following are proposed requirements, not existing standards:

1. Retain full stacks, capture metadata, alignment validity masks, intended focus domain, material labels and reference provenance. Split by scene/specimen/capture session before crops or synthetic variants.
2. Tag each scored region by artifact type and source availability. Use boundary-normal profiles and connected defects, not only global averages. Preserve distributions and worst regions.
3. Calibrate detector sensitivity with controlled defect ladders: halo width/amplitude, blur radius, missing-line fraction, duplicate-edge displacement, color error, clipped area and noise. Include mixtures and negative controls.
4. Test metric gaming with noise, unsharp masking, local contrast, blur, fabricated microtexture and cropping. Require agreement with blinded expert judgments on unseen scenes.
5. Keep fidelity, perceived quality and capture difficulty separate. Report uncertainty and abstentions. Lock test scenes and metric weights before comparing tuned algorithms.

These requirements should guide phase 2's pros/cons analysis. Base selection remains open. Current evidence supports a measurement-first program, not a declared winning algorithm.
