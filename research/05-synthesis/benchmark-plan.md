# Measurement-first macro focus-stacking benchmark

Research date: **2026-11-15**. All four reviews and integration are complete. This is a proposed benchmark, not a validated quality score or an assembled training corpus.

## Recommendation

Build an **evidence-tiered artifact suite**, with explicit coverage and failure accounting, and use blinded human comparison for overall delivered quality. Do not start with a single weighted “sharpness” number. The suite must prove that its measurements respond to real defects before an algorithm is optimized against them.

Keep focus-stack as an independent complex-wavelet comparator. This is an engineering choice, not a measured quality ranking. The [pinned base audit](../02-algorithms/base-comparison.md) explains the important precision, input, tone-mapping, order and border differences.

## 1. Define the image being judged

A run must declare its target viewpoint, intended focus domain, background-blur policy, valid field of view, material assumptions and motion policy. A subject-only stack with a deliberately blurred background should not be penalized for that background. A rail sweep with changing perspective cannot silently be compared as though every source were one fixed-view photograph.

Use the [artifact taxonomy](../01-artifacts/artifact-taxonomy.json) to label source availability:

- **Directly observed:** suitable focused, clean target information is present in a source.
- **Contaminated or inferable:** information contributes to measurements but is mixed, defocused or partly occluded.
- **Unobserved or indeterminate:** desired information is absent or not identifiable under the declared model.

Absence of a clean source pixel does not rule out reconstruction from complementary measurements. Conversely, an attractive reconstructed hair or highlight is not evidence that its exact shape was recovered. When two plausible scenes explain the same measurements but require different target images, the measurements alone cannot certify which target is correct. This is an identifiability limit, not a problem solved by selecting a stronger image-quality network.

## 2. Keep evidence types separate

| Evidence | What it can support | Main limit |
|---|---|---|
| Analytic or renderer-defined targets with known scene state | Error under a declared model and numerical uncertainty; geometry, visibility and artifact localization | Only analytically verified cases can claim strict numerical truth; finite Monte Carlo references remain estimates. Model fidelity to real optics is a separate question. |
| Controlled real targets and repeated captures | Measured edge/detail/color/noise behavior and specific physical failures | Often supplies trusted regions or properties, not an exact full-frame reference. |
| Real photographic stacks without a trusted all-in-focus image | Source-conditioned support, failure alarms, expert comparison and ecological relevance | General factual correctness can remain unknown. |
| Software-generated or hand-retouched composites | Teacher agreement and useful review aids | Can inherit fusion, geometry or retouching errors. |
| Blinded human judgments | Visible artifact severity and overall preference under controlled viewing | Preference alone does not establish physical correctness. |

Each score carries its evidence tier, applicable domain, supported-area fraction and uncertainty. Missing evidence is not zero error. “No reference” must distinguish an output-only score from a diagnostic that uses the entire source stack but no all-in-focus reference.

## 3. Measure localized defects before aggregating

The detailed [measurement design](../03-measurement/benchmark-design.md) and [metric registry](../03-measurement/metric-registry.json) define the proposed diagnostics. They must cover the 20 artifact classes without pretending one output pattern uniquely identifies its cause.

Report at least four quantities for each applicable defect: **extent, conditional severity, worst-region burden, and confidence/support**. Preserve units such as pixels, radiance residual, boundary length, lost line length, object count or clipped area before any perceptual calibration. Report both absolute and resolution-normalized scales where appropriate.

Treat detail retention and false detail together. Low noise alone rewards blur; high gradient energy can reward noise and contrast; smooth boundaries can hide missing hairs. Likewise, count missing/duplicated structures and assess shape, opacity and source support. Global PSNR, SSIM, fusion metrics, learned perceptual distances and blind IQA are useful secondary views with known domains, not substitutes for artifact-specific checks.

Artifacts can overlap. A doubled hair can also create a seam and halo. Retain multiple labels and component overlap; do not add those labels into a fictitious count of independent physical defects. Small critical structures require object/line matching and local maps so a large clean background cannot hide their loss.

## 4. Control the complete pipeline

Use two primary tracks:

1. **Controlled-input fusion:** fixed decoded/color-managed inputs, validated supplied registration, frozen visibility/validity masks, and a declared output coordinate system.
2. **Native end-to-end processing:** original capture inputs, each application's own alignment and input/output handling, with every correction and fallback recorded.

Where possible, retain pre-tone/pre-sharpen output and final delivered output as separate stages. Verify signal precision, range, transfer function, color transform and clipping with controlled ramps and patches. A 16-bit filename does not establish 16-bit image processing. Floating-point storage does not establish linear radiometry.

Freeze evaluation masks and allowed output alignment independently of the candidate result. Do not use a powerful output-to-reference warp or per-output exposure fit to erase geometric or photometric errors. Do not let an algorithm improve its mean score by cropping hard regions or extrapolating unsupported borders. Report coverage, missing output and failure explicitly on a fixed target domain.

Order tests follow method claims. Ordered focal-plane methods receive valid ordered sweeps. Arbitrary permutation tests apply only to methods claiming order independence. Frame thinning, reversal, duplicate frames, focal gaps, slab size and reduction trees must be declared. Record device path, fallback, seed, parameter search budget and checkpoint hash. Keep manual retouching under a separate effort budget.

## 5. Assemble several complementary data tracks

The [dataset review](../04-datasets/dataset-review.md) audits existing sources and their access/reference limits. An effective corpus needs all of these roles:

- Real long photographic stacks for natural blur, geometry, sensor behavior and long-stack scaling.
- Controlled physical targets for hairs/lines, depth edges, color, highlights, noise and coverage tests.
- Analytic and physically rendered scenes for explicit target geometry and localized error truth.
- Short-pair/light-field and existing quality datasets for compatibility and supplementary qualification.
- A separate representative field test and deliberately difficult challenge suite.

The initial acquisition shortlist is:

| Role | Candidate | Intake conditions |
|---|---|---|
| Real photographic RAW development | [LSFD / FocusDeep](https://github.com/araujoalexandre/FocusStackingDataset): 94 reported 30-frame bursts | Clarify dataset terms, inspect archive and related scenes, preserve original test role, audit incomplete focal coverage and Helicon pseudo-targets. |
| Long-stack scaling and alignment | [Sood macro sweep](https://huggingface.co/datasets/jjjsood/focus-stack-sample): 999 listed CR3 files plus TIFF derivatives, one scene | Creator states CC BY 4.0; verify acquired bytes and decode. TIFFs are 8-bit sRGB. Prefix subsets change focal range and are not matched-coverage length tests. |
| High-magnification/material challenge | [Amber mite](https://doi.org/10.6084/m9.figshare.14707077.v1) and [amber spider](https://doi.org/10.6084/m9.figshare.30771638.v1) optical stacks | CC BY 4.0 metadata; inspect ZIP interiors, duplicate uploads, specimen ancestry and prior processing. Do not use confocal data or existing fused JPEGs as exact optical references. |
| Exposure and high-resolution regression | EDMF, HRMFF and UHD-MFF, audited in the catalogue | Retain paired scope and terms. EDMF currently lists 950 matched IDs, with missing A counterparts for IDs 543–592; these are not yet decoded quality-approved pairs. |
| Measurement truth and missing cases | Newly generated analytic/physical-render scenes and newly captured controlled targets | Proposed acquisition, not existing data. Supply known geometry, visibility, coded detail, radiometric calibration, repeated captures and fresh held-out families. |

The [28-record catalogue](../04-datasets/dataset-catalogue.csv) includes additional phone, light-field, microscopy, synthetic and subjective-rating leads. Its access and rights evidence is per source. No image archive was acquired during this review; remote file listings and creator checksums have not been verified against image bytes.

Short pairs do not replace 100-frame macro scenes. One 999-frame sweep is one scene group. A software-computed reference does not become exact because the input is RAW. Microscopy image formation and real transparent layers require their own applicability labels.

For synthesis, begin with analytically controlled opaque diffuse geometry, then add finite-aperture rendering and separately labeled glossy/transmissive cases. Fix the target camera and image-formation convention. Demonstrate rendering convergence and validate depth, visibility, focal order and source support. Independent validation must catch a shared error between the generator and scorer. A Gaussian blur inside image masks is a useful unit test, not general macro optical truth.

If a strictly guaranteed target is required, use cases with analytically verifiable geometry/radiance or exact discrete construction under stated assumptions. A finite path-traced reference has sampling error; a convergence check estimates an uncertainty floor rather than proving every pixel exact. Preserve that uncertainty instead of treating it as algorithm error.

Keep asset rights and source ancestry. Procedural seeds, textures, backgrounds, geometry and imported photographic cutouts can link many nominally different render scenes. “Rendered in Blender” does not guarantee independent scene content or correct macro optics.

## 6. Split by related scene family before deriving samples

Follow the [split protocol](../04-datasets/split-protocol.md), which owns the proposed allocation and corpus manifest. First form a transitive ancestry/duplicate graph across specimens, scenes, sessions, original photographs, render assets and benchmark repackagings. Assign each connected group to one partition. Only then make crops, noise variants, refocused views, focal subsets or perturbations.

Maintain separate purposes for algorithm training, validation/tuning, metric development/qualification and final algorithm testing. Preserve published official test partitions where applicable; carve validation from eligible training groups rather than reusing the test. Record exposure of public scenes to existing checkpoints or benchmark development. A public test is useful but cannot automatically be called sealed or unseen.

Use scene/specimen-level counts and paired clustered statistics. Do not treat thousands of crops or focal planes as thousands of independent examples. Report results by capture/material/geometry/length strata and use preregistered aggregation. A curated challenge corpus can expose failures but cannot estimate their prevalence in ordinary photography.

For a sufficiently large new algorithm corpus, the proposed starting allocation is **60% train / 20% validation / 20% final test by independent group**. This is a planning policy, not an established optimum or an assigned split. Preserve ancestry and official holdouts even when ratios change. Keep the 24-group metric-development pilot and fresh metric-qualification groups separate from final algorithm testing.

## 7. Qualify the measurements before ranking algorithms

The [validation protocol](../03-measurement/validation-protocol.md) should establish:

1. **Construct validity:** controlled defect ladders change the intended artifact amount while clean negatives and nuisance changes do not trigger it indiscriminately.
2. **Real-failure transfer:** detectors localize and order actual stacking defects on held-out scenes, materials and method families, including mixed artifacts.
3. **Resistance to gaming:** noise, contrast, sharpening, blur, false texture, color shifts, geometry changes and cropping cannot produce an unsupported improvement in the complete evidence vector.
4. **Human calibration:** randomized blinded comparisons and local annotations, repeated trials, ties, calibrated experts and inter-rater uncertainty relate scores to meaningful visible changes.
5. **Statistical adequacy:** paired scene-level uncertainty, tail/worst-case outcomes, coverage and failure rates support a practically meaningful claim rather than a pooled crop-level significance result.

Use maximum-disagreement tests to find counterexamples to competing scores; the [independent metric audit](metric-evidence-audit.md) explains this prior art and its limits. Qualification cannot prove universal correctness. It can establish a defensible applicability envelope and expose important failure modes.

No numerical pass threshold, scalar weight or sample-size target becomes an established standard by appearing in this proposal. Estimate them on metric-development data, validate them on separate groups, then freeze them before the final algorithm test.

## 8. Implementation order

The first metric-development pilot uses **12 controlled scene/asset families and 12 independent real macro groups**. These are planning counts, not a powered train/validation/test benchmark. All variants remain attached to their parent group. The pilot estimates feasibility, detector errors and between-group variance; qualification and final test require new groups.

1. **Intake and contracts:** select eligible sources, preserve provenance and terms, verify image bytes/metadata, identify ancestry groups, and freeze scene/run manifests and target conventions. Existing file listings remain intake candidates until these checks pass.
2. **I/O and adapters:** implement independent decode/export checks, fixed-domain coverage/failure accounting and adapters for the two requested bases. Preserve full stacks, decoded signals, transforms, masks, intermediate stages and errors.
3. **Controlled diagnostics:** implement analytic targets and a small interpretable set covering boundary halos, fine structures, noise/detail, geometry, radiometry/highlights and precision. Include clean controls and known failure doses from the start.
4. **Pilot and qualification:** run real and controlled development groups, collect blinded labels, search for metric attacks, then freeze the scoped contract and test it on new qualification families. Expand sample size from observed variance and desired effect/interval width.
5. **Final comparison:** freeze algorithm settings, test groups and endpoints; run focus-stack and eligible comparators once under the declared policy. Retain failures, tails, coverage, artifact maps, human uncertainty and reproducibility records.

Add learned judges only when they improve held-out defect detection beyond these controls. The 24-group pilot cannot certify rare failures across 20 artifact classes; the [qualification protocol](../03-measurement/validation-protocol.md) gives explicit independent-case calculations and provisional goals.

Run the two requested bases before expanding the comparator list. Add a simple selector/blend, a boundary-aware method, a whole-stack method and a real-burst learned model when their code, data, weights and usage terms are reproducible. Keep generated restoration separate from deterministic fusion. The [algorithm review](../02-algorithms/review.md) provides the candidate set and caveats.

Benchmark output should be a scene-level report with artifact maps, severity/extent distributions, support and coverage, blinded preference, uncertainty and a run manifest. A pass/fail gate or summary rank is acceptable only after its semantics and calibration are frozen. Publish runtime and peak memory separately; speed does not compensate for missing structures or invalid output.

The [structural verification record](research-validation.json) passed schema/example, cross-file join/hash, registry, source-ID and local-link checks. Source spot checks preserve remote counts, missing pairs and duplicate ancestry. These checks establish readable, consistent research artifacts; no image corpus, algorithm result or metric qualification is claimed. See [validation scope](validation-scope.md) for the exact boundary.
