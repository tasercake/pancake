# Focus-stacking benchmark

Run the mean-blend pilot on the InfinigenDefocus dataset from the repository root. Use a new output directory for each run.

```sh
uv run --project bokehforge --frozen bokehforge data download
uv run --project bokehforge --frozen bokehforge bench \
  --algorithm mean --output artifacts/bench-mean --save-images --progress
```

Repeat `--dataset` or `--scene` to select a fixed cohort. By default the benchmark scores configured downloads with supported all-in-focus RGB targets. The shipped configuration selects D29. `--root`, `--config`, and `--lock` have the same meanings as in the catalog. Check that the cohort fingerprint and score version match before comparing runs. Candidate images depend on the stacker environment, so pin the stacker and its dependencies too.

## Stacker interface

Pass `--algorithm package.module:callable` with the module installed in the BokehForge environment or on `PYTHONPATH`. The callable runs once per eligible scene and returns one NumPy array containing finite, full-resolution, float32 linear-sRGB samples in H×W×3 order. The default profile requires the declared target grid. The crop-tolerant profile accepts a smaller image at the original pixel scale and infers its placement from its pixels. `StackInput` provides the scene ID, ordered focal frame IDs, target shape, and a single-pass frame iterator. A frame includes canonical RGB pixels, optional source alpha, and available capture metadata. Read focal frames during the call and copy arrays before modifying them.

```python
from bokehforge.stacking import StackInput
import numpy as np

def first_frame(stack: StackInput) -> np.ndarray:
    return next(stack.frames).pixels
```

For external tools, generate one canonical interleaved RGB float32 TIFF per scene using the path `<candidate-dir>/<scene-id>.tif`. Use orientation 1 and convert the original colors and transfer to linear sRGB before writing. A TIFF from `--save-images` already meets the contract. The candidate-directory path evaluates delivered files without decoding the focal frames.

```sh
uv run --project bokehforge --frozen bokehforge bench \
  --candidate-dir artifacts/bench-mean/images --dataset D29 --output artifacts/bench-replay
```

## The score

Standard output contains one cohort score from 0 to 100. A perfect delivery scores 100. Higher scores indicate closer agreement with ground truth across the tested artifact families. The default profile is `reference-fidelity-v2`.

The default scorer compares the whole delivered image with the RGB target at original resolution, including borders. It uses the target coordinates and the delivered exposure and colors. Range checks retain every original linear sample. Boundary and structure detectors measure observable RGB discrepancies. They can flag a suspect region for inspection. Assigning causes, checking exact filament topology, and comparing depth or materials require additional annotations or human review.

| Family | Weight | Measures |
| --- | ---: | --- |
| Tone | 17% | Multiscale CIELAB lightness residuals from exposure and contrast shifts, uneven lighting, or seams. |
| Color | 16% | CIELAB opponent-plane distance from casts, saturation shifts, fringing, or localized color errors. |
| Lost detail | 18% | Reduced or reversed directional contrast in four directions at three scales, including two-pixel-period texture. |
| Added structure | 15% | Excess or reversed directional contrast and high-frequency residuals from noise, sharpening, ghosting, banding, blocking, or invented texture. |
| Boundary | 23% | Edge-neighborhood residuals and overshoot from displacement, softness, halos, ringing, or duplicate contours. |
| Range | 11% | Changed out-of-SDR samples and differences flattened toward an SDR channel endpoint. |

Each family combines mean damage (40%), the mean of the worst 1% of tiles (35%), and the strongest supported tile (25%). The local checks use 8- and 32-pixel squares and take the worst results across four grid offsets for each size. This gives localized defects leverage while requiring pixel support. The scene score is `100 × exp(−5 × weighted burden)`. The weighted burden sums the six measurements multiplied by their family weights. Each dataset gets equal weight, and each independent catalog ancestry group gets equal weight within its dataset. Scenes within a group are averaged first.

An eligible stacker failure or a missing or invalid delivery scores zero. An eligible reference, input decode, or scoring error withholds the aggregate because the evaluation failed. A scene without a supported reference is listed as unscored and excluded from the aggregate. The command prints `unscored` if the aggregate is unavailable. A nonzero exit reports a runtime failure even when some scene scores were computed.

`report.json` uses schema version 3 and records the family weights and the curve's `damage_scale` under `scoring`. It also contains the cohort fingerprint, runtime versions, eligibility and exit details, six `native.*` score features, and diagnostics. For each family it gives the affected fraction and the worst tile on the reference grid. Boxes use half-open `[left, top, right, bottom]` pixel coordinates. RGB RMSE and signed lightness, Lab chroma, and exposure offsets are diagnostic.

## Score explanations

Add `--explain-score summary` to a benchmark run to save a versioned explanation for each measured scene. Use `--explain-score tiles` when you need exact local values for overlays. Both options give the same score on standard output. The report links each explanation through `score_explanation_path` and records the dataset and group averages, weights, and scene counts under `score_aggregation`.

In Python, `measure(candidate, target, details="summary")` gives the scalar as `.score` and the comparison as `.explanation`. For an existing canonical TIFF, use `score_saved_output(catalog, scene, path, details="summary")` from `bokehforge.bench` to get the score, status, and full scene explanation. Both calls also accept `details="tiles"`. Pass `profile="reference-fidelity-crop-v2"` to score a saved crop. With no detail level, the calls skip building the explanation.

- **What drove the score.** Each placement gives its overlap score, coverage, match weight, and contribution to the scene score in points. Its `comparison` gives the total weighted damage and a stable ID, label, description, burden, weight, and weighted damage for each family. The `pool` entries give the mean, worst-1% average, and worst tile with their values, weights, and contributions to the family burden. Damage and burden are on a 0–1 scale, where higher means more difference. The exponential score curve turns their weighted sum into points.
- **Where the scorer saw damage.** Worst-tile boxes use zero-based, half-open `[left, top, right, bottom]` coordinates on the native reference grid. A family with zero local damage has no worst box. The `tile_grids` object in the `tiles` payload contains all eight grids. Consecutive `x_edges` and `y_edges` define each box. Values, coverage counts, and selected tile indices follow row-major order, left to right and then top to bottom. Each value is summed pixel damage divided by `nominal_pixels`. Tiles with no measured pixels have coverage zero and are excluded from the worst-1% selection. When `covered_pixels` is null, every tile is covered throughout its actual bounds. Grids overlap, so show a single grid at a time rather than adding tiles from several offsets.
- **How to place an overlay.** Each full explanation identifies the scene, reference image, profile, and native sizes. It uses the same layout for fixed and crop scoring. The two-row `candidate_to_reference_xy` matrix maps the saved candidate's `[x, y, 1]` pixel centers into the reference, with the upper-left pixel center at `[0, 0]`. To draw a reference box on a cropped delivery, subtract 0.5 from each edge coordinate, apply the inverse placement, then add 0.5 to convert back to edges. Fixed-grid scoring uses the identity matrix. Crop mode supplies one record per evaluated placement and its measured coverage. An unmatched crop records the reason and an empty placement list. An invalid delivery or unavailable reference retains its status and has no measured damage to display.

In a review viewer, bind the explanation to the run, scene, stacker and revision of the image currently visible, including during thumbnail previews. Fetch the tile payload when a user opens an overlay. Use the actual painted image bounds so the overlay follows the image through zoom, pan and reference swipes. The score detects differences from the target. A highlighted patch is evidence for inspection.

## Crop-tolerant scoring

Add `--score-profile reference-fidelity-crop-v2` to either an algorithm run or a replay of saved images. The stacker delivers one image. Crop coordinates and other alignment metadata come from the scorer's own image comparison.

This profile searches for shifts and up to three degrees of rigid rotation using brightness patterns spread across the image. The unrotated crop rectangle must fit inside the reference, so full-size images have a fixed center and trimming one pixel only unlocks one pixel of translation. At full size, the scorer applies rotation only when the image evidence clearly improves over the fixed placement. Enlargement, shrinkage, stretching and local warps are outside the fit.

The scorer samples the delivered pixels on the original-resolution reference grid and measures quality where the images overlap. The reported coverage is the fraction of reference pixel positions covered by real delivered pixels, capped at the ratio of delivered to reference pixels. A perfect crop covering 95% scores 95 when its placement is clear.

When several placements have similar evidence, the score averages their overlap-quality scores multiplied by their coverage, weighted by image agreement. Weak evidence reduces the available weight smoothly toward a zero-score no-match outcome. These weights are heuristics based on image agreement. Crop-profile reports record the inferred offset, rotation, coverage, overlap score and weight for each evaluated placement. The offset is the unrotated crop origin in row, column order, and rotation is around the crop center. The feature readings describe the leading placement. An unmatched delivery receives zero and is reported under the scene's `alignment` field. The saved image, if requested, is the stacker's original delivery.

Compare results only within the same score profile. Rotation requires interpolation and can affect measured fine detail. The three-degree limit and agreement thresholds are starting points for this profile.
