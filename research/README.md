# Macro focus-stacking research

Research date: 2026-11-15. Scope: build an open-source, high-quality macro focus-stacking benchmark before choosing or changing an algorithm.

## Review sequence

1. `01-artifacts/`: documented failure modes, causes, and visual evidence.
2. `02-algorithms/`: state of the art, reproducibility, and candidate-base assessment. Starts after artifact review completes.
3. `03-measurement/`: robust evaluation design. Starts after algorithm review completes.
4. `04-datasets/`: dataset discovery and suitability audit. Runs alongside measurement design.
5. `05-synthesis/`: consolidated benchmark specification, decisions, and next steps.

## Evidence rules

- Prefer papers, author repositories, official dataset pages, and original software documentation.
- Record direct URLs, access date, source type, and the claims each source supports.
- Separate published findings, authors' claims, code inspection, and proposed methods.
- Treat multi-focus image fusion, long macro stacks, microscopy, and light-field data as distinct domains.
- Do not imply that an all-in-focus reference is physically exact, or that a proposed metric has been validated.
- Mark download status, licensing uncertainty, missing evidence, and unresolved questions explicitly.
- Store all notes, manifests, scripts, and intermediate research under this directory.

This index will be updated as reviews finish.
