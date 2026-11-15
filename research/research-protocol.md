# Research protocol

## Objective and boundaries

Design an evaluation program capable of distinguishing high-quality macro focus stacks from outputs with subtle but serious defects. The immediate deliverable is research and a concrete benchmark design, not a claim that a metric or algorithm is already validated.

The intended task includes long photographic stacks, fine hairs, depth discontinuities, specular and translucent materials, real sensor noise, focus breathing, and rail-induced viewpoint changes. Small two-image multi-focus benchmarks remain useful only where their assumptions apply.

## Required evidence in each review

Each material claim should identify a primary source, an exact link, what was inspected (full text, abstract, code, manual, or landing page), and what remains unverified. A paper's reported superiority is a claim under that paper's protocol, not a global ranking. Preprints and published papers must be labeled separately. Access failures must be explicit.

Review scope is current public evidence discoverable on 2026-11-15. Reviews are broad, but cannot prove exhaustive coverage or the absence of better unpublished methods.

## Handoff sequence

The artifact review must finish before the algorithm-review agent starts. The algorithm review must finish before the measurement and dataset agents start. Those last two reviews run together and exchange requirements. Final synthesis must reconcile their recommendations instead of treating them as independent lists.

## Questions final synthesis must answer

1. What defects matter, and which can be judged without a trusted reference?
2. What evidence would show that a measurement rewards real improvement instead of noise, sharpening, cropping, or hallucinated detail?
3. Which existing datasets meet the task, access, license, provenance, and reference-quality requirements?
4. How will train/validation/test assignment prevent related scenes, crops, render assets, or development decisions from leaking?
5. What is ready to use, what is only proposed, and what must be captured or generated next?

## Source handling

Store original research notes, structured registries, metadata, and any acquired samples under `research/`. Link externally hosted datasets before making large downloads. Do not treat a public download as permission to redistribute. Preserve immutable software revisions so future experiments can be reproduced.
