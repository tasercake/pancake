# E9 reference eligibility and correction plan

## Verified source and retained outputs

- Pancake's `AdvancedAccumulator::finish` and `finish_exposure_batch` use the shared
  CubeCL `accumulate_batch` and `average_batches` kernels. The same tree
  applies to CPU, Vulkan and CUDA. The inspected historical source is
  `12f2151c8a198b5209c8f4d506dbd17fd9aaa74f`.
  [Exposure tree](../../../pancake/src/fusion/advanced/mod.rs) ·
  [Equal accumulation and division](../../../pancake/src/fusion/advanced/levels.rs)

The retained exports were genuine 337 × 510 RGB16 in all lanes. Full-image
Pancake/reference MAE was 2,823.78 u16 codes on CPU and about 2,825.18 on the
GPU comparisons. The matching-geometry exports differed in all three
reviewed regions. Attributing the error requires the intermediate partials
and a controlled comparison. The recorded timings and telemetry remain
historical diagnostics under the documented algorithm mismatch, including
the original numeric misses. [CPU region review](integrated-e9-12f2151/cpu-review.json) ·
[Vulkan region review](integrated-e9-12f2151/vulkan-review.json) ·
[CUDA region review](integrated-e9-cuda-12f2151/cuda-review.json)

## Separate color convention

## Test-first launcher plan

Use a scoped E9 reference launcher under the allowed Pancake adapter, selected
only by the qualification runner for the E9 exposure-fusion profile. Invoke
the pinned `src.cli` in the measured reference process with the same ordered
native inputs, alignment, crop, writer, backend selection, caches and exit
contract. Record the launcher hash alongside the untouched reference commit,
descriptor and runtime lock. Keep the correction within the E9 launcher
and the qualification runner's dispatch.

1. **CPU.** Substitute a scoped `_exposure_core` implementation with the
   same batching, pause, cancellation, progress and singleton behavior. Call
   the actual pinned `CPU.mertens_fuse_batch` on each multisample group. Add
   each reconstructed image once and divide by the number of batches in
   float32. A length-based interception of `mertens_fuse_batch` could confuse
   an incomplete source batch with the final merge. Keep the interception at
   this API boundary.
2. **OpenCL.** Scope the substitution to `ExposureGPU.result`. Keep `add`,
   `_flush`, `merge`, singleton passthrough, and the reference upload path.
   Sum the partials on the existing OpenCL queue using supported device-array
   arithmetic and divide by their count there. The normal result path then
   crops and exports the same type of device result. This keeps the equal
   combine in the explicitly selected GPU path.
3. **Meaningful failing witnesses before code.** Hold three distinct,
   already reconstructed partials fixed. Values 20, 80 and 200 must give
   100 per channel under equal batch weights. Counting original exposures
   again would give 66⅔. For a nonconstant witness, use a red/green
   checkerboard, a constant blue image and a constant black image as the
   three partials. With RGB values in 0–255, their mean is `(85, 0, 85)`
   at red pixels and `(0, 85, 85)` at green pixels. Under the stock BGR
   contrast convention, the checkerboard has unit-scale four-neighbor
   Laplacian magnitude `4 × (0.587 − 0.114) = 1.892` before saturation
   weighting. The two constant partials have zero contrast and only the
   epsilon weight, so a second Mertens merge favors the checkerboard.
   Check generic incomplete groups, the singleton-only path, input order,
   unchanged calls into the pinned batch functions, and physical OpenCL
   dispatch when a later device check is authorized.

For the independent real-input check, capture the registered full working
canvas and reconstructed pre-export partials from the exact nine originals.
Compute the expected per-channel mean independently in a wider accumulator
and compare both launcher paths before applying the common output crop and
RGB16 quantization. Use the same crop afterward for region and whole-image
comparison. To attribute the color contribution, hold registration, source
bytes, batch order, border/pyramid operations and final combiner fixed while
varying only luma storage semantics in an independent diagnostic. Derive
the expected contrast response from separate red and blue edge controls.
Record intermediate arrays or hashes and the original pinned-output control.
Only the unchanged frozen quality checks govern a later qualification.

This is a source and receipt audit with a test plan. The launcher correction,
independent recomputation and qualification have not run.
