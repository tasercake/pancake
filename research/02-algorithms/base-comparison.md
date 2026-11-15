# Candidate-base audit

Read-only inspection on **2026-11-15**. No changes to the upstream project, build, algorithm run, quality comparison, or precision round-trip test were performed. Findings describe the pinned paths inspected, not every possible application/API configuration.

| Item | focus-stack |
|---|---|
| Revision | `0cd289e224fe24aead5b146faeaa46950775f346` |
| Main implementation | C++/OpenCV; OpenCL wavelets |
| Inspected license | MIT, full file preserved in `../sources/` |
| Fusion choices | Complex-wavelet merge, consistency/denoising, color reassignment |
| Alignment | Neighboring affine ECC by default; global-reference option; photometric matching |
| Photographic sample precision | Inspected file-loading and color-reassignment path is explicitly 8-bit |
| Stack-length strategy | Task graph, batched wavelet merge and compact color map |
| Strong research advantage | Distinct, interpretable wavelet baseline with a compact compiled pipeline |
| Main quality concerns to test | Precision floor; halo/color reassignment behavior; grayscale focus representation; batch/consistency sensitivity |

## focus-stack: verified implementation facts

**Actual sample precision.** [Task_LoadImg](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_loadimg.cc#L53) uses `cv::IMREAD_ANYCOLOR`, without `IMREAD_ANYDEPTH`. More decisively, [color-map construction](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_reassign.cc#L49) requires `CV_8U` gray and `CV_8UC3` color, and [reassignment output](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_reassign.cc#L186) allocates `CV_8UC3`. Supporting a TIFF filename does not establish preservation of 16-bit sample precision. Float wavelet coefficients and a 16-bit depth/index map do not change that color-output contract.

**Alignment and photometry.** [Task_Align](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_align.cc#L271) uses affine ECC; [resampling](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_align.cc#L405) uses cubic interpolation and reflected borders. The [algorithm documentation](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/docs/Algorithms.md) describes default neighboring alignment and exposure/white-balance matching. Thus “source value” in later reassignment means an aligned, potentially photometrically changed source sample, not an unchanged camera measurement.

**Merge and color.** [Task_Merge](https://github.com/PetteriAimonen/focus-stack/blob/0cd289e224fe24aead5b146faeaa46950775f346/src/task_merge.cc#L20) selects maximum-magnitude complex coefficients, then applies subband and neighbor consistency at enabled levels. Its persistent state is merged between batches. The documentation explicitly acknowledges halo trouble and that color reassignment can make halos more conspicuous when a good color match does not exist. That is direct upstream evidence of a failure mode, not a new measured defect rate. Its depth-map visualization branch is separate from the main fused-color reconstruction.

**Memory and order.** Batched processing and compact grayscale-to-color entries avoid retaining all full-color frames indefinitely. The README gives memory estimates, but these were not reproduced. Consistency filters, ties, batch boundaries, and denoising make order/batch invariance something to test, not assume from the raw maximum operation. Global PCA grayscale conversion and later color reassignment motivate isoluminant-color test patterns; this is an inference requiring experiments.

## Proposed qualification

Before a quality-based base decision, qualify five bounded items:

1. Fix external RAW conversion and color/range contract, then compare shipped native input as a separate end-to-end track.
2. Preserve pre-tone/pre-sharpen output and final delivered output as separate measured stages where interfaces allow it.
3. Test supplied registration and native alignment with coverage masks.
4. Test order, frame count, noise, slab/batch size, CPU/GPU path, and parameter sensitivity on the same independent scenes.
5. Verify high-bit-depth ramps, dark gradients, highlights, and export round trips before accepting a 16-bit label.

These are proposed qualification checks. No implementation changes or experiment results are claimed. Source snapshots record exactly what was read; upstream links and immutable revisions remain the owning evidence.
