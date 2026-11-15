# Dataset review for a macro focus-stacking benchmark

Research cutoff: **2026-11-15**. Phase 4 ran alongside measurement design after artifact and algorithm reviews completed. Evidence IDs resolve in [sources.md](sources.md); all 28 records appear in [datasets.json](datasets.json) and [dataset-catalogue.csv](dataset-catalogue.csv).

**Result:** useful data already exists, including real RAW stacks and optical microscopy stacks with hundreds of frames. No inspected existing corpus supplies all required independent scene diversity, controlled macro optics, exact references, artifact labels, precision, and clean train/validation/test ancestry. Assemble several complementary tracks, then capture the missing controlled data.

This is a catalogue and acquisition/split design. No image archive was downloaded, no source image decoded, no algorithm run, and no final new split assigned. Metadata availability does not establish complete binary-download access or image quality. Proposed measurements remain unvalidated.

## 1. Recommended starting portfolio

| Priority | Dataset | Useful evidence | Main limitation |
|---|---|---|---|
| First real RAW development source | LSFD / FocusDeep (D01) | 94 real bursts × 30 RAW frames; original 84/10 train/test split; photographic resolution. | Dataset terms still need clarification; some macro bursts lack complete focal coverage; Helicon pseudo-targets. |
| First long-stack stress source | Sood 999-frame macro (D02) | Direct listing verifies 999 CR3 files, 999 TIFFs, explicit CC BY 4.0, creator checksums. | **One scene**, not 999 independent training examples; no trusted all-focus target. |
| First high-magnification challenge sources | Amber mite and spider optical stacks (D03–D04) | Explicit CC BY 4.0; optical source stacks, fine structures, deep focus and difficult material. Mite description states up to 200 focal planes. | Immersion, transmitted light, amber and prior processing differ from dry reflective macro; several views share specimens. |
| First radiometric and scale controls | EDMF, HRMFF, UHD-MFF (D10–D13) | Exposure differences and high-resolution pairs expose seams, border errors and scale issues. | Short pairs; incomplete EDMF release; terms and source ancestry vary. |
| First expanded photographic training lead | Learning to Refocus (D18) | Paper describes 1,637 real nine-frame scenes, DNG originals and calibrated full-resolution processing. | Archive listing not accessible in this review; dataset terms unresolved; Helicon pseudo-target and phone-sized blur. |

These priorities are engineering judgments from source suitability, not measured quality rankings. Retain legacy Lytro/MFFW/MFIFB only as a public regression panel. Use synthetic and controlled-real targets to qualify measurements independently.

## 2. Real long stacks: useful, but independence is scarce

**LSFD.** The original paper reports 94 bursts with 30 frames each and a split of 84 training/10 test bursts. Its archive metadata is available and reports approximately 103 GB decimal. The paper explicitly discusses incomplete macro focal coverage and errors inherited from Helicon pseudo-targets. This makes LSFD valuable for real blur, RAW processing and noise studies. Pixel agreement with its target primarily measures agreement with that teacher. Preserve original test bursts; derive validation only from original training ancestry after intake. Do not use random crops as an independent split. [D01](https://arxiv.org/html/2311.17846v1)

**Sood long macro sweep.** The creator supplies one Canon R5 Mark II/RF 100 mm macro sweep at f/2.8, ISO 400, approximately six minutes. API, card, explicit license and checksum text were acquired; image bytes were not. TIFF exports are **8-bit sRGB**, not a high-bit-depth benchmark. RAW and TIFF dimensions differ, requiring a declared decode/grid mapping. Advertised 10/100/500-frame subsets are contiguous prefixes; they change focal range and cannot be used as equal-coverage length ablations. Uniform focal-range thinning must be designed from inspected focus coverage. All variants remain one group. [D02](https://huggingface.co/datasets/jjjsood/focus-stack-sample)

[hf-999-file-manifest.csv](hf-999-file-manifest.csv) records the 1,998 remotely listed source paths with creator-provided SHA-256 values and pinned URLs. These hashes have **not** been checked against locally acquired image bytes. The preview stack is a demonstration output, not ground truth.

**Optical amber microscopy.** Primary Figshare records provide explicit attribution licenses and source ZIPs. The mite record has **nine source-stack ZIPs**, separate scale bars and existing fused JPEGs; its creator states up to 200 focal planes. The spider record includes a duplicated optical ZIP upload with the same provider MD5, plus separate confocal CZI files. Archive interiors were not inspected; secondary claims of exact frame totals were not adopted. These are excellent difficult-material and fine-structure leads. They are not representative prevalence samples, and existing fused JPEGs are pseudo-references. Treat confocal imaging as a separate modality, not truth for brightfield/DIC image appearance. [D03](https://doi.org/10.6084/m9.figshare.14707077.v1), [D04](https://doi.org/10.6084/m9.figshare.30771638.v1)

## 3. Real photographic expansions beyond traditional fusion pairs

**Learning to Refocus** describes 1,637 scenes with nine focus settings, five iPhones and 4032 × 3024 RAW capture; original split is 1,474/163. Its AiF reference uses Helicon depth mode after ACR development, calibrated radial correction and breathing compensation. Source DNGs and processed images serve different benchmark tracks. The author code includes split files, but they were not deserialized; the Sync download listing was inaccessible. This is a promising richer source once access, terms and scene/object ancestry are resolved. The released captured stacks are distinct from the paper's generated diffusion outputs. Generated stacks must never enter the factual-reference track. [D18](https://arxiv.org/html/2512.19823)

**LearnAF** offers 49 focal positions, simultaneous multi-phone views, poses, dual-pixel measurements and depth confidence. Its release readme describes 351 training/47 test sweeps, while the project headline says 510. Record both until actual intake resolves the version difference. Estimated depth has an explicit scale caveat; it is not exact optical geometry. Five simultaneous phone views are linked observations, not five independent scene families. [D17](https://storage.googleapis.com/cvpr2020-af-data/LearnAF%20Dataset%20Readme.pdf)

**Mobile Depth** supplies original/calibrated and aligned focal-stack routes. It is useful for handheld alignment and viewpoint change, with a large domain difference from true high-magnification macro. Original release counts and terms were not confirmed. Keep its reconstructed AiF/depth products distinct from independent references. [D19](https://www.supasorn.com/dffdownload.html)

## 4. Conventional pair datasets: regression and diagnostics

**Lytro** contains 20 pairs and four triples in the author tree; all are low-resolution compared with modern macro capture. **MFFW** has 19 sets in the original paper: 13 pairs and six sets with three or more sources. Its pair focus polygons and Photoshop-edited references are useful for annotation and boundary evaluation, but the authors explicitly allow disagreement with the real scene. Neither set substitutes for long-stack testing. [D05](https://github.com/mnnejati/LytroDataset), [D06](https://arxiv.org/html/2002.04780)

**MFIFB** lists 105 pairs and 3,150 comparator outputs. Metadata comparison verifies **40 identical Git blob identities** shared with the Lytro author tree, covering all 20 Lytro pairs; [exact overlap records](Lytro-MFIFB-exact-blob-overlap.csv) preserve those paths. Thus, adding MFIFB to Lytro does not add 105 independent scenes. Existing fused outputs can supply diverse candidate failures for a blinded study, subject to rights and common source grouping; they are not references. **Real-MFF** uses light-field derived pairs/reference constructions and includes 443 Stanford light fields. **MFI-WHU** derives 120 pairs from public originals whose individual provenance is not enumerated in its README. A repository MIT file alone does not resolve every upstream image's terms. [D07](https://arxiv.org/html/2003.12779), [D08](https://github.com/xingchenzhang/MFIFB), [D09](https://github.com/HaoZhang1018/MFI-WHU)

**EDMF release defect:** the paper describes 1,000 pairs, with first 500 used for training and remaining 500 for testing. Its current complete GitHub tree contains 950 Source_A paths and 1,000 Source_B paths; A counterparts for numeric IDs **543–592** are absent. [EDMF-pair-audit.csv](EDMF-pair-audit.csv) records every pair's listing status. There are 950 matched filename IDs, not 950 decoded or quality-approved pairs. Do not silently invent missing frames or report the release as 1,000 usable pairs. Original row-order-to-numeric-ID correspondence remains provisional until release documentation confirms it. Registered black borders need independent validity masks. [D10](https://www.mdpi.com/1424-8220/24/22/7287)

**HRMFF** verifies 20 JPEG pairs; **UHD-MFF** reports 150 real and 1,800 depth-blurred synthetic 3840 × 2160 pairs, with synthetic 1,500/300 training/test. Drive folder structures were listed. They are useful resolution and tiling controls. High resolution does not supply long-stack, RAW, highlight, or exact-geometry truth automatically. [D11](https://github.com/zwy0913/HRMFF), [D12–D13](https://arxiv.org/html/2606.31242)

## 5. Synthetic references and light fields

**MattingMFIF** renders photographic cutout planes in Blender. Its original paper reports 10,000 training and 200 test samples, 4K output, and a DOF-off reference. This is a model-defined layered render with numerical sampling uncertainty; it is not general macro ground truth. Cutout RGB can contain baked lighting, texture and pre-existing blur; a plane is not a curved translucent hair or refractive droplet. Foregrounds, backgrounds and HDRIs must remain grouped across splits. A directly usable corpus archive and complete asset terms were not located. [D14](https://arxiv.org/html/2510.19581)

**StackMFF recipes** and **FoD500/DefocusNet** are useful generation leads. The former combines multiple source-image/depth datasets, sometimes using estimated depth. The latter supplies a Blender generation scene, five-focus rendered samples and depth, with an author-described 400/100 split. Measured/estimated depth, source RGB and reconstructed AiF are distinct reference types. Neither establishes exact macro performance under a real lens. [D15](https://github.com/Xinzhe99/StackMFF-V2), [D20](https://github.com/dvl-tum/defocus-net)

**HCI/Konstanz 4D light fields** offer known rendered geometry, masks and camera configuration under a stated license. Current site lists 28 scenes; original paper used 24. These are dense pinhole-view samples, not an ordinary focal sweep. Refocus through a sampled light field defines another renderer with angular-sampling limitations. Release history includes renderer-noise and depth corrections, so even synthetic truth needs a fixed version and convergence audit. Reserve official hidden test scenes. [D16](https://lightfield-analysis.uni-konstanz.de/)

**DDFF12** illustrates an existing leakage trap: the DFV authors explicitly report original training/validation stacks from the same physical scene and provide a different scene-disjoint preparation. LFSD's “ground truth” is saliency segmentation, not exact fusion pixels. EPFL optical light fields have research-only terms. These resources remain secondary geometry/refocus sources, with explicit ancestry and reference conventions. [D21](https://github.com/fuy34/DFV), [D25](https://nianyi-li.github.io/LFSD/), [D26](https://www.epfl.ch/labs/mmspg/downloads/epfl-light-field-image-dataset/)

## 6. Microscopy and quality-rating resources

**Cervix93** has 93 optical stacks, 1,602 source PNGs and 93 existing EDF images in its complete tree. Exact official 69/24 split IDs were retrieved and preserved. Nuclear point/grade labels concern another task; EDF images are composites. Transmitted translucent cells need a material-specific track. Confirm slide/patient grouping and terms before training. EPFL's historical EDF examples are another useful source lead, but several originals require a request. A newer HF insect microscopy set is manually gated; no contact-sharing agreement was accepted. [D22](https://github.com/parham-ap/cytology_dataset), [D23](https://bigwww.epfl.ch/demo/edf/), [D24](https://huggingface.co/datasets/Data-set/microscopy-focus-testset)

**MEMF** is a relevant subjective-rating lead from Hassen/Wang/Salama, but a current standalone dataset download, ratings and terms were not verified. Do not confuse it with Waterloo's 17-scene, 136-output **MEF-only** subjective database. The latter can test radiometric preference assumptions, but cannot establish focus, hair or macro artifact correctness. [D27](https://doi.org/10.1109/TIP.2015.2428051), [D28](https://ivc.uwaterloo.ca/database/MEF/MEF-Database.html)

## 7. Remaining data gaps and measurement fit

No inspected source provides a large, cleared collection of **independent** high-magnification real scenes with calibrated thin structures, correct opacity/visibility, repeated RAW captures, trustworthy sharp/color/highlight references and sealed test ancestry.

| Measurement requirement | Existing support | Missing evidence to collect |
|---|---|---|
| Boundary halo width, ringing, missing/false filaments | MFFW annotation; amber structures; synthetic geometry | Independent depth-edge/line masks, known contrast/width and uncertainty; near/far-side profiles. |
| Factual fine-detail fidelity | RAW sources, controlled/rendered geometry | Coded microtexture, repeated capture, independently resolved detail and recoverability labels. |
| Highlight, color, clipping and noise | LSFD RAW; Sood CR3; EDMF exposure; phone data | Calibrated RAW range/response, flat/color/gloss controls, repeated dark/flat captures and illumination records. |
| Geometry, coverage and long-stack failure | Sood length; amber stacks; multiview phone metadata | Independent transforms/visibility, calibrated focal steps and target view; equal-coverage length variants. |
| Validated artifact severity and perceived quality | Existing subjective-study leads and diverse method outputs | Blinded macro-specific local annotations/preferences on unseen groups and methods; reliable negative controls. |

Use [acquisition-plan.md](acquisition-plan.md) to fill these gaps, [split-protocol.md](split-protocol.md) to preserve independence, and [scene-manifest.schema.json](scene-manifest.schema.json) to record evidence. The measurement review owns exact metric definitions, synthetic target contracts and detector qualification. Dataset labels must supply their inputs without claiming unsupported ground truth.
