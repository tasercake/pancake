# Macro focus-stacking research

Research cutoff: **2026-11-15**. All four reviews and final integration are complete. Scope: design a defensible quality benchmark and dataset strategy before choosing or changing the stacking algorithm.

**Start with the [integrated benchmark plan](05-synthesis/benchmark-plan.md).** Recommendation: artifact-specific severity/extent measurements, explicit coverage and failure accounting, and blinded expert comparison. Qualify the measurements before optimizing an algorithm against them.

## Research package

| Question | Deliverable |
|---|---|
| What goes wrong? | [Artifact review](01-artifacts/review.md), [20-class taxonomy](01-artifacts/artifact-taxonomy.json), and primary-source ledger. |
| Which algorithms are relevant? | [Algorithm review](02-algorithms/review.md), [29-record method matrix](02-algorithms/method-matrix.csv), and [pinned base audit](02-algorithms/base-comparison.md). |
| How should quality be measured? | [Suite design](03-measurement/benchmark-design.md), [27-measure registry](03-measurement/metric-registry.json), [qualification/human protocol](03-measurement/validation-protocol.md), and [synthetic reference contract](03-measurement/synthetic-ground-truth.md). |
| Which data can support it? | [Dataset review](04-datasets/dataset-review.md), [eight-source catalogue](04-datasets/dataset-catalogue.csv), [split protocol](04-datasets/split-protocol.md), [acquisition plan](04-datasets/acquisition-plan.md), schemas and remote-file audits. |
| What should happen next? | [Integrated implementation order](05-synthesis/benchmark-plan.md#8-implementation-order), [decisions](05-synthesis/decisions.md), and [validation scope](05-synthesis/validation-scope.md). |

The requested sequence was preserved: artifact review first; algorithm review after that; measurement and dataset reviews together afterward. Their handoffs, evidence ledgers and source snapshots remain in the corresponding folders. [review-status.json](review-status.json) records stage and experimental status.

## Key decisions and limits

- Keep focus-stack as a wavelet baseline.
- Start measurement development with **12 controlled scene/asset families + 12 independent real macro groups**. This is a proposed feasibility pilot, not a powered final evaluation.
- For a sufficiently large new algorithm corpus, propose **60/20/20 train/validation/test by independent ancestry group**. Preserve official holdouts and separate metric qualification from final algorithm testing. No new final scene assignment exists yet.
- Existing sources include a licensed 999-frame macro sweep, amber spider stacks, photographic pairs, and synthetic focal stacks. Reference quality, terms, access and independent-group counts vary. See the catalogue and the subsequent [reproducible acquisition pipeline](../bokehforge/docs/data.md).
- The research reviews did not download or decode an image corpus. The subsequent [acquisition and conversion phase](../bokehforge/docs/data.md) preserves native files and creates canonical TIFF sequences, with explicit access/storage failures. **No stacker, detector, renderer or human study was run.** Registries, schemas and acquired data do not constitute validated measurements or an audited final split.

## Evidence and provenance

Each review links primary sources and distinguishes full-text/code inspection from abstract-only or metadata-only evidence. Papers' reported rankings, vendor claims, static code findings and new proposals remain distinct. Synthetic, teacher-produced, manually fused, light-field-derived and controlled physical references retain their different semantics.

[Source snapshots](sources/README.md), [repository pins](sources/repository-pins.json), the [research protocol](research-protocol.md), and phase-specific ledgers preserve provenance. The [independent metric audit](05-synthesis/metric-evidence-audit.md) explains why evaluator qualification and deliberate metric attacks are necessary.

[Structural verification](05-synthesis/research-validation.json) passed registry, schema/example, join/hash, source-ID and local-link checks. These checks validate research artifacts, not stacking quality or detector accuracy.

## Experiments

[Unattended Zerene Stacker](experiments/zerene/README.md) documents how to run PMax and DMap and sweep DMap parameters on the macOS host. Scripts for research experiments live under `research/experiments/`.
