# Review scope and evidence method

Research date and cutoff: **2026-11-17**. This is a new review of synthetic data generation, commissioned after the earlier benchmark review. It does not reopen the earlier algorithm selection or dataset acquisition work.

## Scope

The intended sample is an ordered or unordered set of `N` finite-depth-of-field observations, a declared all-in-focus target, and enough latent scene/camera information to evaluate errors at pixel and structure level. The review asks how to generate these samples procedurally, which existing work is reusable, and which truth claims the simulation can actually support. Training use is secondary to evaluation integrity. Haze, dust, vibration and subject motion are extension tracks.

The existing `research/README.md`, synthetic-reference contract, dataset review and synthesis were read first to identify overlap. Four agents then independently searched optics/renderers, focal-stack synthesis literature, procedural scene/sensor generation, and dynamics/imperfections. The coordinating review searched across those boundaries and developed the implementation and validation synthesis. Earlier recommendations were context, not a search filter. The older protocol's sequential handoff describes that earlier commission; this review follows the user's explicit request for several agents working on separate topics.

## Search coverage

Search families included synthetic focal stacks, all-in-focus references, multi-focus fusion and defocus boundaries; depth-from-focus/defocus and dual-pixel simulation; compound lenses and wave/ray optics; light-field refocusing; procedural macro scenes and material generators; spectral sensor/RAW/ISP simulation; moving-subject fusion and temporal targets; rolling shutter and camera motion; volumetric degradation and dust. Recent searches explicitly included 2025–2026, alongside older foundational image-formation work. AlphaXiv discovery was also used to expand coverage; generated paper summaries were discovery aids, not primary evidence.

Search results are not an exhaustive systematic-review corpus. There is no defensible universal ranking of generators in the retrieved evidence. “State of the art” here means the most relevant public approaches found and inspected, with the scope and status of each stated. Preprints, published results, author code, documentation and our design proposals are distinguished.

## Evidence rules

- Use primary papers, author repositories/project pages, and official documentation. Follow promising search hits to the primary source.
- Record whether evidence is full paper, relevant sections, abstract, README, implementation, or metadata. A paper reporting a feature does not establish that its public implementation contains that feature.
- Record access/release failures and disagreements between paper and repository. Availability of code, data, weights and redistribution permission are separate questions.
- Repository pins describe the inspected code; they are not executed dependency locks. Moving documentation URLs are dated and must be pinned during implementation.
- Keep paraphrases concise and attribute claims near their use. Proposed mathematics, targets, acceptance criteria and architecture are labeled as design work.
- Do not import authors' method rankings as universal findings. No published quality/runtime result is a new measurement made in this repository.

## Deliverables and boundary

The overview links four topical reviews, a proposed pipeline, a generator/evaluation validation design, and machine-readable source ledgers. Local document/link/JSON checks are reported separately from scientific validation. This review does **not** implement or run a renderer, generate a dataset, calibrate a lens, train a network, establish algorithm rankings, or verify all referenced dataset bytes. No expensive asset or dataset downloads are required to read the package.
