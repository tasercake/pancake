# Independent metric-evidence audit

Research/access date: 2026-11-15. Root audit conducted while measurement and dataset reviews ran. This is supporting evidence for the final proposal, not metric validation.

## 1. Evaluate the evaluator with disagreement tests

Wang and Simoncelli, [Maximum differentiation (MAD) competition](https://eceweb.uwaterloo.ca/~z70wang/publications/MAD.pdf), *Journal of Vision* 2008, DOI [10.1167/8.12.8](https://doi.org/10.1167/8.12.8). Author-hosted full PDF text inspected, especially methods and stated limits. The method synthesizes stimuli that maximize disagreement between two models while holding one response fixed, then tests human discrimination. The paper explicitly frames this as a way to falsify models, not prove the remaining model universally correct.

Proposed macro adaptation: search a bounded family of halo, blur, noise, contrast, crop, and false-texture perturbations for results that a candidate score treats as equal while calibrated observers find materially different. Reverse the roles of rival scores. Keep ordinary real failures and clean controls: optimized counterexamples alone do not estimate field performance.

## 2. Perceptual features also need attack tests

Siniukov, Kulikov and Vatolin, [Applicability limitations of differentiable full-reference image-quality metrics, v2](https://arxiv.org/html/2212.05499v2), 2023 revision of a 2022 preprint. Original HTML methods and subjective-evaluation discussion inspected. The study trains preprocessing that improves several perceptual scores under JPEG compression while human quality often stays unchanged or worsens. This is compression-domain evidence; it does not establish failure rates on macro focus stacks.

Implication: LPIPS/DISTS and related learned features should be supplementary judges. Include output optimization, localized false detail, and sharpening/noise attacks during macro-specific qualification. Do not treat agreement between several correlated learned metrics as independent proof of factual recovery.

## 3. Metric consensus is distinct from human or physical truth

Liu, Qi, Cheng and Chen, [Rethinking the Effectiveness of Objective Evaluation Metrics in Multi-Focus Image Fusion: A Statistic-Based Approach](https://doi.org/10.1109/TPAMI.2024.3367905), *TPAMI* 2024; [indexed original abstract](https://pubmed.ncbi.nlm.nih.gov/38376964/). Only abstract-level evidence inspected here. The stated validation compares 20 fusion metrics with popular full-reference IQA models by statistical correlation. It also proposes a CNN source/fusion similarity score.

Inference: this supports a useful consistency audit, not independent proof that the selected metrics detect every macro defect or follow expert judgment. If the reference IQA family shares a blind spot, correlation can preserve it. The present benchmark should qualify detectors against known physical errors and blinded annotations in addition to metric agreement.

## 4. Fusion-specific subjective assessment is prior art

Hassen, Wang and Salama, [Objective Quality Assessment for Multiexposure Multifocus Image Fusion](https://doi.org/10.1109/TIP.2015.2428051), *TIP* 2015; [author-hosted PDF](https://ece.uwaterloo.ca/~z70wang/publications/TIP_MEMF_fusion.pdf). Original indexed abstract and first-page text inspected; direct full PDF retrieval exceeded web-tool size limit. The work models contrast, sharpness and structure, with subjective experiments and supplementary real/simulated image data. No exact experiment size or current download status is asserted here.

Its focus/exposure scope is relevant, but transfer to high-resolution, long macro stacks requires fresh validation. Dataset review received this lead. Keep this corpus distinct from other multi-exposure-only quality databases.

## Design consequences

1. Validate artifact detectors and any quality summary before optimizing a stacker against them.
2. Keep reference fidelity, source support, artifact burden, and perceptual preference as separate evidence.
3. Hold out scene families and method families during qualification; use paired scene-level statistics.
4. Freeze masks, registration allowances, score calibration and test policy before final comparisons.
5. Publish applicability and abstention rates alongside scores. Unknown correctness is not zero error.
