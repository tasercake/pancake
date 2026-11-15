# Phase 4 dataset handoff

Completed **2026-11-15**, alongside measurement design, after phases 1 and 2 completed.

## Deliverables

- [dataset-review.md](dataset-review.md): actionable portfolio, primary-source audit, reference semantics, measurement fit and remaining gaps.
- [datasets.json](datasets.json) and [dataset-catalogue.csv](dataset-catalogue.csv): 28 matching records with 19 fields covering counts, access, precision, reference type, source/download/license routes, ancestry and recommended role.
- [split-protocol.md](split-protocol.md) and [acquisition-plan.md](acquisition-plan.md): grouped train/validation/test design, metric-development separation, intake and missing controlled-data plan.
- Scene/split JSON Schemas and clearly fictional examples: [scene schema](scene-manifest.schema.json), [scene example](scene-manifest.example.json), [split schema](split-manifest.schema.json), [split example](split-manifest.example.json).
- [sources.md](sources.md), [source evidence summaries](source-evidence-summaries.json), metadata snapshots and [validation.txt](validation.txt). Derived metadata manifests preserve exact remotely listed file IDs and upstream partitions.

## Strongest new data findings

1. **Sood 999-frame macro:** creator API/card/LICENSE/checksums verified, with 999 CR3 and 999 TIFF paths. Explicit CC BY 4.0. One independent sweep. TIFFs are 8-bit sRGB; RAW/TIFF grids differ. Advertised prefix subsets change focal coverage and cannot support equal-coverage length claims. [Remote manifest](hf-999-file-manifest.csv) has 1,998 creator hashes; no image bytes were acquired or locally hash-verified.
2. **Amber microscopy:** primary Figshare records explicitly license optical source stacks CC BY 4.0. Mite record has nine source ZIPs and states up to 200 focal planes; not 18 source ZIPs. Spider record contains duplicate optical ZIP entries with equal provider MD5. Confocal CZI is a different modality. Archive names/body-part views do not establish independent specimens; conservatively group shared amber piece/specimen/session.
3. **Release defects and duplicates:** EDMF current tree has 950 A/1,000 B paths, missing A IDs 543–592. [Audit](EDMF-pair-audit.csv) identifies all 50 missing counterparts. [Lytro/MFIFB overlap](Lytro-MFIFB-exact-blob-overlap.csv) verifies 40 identical Git blobs, covering all 20 Lytro pairs; do not count those as new independent scenes.
4. **Richer real-stack leads:** Learning to Refocus describes 1,637 nine-frame RAW scenes with Helicon pseudo-AiF and split files; archive listing/terms still need intake. LearnAF supplies 49 focal positions and multi-view geometry, but release readme counts differ from project headline. Neither alone replaces high-magnification, hundred-frame macro tests.
5. **Truth and split hazards:** MFFW full original set is 13 pairs plus six multi-source sets; manual references are not physical truth. MattingMFIF DOF-off renders are model-defined references with sampling uncertainty. DFV authors explicitly report original DDFF training/validation scene overlap. Public-code licenses and public-image redistribution rights are distinct evidence.

## Measurement integration

Canonical IDs agreed with measurement reviewer: `scene_id`, `leakage_group_id`, and scene `split.split_role`. Run records join `run.split_role` to scene `split.split_role` and split `groups[].split_role`; they also reference scene/split manifest hashes. Scene examples use the same fictional `EXAMPLE_ONLY_scene_001` / `EXAMPLE_ONLY_family_A` / `unassigned` identifiers as the run example. These example files were not changed after the measurement reviewer computed their hashes.

Scene schema covers source-frame order/hashes; capture/RAW/range/gamma/grid metadata; target viewpoint/focus/background/material/temporal conventions; independent mask provenance and uncertainty; direct/contaminated/indeterminate source support; calibration/repeat links; reference definition; rights and intake eligibility. Schema validation cannot prove truth, independence, rights or source hash agreement.

Agreed pilot: **12 controlled scene/asset families + 12 real macro groups for metric development**, not a powered final test. Qualification and final algorithm test need new groups. Proposed new algorithm-corpus allocation is 60/20/20 by independent ancestry components, subject to final counts/power/strata. Preserve upstream official holdouts; never split frames/crops/variants of one scene. Public legacy/challenge panels remain separate from a sealed final test and representative field captures.

## Completed checks and limits

Both JSON Schemas and fictional examples pass Draft 2020-12 validation. Registry JSON/CSV values match, with 28 unique IDs. Exact upstream Cervix93 labels were retrieved: [93 FOV IDs](Cervix93-official-split.csv), 69 training/24 test, 1,602 listed source frames. Patient/slide independence is not thereby verified. All current local Markdown links resolve. Metadata snapshots have a SHA-256 manifest.

**No image archives downloaded, image decoding, dataset curation, final new split assignment, metric qualification, algorithm runs, quality rankings, author contact or access-gate agreements occurred.** Some dataset routes/terms remain unverified and are labeled. Research and a concrete intake/split design are complete; actual benchmark acquisition and validation remain next-stage work.
