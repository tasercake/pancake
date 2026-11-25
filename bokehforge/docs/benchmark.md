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

Standard output contains one cohort score from 0 to 100. A perfect delivery scores 100. Higher scores indicate closer agreement with ground truth across the tested views and artifact families. The default profile is `reference-fidelity-v1`.

The scorer uses the delivered image and the RGB target. It compares them in the target coordinates, without fitting an alignment, exposure, or color correction to the output. It makes three identically rendered comparisons.

| View | Weight | Comparison |
| --- | ---: | --- |
| Fit | 30% | SDR comparison after antialiasing and downsampling to at most 640 pixels on the longest side. |
| Native | 50% | Whole frame at 1:1, including borders. Range checks retain the original linear samples. |
| Zoom | 20% | Up to five 96×96-pixel native crops at the center and corners, displayed at 2× using bilinear interpolation. |

The fit and crop renderings limit raw samples to ±8 before resampling to keep interpolation stable for finite extreme inputs. All native linear values participate in the range check. Boundary and structure detectors measure observable RGB discrepancies. They can flag a suspect region for inspection. Assigning causes, checking exact filament topology, and comparing depth or materials require additional annotations or human review.

| Family | Weight | Measures |
| --- | ---: | --- |
| Tone | 17% | Multiscale CIELAB lightness residuals from exposure and contrast shifts, uneven lighting, or seams. |
| Color | 16% | CIELAB opponent-plane distance from casts, saturation shifts, fringing, or localized color errors. |
| Lost detail | 18% | Reduced or reversed directional contrast in four directions at three scales, including two-pixel-period texture. |
| Added structure | 15% | Excess or reversed directional contrast and high-frequency residuals from noise, sharpening, ghosting, banding, blocking, or invented texture. |
| Boundary | 23% | Edge-neighborhood residuals and overshoot from displacement, softness, halos, ringing, or duplicate contours. |
| Range | 11% | Changed out-of-SDR samples and differences flattened toward an SDR channel endpoint. |

Within a view, each family combines mean damage (40%), a tail of the worst 1% of nonoverlapping tiles (35%), and the strongest supported tile (25%). The tail and peak use 8- and 32-pixel windows and take the worst value over four tile phases. This gives localized defects leverage while requiring pixel support. The zoom view averages mean and worst crop burden per family. The scene score is `100 × exp(−5 × weighted burden)`. The weighted burden sums each measurement multiplied by its view weight and family weight. Each dataset gets equal weight, and each independent catalog ancestry group gets equal weight within its dataset. Scenes within a group are averaged first.

An eligible stacker failure or a missing or invalid delivery scores zero. An eligible reference, input decode, or scoring error withholds the aggregate because the evaluation failed. A scene without a supported reference is listed as unscored and excluded from the aggregate. The command prints `unscored` if the aggregate is unavailable. A nonzero exit reports a runtime failure even when some scene scores were computed.

`report.json` uses schema version 2 and records the fixed weights and the curve's `damage_scale` under `scoring`. It also contains the cohort fingerprint, runtime versions, eligibility and exit details, score features, and diagnostics. For each view and family it gives affected fraction and the worst tile in both view and native coordinates. Boxes use half-open `[left, top, right, bottom]` pixel coordinates. RGB RMSE and signed lightness, Lab chroma, and exposure offsets are diagnostic.

## Crop-tolerant scoring

Add `--score-profile reference-fidelity-crop-v1` to either an algorithm run or a replay of saved images. The stacker still delivers one image. Crop coordinates and other alignment metadata come from the scorer's own image comparison.

This profile searches for shifts and up to three degrees of rigid rotation using brightness patterns spread across the image. The unrotated crop rectangle must fit inside the reference, so full-size images have a fixed center and trimming one pixel only unlocks one pixel of translation. At full size, the scorer applies rotation only when the image evidence clearly improves over the fixed placement. Enlargement, shrinkage, stretching and local warps are outside the fit.

The scorer samples the delivered pixels on the reference grid and measures quality where the images overlap. The fit view retains the reference viewing scale, and the close-up windows stay at the reference corners and center. A partly covered close-up carries less influence, and a window with no overlap contributes no readings. The reported coverage is the fraction of reference pixel positions covered by real delivered pixels, capped at the ratio of delivered to reference pixels. A perfect crop covering 95% scores 95 when its placement is clear.

When several placements have similar evidence, the score averages their overlap-quality scores multiplied by their coverage, weighted by image agreement. Weak evidence reduces the available weight smoothly toward a zero-score no-match outcome. These weights are heuristics based on image agreement. Crop-profile reports record the inferred offset, rotation, coverage, overlap score and weight for each evaluated placement. The offset is the unrotated crop origin in row, column order, and rotation is around the crop center. The feature readings describe the leading placement. An unmatched delivery receives zero and is reported under the scene's `alignment` field. The saved image, if requested, is the stacker's original delivery.

Compare results only within the same score profile. Rotation requires interpolation and can affect measured fine detail. The three-degree limit and agreement thresholds are starting points for this profile.
