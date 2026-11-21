# Train, validation and test protocol

Status: **proposed design**. No new final corpus or split has been created.

## 1. Unit of assignment

Assign **independent scene/specimen/asset families**, never frames, crops, focal planes, method outputs or rendered random seeds. Build a provenance graph before assignment. Connect records when they share any of:

1. Physical specimen, amber piece, patient/slide, location/scene, capture session, burst repeat or simultaneous camera rig.
2. Original photograph, light-field capture, frame sequence, foreground cutout, identifiable mesh/object asset, reusable texture/background image or HDRI family.
3. Crop, resize, demosaic, exposure/noise change, focus thinning, slab, registration, manual retouch, pseudo-reference, refocus or render derivative.
4. A near-duplicate identity confirmed by exact bytes, normalized-image hashes, feature matching or manual review.

Use connected components as `leakage_group_id`. This is transitive: if A shares a foreground with B and B shares a background with C, A/B/C cannot be divided across train/test. Seeds alone do not create independence. A common procedural primitive is not automatically one asset identity; define geometry-family holdouts separately. Shared substantive photographs/textures do require grouping. If ubiquitous HDRIs/backgrounds create a giant component, acquire disjoint asset pools or remove/rebuild examples; do not ignore edges to obtain attractive split ratios.

Record uncertain ancestry explicitly. Quarantine uncertain cross-partition matches until resolved. “No duplicates found” describes the tested detectors and review, not a proof that all semantic overlap is absent.

## 2. Keep evaluation roles distinct

| Role | Permitted use | Prohibited inference |
|---|---|---|
| `algorithm_train` | Train model weights or learn fusion parameters. | Training-set quality does not establish generalization. |
| `algorithm_validation` | Select architecture, parameters, stopping rule, runtime budget and reported configuration. | Repeated validation tuning does not make it final test. |
| `algorithm_test` | Frozen, limited final comparison using frozen metrics/adapters/selection policy. | Test-driven tuning invalidates the original final-test claim. |
| `metric_development` / `metric_qualification` | Develop/calibrate detectors on the former; qualify frozen detectors on different groups/methods with hidden labels in the latter. | Metric development on algorithm-test outputs leaks evaluator design. |
| `external_regression` / `external_stress` | Public legacy comparisons, reproducibility and targeted challenge checks. | Public reused examples are not a pristine sealed test or representative failure-prevalence sample. |

For the strict benchmark, metric-development/qualification groups remain disjoint from final algorithm-test groups. Prefer a dedicated metric dataset. If development uses some algorithm-training groups, disclose that overlap and still keep metric qualification and final test independent. Shared clean images with different injected defect seeds remain the same group.

The proposed **12 controlled scene/asset families + 12 real macro groups** are a metric-development pilot, agreed with the measurement reviewer. They are not a powered final benchmark and must not be forced into a small train/validation/test split. Qualify on newly acquired groups after metric settings freeze. Choose qualification size using observed errors, rare-defect coverage and uncertainty targets.

## 3. Proposed new-corpus allocation

Default planning target for a sufficiently large **new algorithm corpus**: **60% training / 20% validation / 20% final test by independent groups**, stratified by broad material, magnification, capture mode, stack length, noise and boundary difficulty. This ratio is a design choice, not a literature-established optimum. For illustration only, 300 independent groups would yield approximately 180/60/60; neither 300 nor those exact counts have been acquired or shown sufficient.

Small strata need minimum independent-group coverage or must be reported as exploratory. Do not move test groups after viewing algorithm outputs. Ratios may deviate when preserving ancestry or official holdouts; report the resulting counts of groups, scenes, stacks and frames separately.

Reserve separate domain holdouts where feasible: a camera/lens family, magnification regime, material family, capture session/site and new synthetic geometry/texture families absent from training. Do not select an OOD camera after finding which camera a preferred method handles well. Distinguish in-domain test estimates from OOD challenge estimates. Population-weighted field performance requires a representative capture sample, while deliberately difficult cases need a separate challenge scorecard.

## 4. Existing datasets: preserve provenance and existing holdouts

| Dataset | Existing evidence | Proposed handling |
|---|---|---|
| Sood 999-frame stack | One scene; HF labels all frame rows `train`. | All frames, CR3/TIFF variants and thinned subsets remain one `external_stress` group. Random-frame splitting is invalid. |
| Amber spider | Repeated specimen views and a duplicate spider ZIP upload. | Group shared specimens/amber piece; remove duplicate content by provider/hash/intake audit. Number of archives is not number of independent specimens. |
| EDMF | Paper first 500 train / remaining 500 test; current release missing Source_A IDs 543–592. | Preserve the source's published partition rule. Quarantine missing pairs. Mapping paper row order to numeric filename IDs must be confirmed before claiming exact official assignment. Do not refill missing test cases with training cases. |
| FoD500 | Author first 400/remaining 100 split. | Preserve original split, then audit mesh/texture/HDRI sharing. An official sample split is not proof of asset-disjointness. |
| HRMFF | 20 high-resolution pairs in the author tree. | Use for external regression. Audit capture ancestry before reassigning paired images. |
| InfinigenDefocus | Five apertures per rendered view. The release is labeled train. | Keep views and apertures that share a scene seed together. Audit shared assets before assigning new splits. |

## 5. Pretrained-model exposure and test access

Record each method/checkpoint's known training/validation datasets, pseudo-teachers, source-image corpora, depth estimators and foundation models. A public set used in a model's training cannot support an unseen-test claim for that model. Unknown exposure remains `unknown`, not `clean`. Foundation-model pretraining overlap can be difficult to exclude; new unpublished captures and new owned/generated asset families reduce this risk but do not prove absolute semantic novelty.

For a sealed final test, assign groups and freeze manifests before development access. Keep images/labels access-controlled while models and metric definitions are tuned. Open-source code can coexist with a temporarily sealed test; after release, label that version public and obtain a new hidden challenge set for future iterative development. Limit final-test submissions and log every query. If test outputs influence a metric threshold, artifact definition, crop, parameter choice or method change, demote that set to validation/regression and create a replacement final holdout.

## 6. Reproducible assignment and freeze

1. Complete rights/access intake, content hashing, grouping and eligibility before assignment. Preserve upstream IDs and exact raw bytes. Store masks and derivatives with creator/tool/version/hash provenance.
2. Pre-register group definitions, strata, ratios, OOD domains, minimum coverage and representative-versus-challenge sampling. Assign groups using deterministic seeded group-level stratification with a documented algorithm. Select seed before viewing result quality; do not search seeds for preferred rankings.
3. Write immutable versioned JSONL manifests conforming to [scene-manifest.schema.json](scene-manifest.schema.json), plus split policy, group-edge table, preprocessing recipes and SHA-256 hashes. Independently validate no connected component crosses roles that require separation.
4. Freeze metric code/weights, masks, normalization, adapters, runtime/parameter budgets, human study plan and statistical analysis before final test. Attach run hashes to the frozen dataset version.
5. Correct discovered data errors through a new benchmark version and documented migration. Keep old hashes/results; never silently replace a test image. Report exclusions, missing counterparts, failed decodes, uncertain labels and achieved denominators.

Schema validation checks record shape. It does not establish rights, optical truth, correct grouping, hash-to-file agreement, independent masks or adequate statistical power. Those are separate intake and review checks.
