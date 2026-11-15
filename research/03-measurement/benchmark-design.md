# Macro focus-stacking measurement suite

Design date and literature cutoff: **2026-11-15**. Status: proposed engineering design, not an implemented or validated benchmark. No algorithm output, metric, detector, human study, or timing experiment was run. Evidence IDs resolve in [sources.md](sources.md). Artifact IDs refer to [phase 1](../01-artifacts/artifact-taxonomy.json).

## Decision

Use **an evidence-tiered artifact vector, explicit coverage/failure gates, and blinded human comparisons**. Do not launch with one universal quality score. A method can remove halos by deleting hairs, increase sharpness by adding noise, and improve a mean score by cropping hard pixels. These tradeoffs must remain visible.

The suite has three distinct outcomes:

- **Fidelity:** did the output preserve independently established scene content, geometry and radiometry?
- **Artifact burden:** how much area, boundary length, structure length or object count is defective, how severe are the defects, and how certain is the judgment?
- **Perceived usefulness:** under fixed viewing conditions, which output do experts prefer for the declared macro task?

No-reference in this report means *no trusted all-in-focus image*. It does not mean no information: stack-conditioned measurements can use original frames, calibrated optics, geometry, repeated captures and labels. Output-only blind IQA is a different, weaker evidence class.

## 1. Define the target before running a method

Each scene declares a target camera/view, pixel grid, visible domain, intended sharp subject region, intended background blur policy, material class, temporal policy, color/range contract and reference type. A subject-only stack with smooth background must not be penalized for failing to sharpen the background. A full-scene all-in-focus target must not silently inherit that aesthetic exception.

Separate source availability into directly observed, contaminated/inferable and unobserved/indeterminate. These labels are independent of the tested output. No clean source pixel does not prove that reconstruction is impossible: multiple mixed observations may constrain the answer. Conversely, a plausible reconstructed hair is not evidence that its exact geometry was recovered.

There is a fundamental limit: if two admissible scenes produce indistinguishable input stacks but different target details, a stack-only judge cannot certify which detail is correct. The suite must expose uncertainty or abstain. Better blind IQA cannot remove this ambiguity. This is an identifiability argument, not a measured failure rate.

## 2. Use two pipeline conditions and separate evidence classes

| Condition/class | Inputs and comparison | What it isolates |
|---|---|---|
| F: fusion core | Fixed external decoding/radiometry; independently supplied transforms and coverage masks; same aligned stack for all methods | Fusion, noise/detail balance, boundary reconstruction |
| E: complete pipeline | Original unaligned capture; method's native registration, correction and export | Delivered behavior, geometric failures, fallback, crop and reliability |
| C: controlled truth | Analytic targets, renderer-defined truth and measured target geometry, including exact-labeled stress cases | Artifact severity and factual error where reference assumptions are defensible |
| R: realistic no-AIF-reference | Full real macro stacks, local support annotations, repeats/calibration where possible, blinded review | Real transfer and failures missing from simulation |

F/E are pipeline conditions; C/R are evidence/corpus classes. These axes are orthogonal, not four mutually exclusive datasets. A controlled scene can run in both F and E conditions, and a real scene can have both trusted local evidence and unreferenced regions. All derived conditions retain one scene-group identity for splitting and statistics; do not count them as independent scenes.

Run native decode as an E-track condition; do not mix it with a fixed-decode F comparison. Record pre-tone/pre-sharpen output when available and final delivered output as different stages. A method lacking an intermediate interface can still enter the delivered-output track; do not infer its hidden core score.

Preserve decoded precision, nominal range, transfer function, color primaries, white point, exposure scaling, clipping and demosaic settings. Float32 values in 0–255 do not imply 8-bit quantization; a 16-bit file does not prove a 16-bit pipeline. Radiometric errors use fixed scene-linear units. Display metrics use one frozen color-managed rendering for every candidate/reference; no per-output exposure normalization, histogram matching or white-balance fit.

The supplied transforms and masks are benchmark assets, audited against independent landmarks or known geometry. They are not produced by the method being judged. Registration uncertainty is recorded; a low-confidence correspondence cannot support a precise fusion-error claim. Do not align candidate outputs with a flexible warp before scoring: it can repair the geometric error being tested. Only an externally specified grid transform, identical by contract, is allowed.

## 3. Reference classes determine valid claims

| Evidence class | Permitted measurements | Principal limit |
|---|---|---|
| Analytic or converged synthetic target | Pixel/color error; exact visibility, line topology, halo, highlight and code tests | Exactness is relative to declared scene/camera/material/sampling model |
| Controlled physical target | Calibrated edges, line codes, color patches, landmarks, repeated noise measurements | Manufacturing, calibration and optical uncertainty remain |
| Independently supported real region | Registered focused source fidelity, local line/edge retention, noise/detail checks | Only supported locations; contaminated boundaries often excluded or qualified |
| Expert artifact annotation | Localization, ordinal severity, preference and task accuracy | Rater uncertainty; no universal pixel truth |
| Software-produced composite | Teacher-agreement PSNR/SSIM and disagreement triage | Pseudo-reference contains its producer's errors |
| Output-only IQA | Exploratory quality/naturalness alarms | Cannot certify focus completeness, correct hidden geometry or factual detail |

Keep the class in every score record. Do not pool synthetic-reference PSNR with teacher-agreement PSNR as though they measure the same quantity. MFIFB's broad metric collection is useful compatibility prior art; its pair scope and metric-selection warning do not establish a long-macro quality standard. [Q01]

SSIM compares structure against a reference; LPIPS models learned perceptual similarity; DISTS deliberately tolerates texture resampling. That last property is useful for perceived texture but can miss the exact bristle/code identity needed here. FLIP produces perceptual difference maps for specified viewing assumptions. Use these as secondary views alongside factual checks, never as universal adjudicators. [Q02–Q05]

NIQE models departures from natural photographic statistics. A clean scientific target or unusual macro texture can depart from those statistics without being defective. Its macro suitability must be measured. Do not let a blind score override known line geometry or source evidence. [Q06]

Fusion-specific quality prior art includes Hassen et al.'s contrast/sharpness/structure index with a subjective database, edge-transfer measures, and later metric-consistency studies. These are comparator hypotheses. Agreement with another IQA model is not the same as independent agreement with macro experts. No one such score is promoted to the main endpoint without qualification. [Q07–Q09]

## 4. Artifact measurements and counting

[metric-registry.json](metric-registry.json) supplies implementable definitions, applicability limits and attacks. It contains one or more measures for every A01–A20 artifact. A score is a structured record, not merely a scalar:

`{metric_id, scene_id, region_id, value(s), unit, valid_denominator, eligible_denominator, confidence, uncertainty, reference_class, status, reason}`.

Use `not_applicable`, `insufficient_evidence`, `measurement_failed` and `method_failed` separately. Unknown is never zero. Detector confidence is calibrated on independent data; raw neural confidence is not a correctness probability.

For each artifact report extent, conditional severity, joint burden and tails. For a defect map `d(x)` and a frozen threshold `tau`, area fraction is `|{x in D: d(x)>tau}|/|D|`; conditional severity averages `d` over that set; burden averages the nonnegative excess `max(d-tau,0)` over the full eligible domain. Also report raw continuous distributions before thresholding. Use boundary length for boundary defects and true structure length for lost hairs; image area alone hides critical thin detail.

Count connected defects with fixed scale, connectivity, minimum size and matching rules. One missing hair can occupy few pixels but lose its full known length. A halo/ringing/geometry error may receive multiple labels: report overlap and multilabel components, not a sum pretending these are independent physical events. Preserve a union defect map and separate class counts.

At each viewing scale report per-scene median, 95th percentile and worst fixed-size region; the region grid/window size is fixed before comparing outputs. Include physical units where calibrated and original-pixel units always. Multiscale testing uses prescribed resampling of *both* reference and output; it cannot select a favorable scale per method.

Measure noise, retained detail and invented detail together. A repeated-stack difference estimates output noise/instability, while an independent reference measures bias. Blurring can reduce noise; sharpening can increase SFR; both require paired detail/overshoot tests. Independent source support avoids selecting the noisiest maximum-response frame as truth. When no independent noise replicate exists, report a qualified estimate rather than an exact noise score.

## 5. Coverage, capture limits and failure

Freeze the intended evaluation domain before algorithms run. Save each output's true source coverage and crop transform. Reflected/replicated border pixels are not additional observations. Never shrink a scored mask to whatever region the output makes convenient.

Report (i) intended-domain coverage, (ii) available-source focal coverage, (iii) scored evidence coverage, and (iv) method success rate. Conditional pixel error is reported on valid measured pixels, but a method cannot win the overall comparison by omitting difficult regions. Missing pixels and failed scenes remain visible in the coverage/failure gates; do not silently drop them or assign zero error. A fixed test cohort is the denominator for success statistics.

Missing focal coverage is partly an acquisition problem. Use a complete-coverage track to judge achievable fusion, and an incomplete-coverage robustness track to judge truthful uncertainty, graceful output and warnings. Do not penalize a method for failing to recover exact unidentifiable structure; do measure fabricated *incorrect* structure on known synthetic/physical codes and distinguish perceptual filling from factual recovery.

Inherited chromatic aberration, diffraction, sensor defects and source clipping are labeled before fusion where evidence permits. Report input defect level, output level and change. This separates inability to undo acquisition damage from newly introduced damage without pretending they are independent in every natural scene.

## 6. Source consistency and physics checks

A source-conditioned check evaluates a locally supported patch against independently selected clean sources after fixed geometric/radiometric conversion. It is not `max` similarity over all source frames at each output pixel: that can bless inconsistent mosaics, noise maxima and finite-aperture background leakage.

Forward consistency is an optional *restricted track*. Let calibrated image formation be `I_k = Phi_k(L, G, M, theta) + noise`, with scene radiance/texture `L`, geometry `G`, materials `M` and camera parameters `theta`. Hold out focal settings/captures, fix model complexity and calibration without seeing outputs, and test noise-whitened residuals on held-out measurements. Include geometry/material mismatch checks and likelihood calibration. Different models with equal observational support can still predict different sharp targets; residual agreement is necessary evidence, not proof of unique recovery.

An all-in-focus RGB image is generally insufficient to generate a finite-aperture stack: visibility, hidden surfaces, depth and view-dependent materials matter. Reblurring final RGB with a generic depth-varying 2D Gaussian is not accepted as a physical validation of arbitrary macro scenes. If the candidate cannot be mapped to a declared latent scene representation, the physics score is unavailable. [Q13–Q15]

## 7. Robustness without invalid invariance demands

Ordered methods receive a valid focal sweep. Permutation is a required invariance only if the method claims order independence. Reversal, duplicate frames, valid thinning, slight exposure/noise variation, slab size/overlap and batch size are separate declared conditions; thinning can genuinely remove information and must not be treated as an invariant.

Test identity/single-depth stacks, duplicate-noise versus independent-noise duplicates, rotations with exact ground truth, repeated texture, increasing frame count, 2/4/8/30/100+ frames where data permit, and full native resolution. Every transformation has a known expected relationship; an operation that changes optics, coverage or evidence receives a changed target/mask. Measure CPU/GPU/fallback differences and stochastic seed variation. Report the reduction tree for recursive pair methods.

## 8. Overall comparison and statistics

Primary release report: coverage/failure gates, artifact vector by scene/material/difficulty, factual target accuracy, blinded human preference with uncertainty, and runtime/memory. Show a Pareto comparison where methods trade detail, halo, color and noise. Quality is not allowed to disappear behind speed.

Use scene/specimen/asset family as the independent unit. Frames, pixels, crops, overlapping windows and derived render variants are not independent test samples. Compute paired method differences per scene, then stratified/clustered bootstrap intervals over independent groups. Include variability from seeds and raters with a hierarchical analysis or separately reported intervals; do not inflate scene sample size with those repeats.

Keep an intentionally difficult challenge set separate from a representative field test. A challenge set measures capability on selected failures; it cannot estimate field prevalence. Predeclare field sampling, strata and mixture weights. Report stratum sizes, eligible/scored coverage, macro-average over groups, tails and missingness. If sampling is convenience-based, do not label the mean a population prevalence estimate.

A single scalar may be added later only after locked validation: preregister normalization, failure/coverage treatment, artifact weights and target use; fit on metric-development data; test on untouched metric-qualification scenes and unseen algorithm families; disclose pairwise ranking errors. Retain the vector and human result even then. A catastrophic missed hair or false transparency may be a noncompensable gate for scientific use, while an artistic use may choose another policy. Such gates are task decisions calibrated with users, not universal image laws.

## 9. Concrete first pilot

Proposed pilot, not a statistically powered final benchmark:

1. Implement deterministic I/O, fixed-domain accounting and analytic targets; verify ramps, color, crop and precision before evaluating algorithms.
2. Generate 12 independent controlled scene/asset families spanning opaque occlusions, fine filaments, smooth surfaces, repeated texture, glossy points and transparent material. Keep whole families together; use separate seeds only as repeats.
3. Acquire/license 12 independent real macro scene/specimen groups across several capture sessions; include complete and incomplete coverage, ring and rail, diffuse and reflective subjects. A 999-frame stack of one subject counts as one group.
4. Run requested baselines plus identity/blur/sharpen/noise/crop controls. Calibrate targeted metrics using [validation-protocol.md](validation-protocol.md); obtain blinded expert labels on outputs and defect ladders.
5. Freeze a v0 metric contract only after its scope-specific acceptance criteria pass. Expand sample size from pilot variance and desired minimum effect; then run a preregistered, independently held-out evaluation.

The proposed counts are planning values. They cannot establish rare-artifact rates or general macro superiority. Dataset review owns acquisition terms, concrete corpus suitability and leakage-safe scene splits. [handoff.md](handoff.md) identifies remaining decisions.
