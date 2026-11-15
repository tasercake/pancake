# Measurement handoff

Completed design review: **2026-11-15**, after artifact and algorithm reviews. Measurement and dataset reviews ran together and exchanged data/schema requirements. No detector, algorithm, renderer, image metric or human experiment was run. This is a concrete proposed suite and qualification program, not a validated quality score.

## Decisions

1. Use an evidence-tiered artifact vector, coverage/failure gates and blinded human preference/factual tasks. Preserve local maps, extent, conditional severity, joint burden and tails. Do not release with a mandatory universal scalar.
2. F/E are two pipeline conditions: supplied-registration/fixed-decode fusion core and complete native pipeline. Controlled-truth/real-no-AIF-reference classes are a separate axis. Derived conditions retain one scene/specimen/asset ancestry group.
3. Unknown, not applicable, measurement failure and method failure are distinct. Masks/registration/reference support are fixed independently of the tested output. Missing/cropped/extrapolated regions stay in coverage accounting. Overlapping artifact labels are not additive independent physical-defect counts.
4. Qualify detectors with controlled severity ladders, real failures, clean counterexamples, metric-gaming attacks and unseen scene/method families. Hold metric qualification apart from algorithm train/validation/test. Blinded expert evidence and known factual geometry/codes remain independent checks.
5. Strict truth requires a declared model and verified numerical bounds. Finite Monte Carlo references are converged estimates with uncertainty. Pinhole appearance is not automatically the finite-aperture truth for glossy/transparent materials; absence of a clean source pixel is not an impossibility proof.

## Files

- `benchmark-design.md`: complete suite design and initial pilot.
- `metric-registry.json`: 27 proposed measurements, all A01–A20 mapped, formulas/procedures, units, inputs, confidence/abstention, confounders and attacks.
- `validation-protocol.md`: four data roles, defect ladders, human protocol, scene-level statistics, proposed qualification goals and release gates.
- `synthetic-ground-truth.md`: analytic/verified, converged-render and calibrated-physical reference contracts.
- `run-output.schema.json`: proposed run/measurement contract; foreign keys `scene_id`, `leakage_group_id`, `scene_manifest_sha256`, `split_manifest_sha256` agreed with dataset agent. `run-output.example.json` is a clearly fictional join illustration with no data/run/score claim; role enum matches dataset schemas.
- `sources.md`: 23 primary evidence entries, with exact access limits; `README.md` index; `validation.txt` structural checks only.

## Dataset integration

Data requirements sent to dataset agent include complete original stacks, transitive ancestry IDs, focal order/positions, native and fixed decode metadata, target view/background/temporal/material policy, independent coverage/visibility/boundary/filament/color/highlight labels, reference provenance/uncertainty, repeated/calibration captures and licenses.

The proposed 12 controlled + 12 real independent-group pilot is **metric development only**. It cannot certify rare-artifact rates or supply final algorithm-test evidence. Dataset agent's proposed new-corpus 60/20/20 assignment is separate; final counts follow deduplication and power analysis. Public legacy sets remain external regression with original holdouts/provenance preserved.

The verified 999-frame source is one scene group; its 10/100/500-frame prefix subsets change focal range. Matched-range experiments should thin the full sweep, record spacing/completeness and avoid claiming invariance when information changes. TIFF and RAW grids/precision differ in creator metadata and need actual decode verification. Figshare amber datasets add refractive-material/long-stack stress; supplied fused JPEGs remain pseudo-references and confocal data is a separate modality. No image archives downloaded here.

## Concrete next implementation order

First establish M01/M02 coverage and failure accounting plus M21 precision/color/grid tests. Then implement controlled halo/ringing and filament/color/highlight endpoints (M05/M09/M11/M16/M17), with simple known targets and visible maps. Add supported-source/detail/noise endpoints only when corresponding evidence exists. Keep restricted physics consistency optional until a calibrated compatible latent representation is available; generic 2D reblurring is not accepted as physical proof.

Implement expert viewer/labels and attack qualification before fitting any composite. Candidate-host choice remains provisional; the suite has produced no quality ranking.

## Remaining uncertainties and checks

All macro-specific metrics are unimplemented/unqualified; thresholds, extraction tolerances, confidence calibration, field prevalence weights and final sample sizes remain to be fixed from independent development evidence. Proposed v0 sensitivity/false-alarm goals are planning values, not standards. Current P.910 (2026) full text is restricted; public2023 edition is superseded. P.9122016 task guidance was read, but our protocol makes no standards-conformance claim.

Schema grammar/JSON parsing and artifact/source-reference checks do not validate measurement correctness. Future run validation must also enforce cross-field semantics (e.g. scored denominator cannot exceed eligible denominator, files/hash equality, output dimensions, split membership, mask independence, finite numeric values, exact metric configuration and no test fitting). A public pretrained model's hidden training ancestry may remain unknown; report that uncertainty.
