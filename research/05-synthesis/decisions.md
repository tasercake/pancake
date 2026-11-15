# Benchmark design decisions

Research date: 2026-11-15. These are design decisions for a proposed benchmark, not empirical validation results. Detailed evidence lives in the four numbered reviews.

| Decision | Reason | Status |
|---|---|---|
| Evaluate a vector of artifact burdens, support/coverage, failures, and blinded overall preference. | A scalar sharpness or similarity score can reward noise, tone changes, blur, cropping, or incorrect detail. | Proposed evaluation policy; qualification still required. |
| Use full stacks and explicitly declare target view, intended focus domain, material, and temporal policy. | Macro optics, occlusion and motion can make several plausible outputs incompatible; short pairs do not represent every long-stack condition. | Required task contract inferred from primary evidence. |
| Keep controlled-input fusion and native delivered-pipeline results separate. | Decoding, registration, tone mapping, precision and export differ materially between inspected implementations. | Code-backed motivation; experimental effect unmeasured. |
| Keep challenge tests separate from representative field tests. | Deliberate oversampling of difficult boundaries is useful for finding failures but cannot establish their population prevalence. | Proposed sampling policy. |

## Non-negotiable distinctions

- **Clean source support versus reconstruction:** a desired source pixel can be absent while complementary measurements still support reconstruction. Conversely, plausible reconstruction does not certify unobserved detail.
- **Model-defined versus real-world truth:** analytic cases can have verifiable exact targets under stated assumptions; finite Monte Carlo renders retain numerical uncertainty. Neither automatically defines correct real-lens appearance. A pinhole target does not silently define correct finite-aperture glossy or transparent appearance.
- **Reference agreement versus quality:** matching a software-generated reference can reward inherited errors. Human preference also does not prove factual recovery.
- **Public test versus sealed test:** a published benchmark may have influenced methods, weights, hyperparameters or metric selection. Preserve its role and add fresh held-out groups for stronger claims.
- **Frames versus independent samples:** all frames, crops, repeated captures and synthetic variants from a related scene family stay in one partition.

## Research boundary

No external algorithm was changed or run. No metric detector has been qualified. Dataset discovery does not mean a corpus has been downloaded, rights-cleared, deduplicated, annotated or finally partitioned. The final plan must retain these status labels.
