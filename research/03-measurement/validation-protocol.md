# Metric qualification and human validation

Proposed protocol, **2026-11-15**. No measurements or human trials performed. Acceptance goals below are planning hypotheses, not published focus-stacking standards. Source IDs resolve in [sources.md](sources.md); metric IDs resolve in [metric-registry.json](metric-registry.json).

## 1. Separate metric development from algorithm evaluation

Use four roles with disjoint scene/specimen/asset ancestry:

1. **Metric development:** choose detectors, bands, masks, thresholds, confidence calibration, viewing scale and possible summary weights. The initial 12 controlled + 12 real independent-group pilot belongs here.
2. **Metric qualification:** untouched scene/material/asset families, new defect variants and withheld algorithm families test whether the frozen measures work. Opening these results to revise a metric retires them to development; create a new qualification cohort.
3. **Algorithm train/validation:** train algorithms and select parameters under the dataset protocol. Metric definitions are already frozen for the comparison.
4. **Algorithm final test:** one preregistered comparison on untouched groups, with failures, coverage and human endpoints retained. Do not tune metric weights or algorithm parameters here.

Public legacy sets are external regression/comparison data; prior use by pretrained methods must be recorded. Their historical popularity prevents a strong claim of completely unseen final testing. A split by frame, crop, focus level, renderer seed or image filename does not establish independence. Build connected ancestry groups across source photos, objects, sessions, cutouts, meshes, textures, materials and derivative datasets before splitting. Shared rendering primitives may be unavoidable; hold out meaningful scene/asset/material families and state residual commonality rather than claiming every polygon is independent.

Use leave-one-method-family-out validation as well as scene holdout. A detector trained only on one pyramid method's halos can miss learned reconstruction failures. Record whether a metric is being used as an optimization loss; include outputs directly optimized for it in qualification.

## 2. Build interpretable defect ladders

Start with independently verified clean targets and matched known truth. Change one variable at a time, using independent scene groups and at least several doses around the human-detection threshold. Then test mixtures. Dose is not automatically perceived severity: verify monotonicity with raters.

| Family | Controlled intervention and measured dose | Required negative controls |
|---|---|---|
| Halo/veil and ringing | Signed foreground/background contamination; band width/amplitude; edge oscillation amplitude/period/decay | Genuine glow, shadow, translucent edge and diffraction/source ringing |
| Softness/detail/fine structure | Blur radius, band-limited attenuation, removed true filament length, line thickening/breaks/joins, wrong-plane content | Naturally smooth surface, unresolved subpixel line, legitimate background blur |
| Geometry/motion | Known subpixel displacement, duplicate edge offset, local stretch, rail parallax, inconsistent animated poses | Correct target-view reprojection and permissible coherent pose |
| Radiometry/highlights/noise | Fixed color/tone shifts, exposure seams, clipping, false/missing specular components, independent sensor noise and fixed-pattern defects | Genuine highlight motion, reflection/transmission, naturally bright diffuse patch |
| Factual/coverage/export | Wrong random code bits, invented lines/texture, missing focal planes, crop, extrapolated border, precision loss and incorrect transfer function | Information genuinely unavailable; valid output quantization and correct resampling |

Keep dose in physical/original-pixel/signal units. A post-hoc blur ladder is useful detector calibration but not a replacement for actual optical defocus. Generate real pipeline failures by varying registration, focus spacing, noise, slabs, batch size, tone mapping and algorithm settings. Include strong methods with subtle defects, not only obviously corrupted baselines.

Inject defects into both analytic/synthetic and real-supported regions. Change injection implementation and texture/shape families in qualification to avoid learning a generator watermark. Store exact masks, perturbation parameters, clean/reference provenance and uncertainty. Visually inspect injections for unintended confounds. Raters who verify labels should not see dose, generator or method name.

## 3. Test metric gaming directly

Every main endpoint faces the following attacks before promotion:

- Add independent noise, structured microtexture or repeated-source noise; increase local contrast or unsharp gain; clip overshoot. The suite must not declare a net quality win when factual detail is unchanged or lost.
- Blur/denoise away halos, remove difficult hairs, erase specular points, crop hard borders or return only easy regions. Coverage, structure recall and failure accounting must expose the tradeoff.
- Substitute a plausible texture/code, copy a sharp but wrong layer, leak background through opaque foreground, or imitate the teacher composite. Perceptual similarity must not overrule known factual error.
- Optimize pixels/preprocessing against each differentiable metric; use constrained MAD-style searches that hold one metric nearly fixed while changing another, followed by blinded human discrimination. Also optimize any proposed composite score and inspect its failures.
- Exploit mask, scale, output range, seed selection, support-source selection or missing-value conventions. All choices are frozen externally and audit logs must reveal deviations.

MAD and adversarial-IQA literature motivate active falsification: a high correlation on ordinary examples does not establish that a metric remains valid once optimized. A successful attack defines an applicability failure, not a reason to conceal the counterexample. [Q10, Q11, Q22]

## 4. Evaluate detectors as detectors

Use independent human/known-target labels to measure scene- and region-level sensitivity, specificity, precision-recall, false positives per eligible area/boundary/structure length, severity calibration and localization error. AUROC alone can hide poor precision for rare defects. Report precision at realistic prevalences only when a representative sample supports that prevalence; an enriched challenge set cannot supply it.

For ordinal dose/severity, report pairwise ordering error and rank monotonicity with uncertainty, by artifact and stratum. Do not treat 0–4 labels as equally spaced physical damage. Extent, amplitude and loss of critical structure are separate targets. Include mixed-defect cases: blur may lower ringing but increase missing-line loss, so a detector should preserve both axes rather than force one severity order.

Calibrate confidence/abstention using held-out labeled eligibility and error events. Report coverage–risk curves, abstention rate by stratum and minimum supported denominator. A detector that abstains on every difficult case does not qualify merely because its remaining errors are low. Verify uncertainty intervals against repeats/reference uncertainty, not just softmax values.

Proposed starting goals for a narrowly scoped v0 detector are: lower one-sided 95% confidence bound of at least 0.90 sensitivity for independently defined severe defects; upper bound no more than 0.05 false-alarm probability on eligible clean scene cases; and demonstrated coverage on a preregistered minimum fraction of eligible cases. These are negotiation targets. The pilot must establish whether they are feasible and meaningful; fix final values **before** qualification. An unqualified detector remains exploratory and cannot decide the primary algorithm ranking.

Sample-size warning: with zero false alarms in `n` independent clean cases, the exact one-sided 95% upper binomial bound is `1 - 0.05^(1/n)`. About 59 independent cases are needed to bring that bound below 0.05. With zero misses, about 29 independent severe cases give a one-sided lower sensitivity bound above 0.90. These calculations assume independent Bernoulli cases; correlated crops/frames cannot be substituted. Multiple artifact/material strata and actual errors require more evidence. The 24-group pilot cannot certify rare-defect rates across the whole taxonomy.

## 5. Blinded human protocol

Recruit experienced macro stackers for fidelity/artifact judgments and, if the intended audience is broader, a separately reported nonexpert panel for perceived usefulness. Proposed pilot: 5–8 expert raters, calibrated on disjoint training scenes; choose final panel and trial count using pilot reliability/variance. This count is not a universal adequacy claim.

Freeze task instructions and target convention. For example: preserve all directly supported subject detail and correct color/geometry while minimizing visible stacking defects; background policy is shown explicitly. Explain that a sharper-looking wrong code is worse for factual fidelity. Do not force a fidelity judgment in unidentifiable regions.

Use two blocks to avoid conflating aesthetics with evidence:

1. **Anonymous output-only preference:** paired outputs on the same target grid; randomized/counterbalanced side and order; choose left/right/tie/cannot judge and confidence. Start with intended viewing size, then allow linked 100% inspection under a fixed policy. No source or algorithm name is shown.
2. **Source-assisted artifact/factual audit:** provide synchronized original frames, independent target labels or reference where available, and fixed boundary/structure ROIs. Collect artifact presence, extent/mask, 0–4 anchored ordinal severity, evidence class and uncertain/not-applicable. Task-based random-code/line counting has a correct-answer endpoint distinct from preference.

Use the same display transform for all outputs. Log display resolution, physical size, viewing distance/pixels-per-degree, brightness/white point, ambient lighting, zoom and interpolation. Inspect actual decoded saved output at 100% as well as full composition. Do not let automatic preview tone mapping vary by candidate. A remote panel without controlled displays can still give useful evidence, but must be reported as such.

Use balanced incomplete pairing when all pairs are too costly; keep its comparison graph connected and choose pairs before seeing scores. Include blind repeats, identity/easy controls, and balanced challenging near-ties. Predeclare exclusion rules based on task understanding, controls and repeat consistency, not agreement with the favored method. Log raw ratings and retained/excluded counts. Limit sessions and counterbalance fatigue/order; set exact session length after the pilot.

Report inter-rater and intra-rater reliability by artifact: agreement tables, ordinal agreement statistic with confidence intervals, pairwise disagreement and uncertain rates. A mixed-effects ordinal/preference model can include scene and rater effects; Bradley–Terry-style models need tie handling and identifiability checks. [Q23] Also publish raw scene-level win/tie fractions so rankings are not hidden behind a model. Near-ties remain near-ties when intervals overlap; do not force a total order.

ITU subjective/recognition protocols are useful methodological prior art. P.910's current 2026 edition was listed as prepublished/restricted during this review; the public 2023 edition is superseded. P.912 (2016) is current recognition-task guidance. This macro still-image/source-inspection protocol is an adaptation, **not a claim of ITU conformance**. [Q20–Q21]

## 6. Statistical comparison and power

Predeclare primary endpoints, artifact gates, material/difficulty strata, supported-domain thresholds, missingness treatment and practical minimum effect. Use paired scene-group differences for methods evaluated on the same scenes. Bootstrap independent ancestry groups, keeping all conditions/crops/seeds/raters nested; a hierarchical model or nested resampling can separate these sources of variation.

Report effect sizes and intervals, not p-values alone. For metric-human validation use held-out scene-level rank/pairwise accuracy and calibration; report where the metric prefers a known worse output. For a final claimed improvement require practical relevance and no unacceptable loss on critical axes. Use a predeclared multiplicity policy for confirmatory endpoint families; leave exploratory findings labeled exploratory.

Select final sample size from pilot variance and desired interval width or decision power. A rough planning relation for a paired mean difference is `n ≈ (1.96*s/w)^2`, where `s` is independent-group paired standard deviation and `w` desired 95% half-width; refine with the actual stratified/non-normal design. Do not fabricate power from thousands of correlated pixels or assume one sample-size formula covers tail risk and human preference.

Report representative field-test estimates separately from enriched challenge performance. Preserve difficult scenes and failures in cohort accounting. Any manual retouch or per-scene parameter selection has a declared effort/search budget and a separate result, because it changes the intervention being compared.

## 7. Release gates

Promote v0 only after all five gates pass for its declared scope:

1. **Contract correctness:** independent decode, range/color, coordinate, mask, precision and score invariance tests pass on known targets; configuration and hashes are frozen.
2. **Reference validity:** target uncertainty and source support are documented; analytic checks and independent render/calibration comparisons expose shared generator/scorer mistakes.
3. **Detector qualification:** prespecified sensitivity/false-alarm/coverage, monotonicity and localization goals pass on untouched groups and real failures; unsupported strata are marked.
4. **Adversarial and human validity:** no unresolved severe gaming failure within claimed scope; blinded macro experts support endpoint decisions and near-ties/uncertainty remain explicit.
5. **Reproducibility and leakage audit:** independent rerun, scene/asset splits, checkpoint/metric provenance, failure accounting and all preregistered choices are reviewable.

Passing these gates supports a scoped empirical claim. It does not prove artifact-free performance on every future lens, subject, material or acquisition condition. Revisions require versioned results and new qualification evidence; do not silently change metric meaning after publishing a leaderboard.
