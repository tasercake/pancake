# Focus-stacking algorithms: evidence, tradeoffs, and macro applicability

Research cutoff and access date: **2026-11-15**. Phase 2 follows the completed [artifact review](../01-artifacts/review.md). Source IDs resolve in [sources.md](sources.md). [method-matrix.csv](method-matrix.csv) contains comparable method records; [base-comparison.md](base-comparison.md) audits the two proposed bases.

**No method was run in this review. No new benchmark, timing, or quality ranking was measured.** “Reported” means the source authors' result under their own protocol. “Inference” means our assessment from the stated assumptions or inspected implementation. The review covers representative strong and recent methods, not an exhaustive proof of the global state of the art.

## Main findings

The most relevant research direction is **whole-stack fusion with explicit control of source support, geometry, and boundary reconstruction**. Current evidence does not justify choosing a learned pairwise fusion winner as the best long-macro stacker.

Three distinctions matter more than model age:

1. **Source selection versus reconstruction.** Selecting observed values constrains invented intensity values, but cannot remove contamination when no clean source value exists. Reconstruction can combine mixed evidence, but needs independent checks of factual detail.
2. **Two-image fusion versus whole-stack inference.** Repeated pairwise fusion changes the inputs seen by later calls. A fused intermediate is not an original optical observation. Errors, noise, and order effects can accumulate; this is documented motivation in StackMFF and its successors. [A13–A14]
3. **Fusion core versus complete pipeline.** Decoding, registration, exposure correction, resampling, cropping, denoising, tone mapping, and export can dominate measured output quality.

## 1. Define the problem before comparing methods

For a registered stack, a selector produces `F(x) = I_k(x)(x)` or a weighted combination of corresponding samples. A multiscale method selects or combines coefficients instead. A reconstructor estimates an image that need not be a source sample or a convex mixture.

These output constraints are useful but incomplete. A hard selector can preserve the wrong pixel perfectly. A convex blend can blur two sharp but displaced structures. A wavelet result can combine complementary evidence successfully or introduce overshoot. A learned reconstruction can recover information distributed across observations or generate an incorrect but plausible structure. None of these cases can be resolved by the word “sharp” alone.

Long macro stacks additionally require a declared target viewpoint and visible domain. Rail capture changes viewpoint; lens focusing changes magnification and can change optical geometry. A global affine or similarity transform is not a general solution to parallax or occlusion. Material appearance also matters: specular highlights need not follow diffuse correspondence, and transparent objects do not have a single opaque surface label. These are applicability constraints, not claims that every method fails in every such scene.

## 2. Classical methods remain essential baselines

### Complex wavelets / extended depth of field — Forster et al., 2004

The original work uses complex-valued wavelet bases for multichannel microscopy, with a color treatment intended to suppress false colors; simulated stacks and biological examples support its scope. It supplies the mathematical lineage used by focus-stack. Its appeal is multiscale detail integration without supervised training. Its original microscopy evidence does not establish behavior on reflective, noisy, high-magnification photographic stacks. The shipped focus-stack implementation adds registration, consistency filtering, denoising, and color reassignment, so paper and application must be evaluated separately. [A01, A24]

### Noise-robust selective fusion — Pertuz et al., 2013

SAF combines focus measurement, a selectivity measure, and fusion that adapts to signal strength relative to noise. It addresses a specific weakness of maximum-focus rules: they also select noise. The paper explicitly assumes frames are already aligned or parallax/magnification negligible. Synthetic and real sequences support a useful noise/detail baseline, not a complete macro registration solution. Neighborhood scale trades spatial resolution against robustness. Inference: its selective averaging should be tested where thin hairs and smooth backgrounds coexist, rather than judged only by output noise. [A02]

### Guided-filter fusion — Li, Kang, and Hu, 2013

GFF separates base/detail content and refines fusion weights with guided filtering. It is a useful interpretable, non-learned baseline for spatially coherent decisions and multiscale blending. The author publication/code page was verified; full paper retrieval from an old author-hosted PDF URL failed in this review, so detailed numerical claims are omitted. Inference: guide edges and weight smoothing do not by themselves model foreground defocus contamination, visibility, or alignment. Test radius and resolution sensitivity. [A03]

### The proposed application bases

## 3. Boundary-aware and explicit-defocus methods

### MMF-Net / alpha-matte boundary model — Ma et al., 2019

MMF-Net first estimates a guidance map and initial fusion, then adds a boundary-focused reconstruction. Its layered alpha-matte synthesis treats foreground and background defocus asymmetrically, improving on independent blur inside hard masks. This is relevant to macro halos. The inspected model still uses simplified layered Gaussian blur, and the network's stated input is a pair. Reported boundary improvements do not guarantee physical accuracy for thick 3D hairs, reflective surfaces, or long stacks. Its reconstructed boundary values require fidelity checks distinct from source-selection accuracy. [A05]

### MFF-SSIM optimization / MS-Lap and MS-ResNet — Xu et al., 2020 preprint

This work optimizes an SSIM-derived, focus-map-aware objective with gradient ascent, using either a classical or learned focus-map detector. The paper studies severe defocus spread on MFFW, including manually fused references and a ten-observer opinion test. It demonstrates that reported rankings can depend on boundary severity; a method favored under mild blur can perform poorly under severe spread. Evidence remains a small benchmark with constructed references. Inference: using its optimized objective as the only independent judge would be circular; runtime, convergence, and window sensitivity also matter. [A06]

### DMANet — Quan et al., AAAI 2025

DMANet embeds an atom-based spatially varying blur model, recurrent multiscale defocus estimation, and uncertainty-aware treatment of boundary/all-defocused pixels. This is a strong physically motivated comparator. The original indexed proceedings abstract and released two-input inference code were inspected; direct full-text access hit a bot-protection page. Consequently, this review does not repeat secondary numerical rankings or claim its supplementary multi-source behavior was reproduced. Explicit blur descriptors improve interpretability; they are not a guarantee of correct occlusion, material appearance, or long-stack scaling. [A12]

### SD-Fuse — online identifier 2025; journal issue May 2026

SD-Fuse combines global structure cues, a CNN focus-detection branch, and a structure-guided filter. Its original publisher excerpts motivate more accurate decision boundaries; released inference takes near/far directories. It is a relevant modern selection/weight-map comparator. Inference: a statistical tendency for focus boundaries to align with strong image structure is not a universal physical law. Low-contrast depth edges, crossing fine structures, and strong texture edges inside a single surface are useful stress cases. Whole-stack applicability is not established by the inspected pair interface. [A17]

### CSNet — Zhang et al., 2026 preprint

CSNet explicitly contrasts source clarity through attention, then refines ambiguous boundaries by choosing between source and reconstructed values using similarity. The original text states a typical two-source formulation. This directly addresses the selection-versus-boundary-reconstruction tradeoff. Its boundary restoration and source-preservation claims should be judged separately. Long-stack extension, full-resolution photographic precision, and specular/transparent behavior remain unverified here. The author repository links the paper but provides little reproduction guidance in its README. [A21]

## 4. Learned whole-stack methods are especially relevant

### FocusDeep — Araujo, Ponce, and Mairal, 2023 preprint

FocusDeep studies real 30-frame high-resolution raw bursts, combining features, neighboring-frame deformable alignment, and reconstruction. Noise augmentation gives a concrete route to joint denoising/fusion. This is stronger task evidence than short-pair tests. However, targets come from Helicon pyramid outputs; reference PSNR/SSIM therefore measures teacher agreement, including teacher errors. Macro coverage can be incomplete. The inspected convolutional fusion layer uses configured `nframes * nf` input channels: demonstration on 30 frames is not proof that a released checkpoint accepts arbitrary lengths. [A10]

### StackMFF — Xie et al., 2025

StackMFF models an entire stack with a 3D CNN instead of repeatedly applying a pair network. The published abstract reports attention and synthetic stacks generated using estimated depth. It is a relevant whole-stack reconstruction baseline. Its output intensities are regressed; correctness is not constrained to observed source values. Inference: synthetic depth errors and blur-model mismatch can affect transfer to true macro occlusions. The current author README says reorganized code differs subtly from the original paper implementation; pin both code and weights before claiming reproduction. [A13]

### StackMFF V2 — Xie et al., 2025

V2 predicts a continuous focal index and samples the input stack, preserving sampled values instead of regressing RGB. The publisher discussion explicitly requires an ordered focal sweep; irregular focus spacing can work if ordinal order remains valid, while violations can make results unreliable. Released color inference samples by an index map. This is an attractive fidelity-oriented whole-stack formulation, but a single index cannot directly produce an uncontaminated value absent from all sources. Its shipped example loader uses ordinary OpenCV color loading, so model formulation and high-bit-depth delivery are separate questions. [A14]

### StackMFF V3, unified release — 2026 author code; manuscript in preparation

The current author release predicts per-pixel probabilities over variable-length stacks using permutation-equivariant attention; output is a weighted observed-value combination or hard source selection. This is a promising comparator for order robustness and source support. Its claimed benchmark results were not reproduced here. The current LICENSE explicitly calls the release source-available rather than open source and restricts integration/redistribution. Earlier MIT releases are distinct artifacts requiring their own provenance audit. Treat current V3 as a research comparator, not ready code for the proposed open-source implementation. [A22]

## 5. Learned priors, self-supervision, and transformers

### CNN focus-map learning — Liu et al., 2017

This representative early learned approach maps source images to a focus map, replacing hand-designed focus features. It provides historical context for modern decision networks. Its publisher abstract supports that formulation; it does not establish hundred-frame macro behavior. Author-associated code is research-use limited. Inference: errors in the learned decision map and synthetic training assumptions remain visible as wrong-focus regions or seams. [A04]

### SwinFusion — Ma et al., 2022

SwinFusion uses within-image self-attention and cross-image attention to model long-range complementary content. It supports multiple fusion tasks; the released multi-focus interface uses two image directories and luminance processing. It is a useful general-transformer baseline, but broad multimodal applicability is not evidence of a correct macro optical model. Pairwise recurrence and photographic color reconstruction need explicit evaluation. No independent long-stack quality ranking was verified. [A08]

### SMFuse, 2021; ZMFF, 2023

SMFuse represents self-supervised mask optimization; the verified author repository exposes training/test entry points, but this review did not obtain its full primary methods text, so no detailed score or speed claim is used. ZMFF's original abstract describes deep image and deep mask priors optimized without a supervised training corpus. This avoids dependence on paired training targets, but does not remove assumptions encoded in the prior or optimization objective. Per-instance optimization budget, stopping rule, seed variance, and alignment must be specified for a fair comparison. [A07, A09]

### CCSR-Net / MCCSR-Net — Zheng, Cheng, and Liu, 2025 journal work

These methods unfold coupled convolutional sparse-representation models into learnable networks, including a multicomponent base/detail extension. They offer more explicit model structure than unconstrained end-to-end regression. The original publisher abstract and author code instructions were inspected. Their reported gains are on standard fusion datasets; long macro stacks and material fidelity remain unverified. The author code states research-only use, so evaluation availability should not be confused with permission to integrate it into an open-source product. [A11]

## 6. Recent high-resolution work solves another important axis

### PDRFuse — Zheng, Zhou, and Xiao, Information Fusion, November 2026 issue

PDRFuse progressively refines coarse decisions across image scales, combining re-parameterization and a state-space module. Author code and publisher-indexed excerpts establish a high-resolution **pair** method, not arbitrary-stack inference. It is relevant to train/test resolution mismatch and includes an HRMFF test-data lead. The repository lists CUDA/selective-scan requirements. Its code was public by May 2026; the paper is discoverable by the cutoff. Full publisher methods were inaccessible here, so no reproduced runtime or quality claim is made. [A18]

### UMF-LUT / UHD-MFF — Zhang et al., 2026

UMF-LUT splits coarse region decisions from high-resolution edge decisions using learned lookup tables. Original text describes 4K near/far pairs and low computational overhead; the author repository supplies checkpoints and synthetic/real data links. This is a valuable scale-efficiency comparator. The paper's conventional information/gradient metrics do not by themselves certify macro boundary correctness. Real-time pair throughput cannot be extrapolated to a full registered 100-frame photographic stack without measuring the complete workload. [A20]

### VAEEDOF — Piano, Huanwen, and Bilcu, 2025 preprint

VAEEDOF uses a pretrained/distilled VAE and a fusion U-Net for up to seven images, with high-resolution synthetic MattingMFIF data. The original text describes Blender scenes made from photographic cutouts, not general exact 3D macro truth. It is relevant to high-resolution reconstruction and controlled boundary synthesis. Seven simultaneous images are still far short of many macro stacks; tiling, multi-pass fusion, pretrained-model licensing, and factual detail need separate audits. The README's model link was found; weights were not downloaded. [A15]

## 7. Diffusion needs separate fidelity scrutiny

### ReDiffuse — Li et al., 2026 preprint

ReDiffuse incorporates approximate rotation equivariance into a diffusion fusion network to reduce directional geometric artifacts. Original full extracted text reports training on Real-MFF pairs and evaluation on four conventional benchmarks. Author code and weights are public. This supports a modern diffusion comparator and a useful geometric consistency hypothesis; it does not establish true long-macro detail recovery. Equivariance is an architectural property, not a proof of image correctness. Report seed/sampling policy and inspect false fine structures independently of perceptual quality. [A19]

### GMFF — Xie et al., 2025 preprint

GMFF separates deterministic stack fusion from latent-diffusion restoration, explicitly targeting missing focal-plane content and residual artifacts. It is a clear example of perceptual restoration extending beyond source selection. Its deterministic component has since been reorganized in the StackMFF series; current and historical versions must not be conflated. Inference: plausible reconstruction in a region without sufficient measurements cannot certify that exact hairs, pores, or codes were recovered. Preserve deterministic output separately and evaluate generated content as a distinct output mode. [A16, A22]

## 8. Commercial software supplies practical references, not ground truth

Zerene documents PMax/DMap as complementary: pyramid detail preservation comes with noise/contrast/color risks; depth-map output can preserve smoother color while losing difficult overlap detail. Retouching combines them. Helicon Focus 9 documents weighted, depth-map, and pyramid choices, radius/smoothing tradeoffs, and moving-subject retouching. These are relevant practical baselines when software access is available. Their undocumented implementation details must not be invented. Automatic outputs and manual retouches are different effort budgets. A commercial result used as training target remains a pseudo-reference. [A25–A26]

## 9. Cross-cutting implications for the next phase

**Registration:** supply a validated registered-input track alongside end-to-end capture processing. Otherwise a fusion improvement may merely reflect a better warp. Save transforms and validity masks; do not hide geometry errors with unrestricted output-to-reference registration.

**Radiometry:** source sampling, high-bit-depth arrays, linear-light processing, HDR headroom, and color management are different properties. Pin raw conversion and exposure normalization. Report tone mapping and sharpening as part of the tested method, with controlled ablations if possible.

**Order and stack length:** preserve native ordered input for methods that require it. Use reversal, valid frame thinning, and duplicate-frame tests to study sensitivity. Arbitrary permutation is a robustness test only for methods claiming order independence. Recursive pair fusion needs a declared reduction tree and must include accumulated-error tests.

**Scale and runtime:** measure the same image resolution, number of frames, registration setting, device, memory budget, and input/output format. Distinguish fusion-only time from complete pipeline time. A small model can still have stack- or attention-dependent memory growth; a fast 512-pixel pair score says little about a 24-megapixel 150-frame job.

**Fidelity and artifact evidence:** source-copy constraints do not ensure correct visibility. Blur priors do not ensure correct optics. No-reference training does not create a no-assumption method. Reference PSNR against a software result does not establish absolute fidelity. Factual correctness, perceptual preference, and capture completeness need distinct evidence.

**Reproducibility:** record code revision, checkpoint hash, preprocessing, parameter budget, random seed, reduction strategy, masks, output decode, and known access/license limits. No source inspected here proves an exact solution across all macro materials and capture conditions.

## 10. Recommended comparator set

This is a proposed experiment order. It is **not** a measured performance ranking or permission to redistribute external implementations. [handoff.md](handoff.md) records the concrete constraints for measurement and dataset reviews.
