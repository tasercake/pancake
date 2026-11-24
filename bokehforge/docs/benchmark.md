# Focus-stacking benchmark

Run the mean-blend pilot on the InfinigenDefocus dataset from the repository root. Use a new output directory for each run.

```sh
uv run --project bokehforge --frozen bokehforge data download --dataset D29
uv run --project bokehforge --frozen bokehforge bench \
  --algorithm mean --dataset D29 --output artifacts/bench-mean --save-images --progress
```

Repeat `--dataset` or `--scene` to select a fixed cohort. The default scans downloaded datasets and scores those with supported all-in-focus RGB targets. `--root`, `--config`, and `--lock` have the same meanings as in the catalog. In the report, compare the cohort fingerprint, score version, and calibration checksum before comparing runs. Candidate images depend on the stacker environment, so pin the stacker and its dependencies too.

## Stacker interface

Pass `--algorithm package.module:callable` with the module installed in the BokehForge environment or on `PYTHONPATH`. The callable runs once per eligible scene and returns a NumPy array containing finite, full-resolution, float32 linear-sRGB samples in H×W×3 order on the declared target grid. Alignment and output framing are the stacker's responsibility. `StackInput` provides the scene ID, ordered focal frame IDs, target shape, and a single-pass frame iterator. A frame includes canonical RGB pixels, optional source alpha, and available capture metadata. Read focal frames during the call and copy arrays before modifying them.

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

Standard output contains one cohort score from 0 to 100. A perfect delivery scores 100. Higher scores indicate closer agreement with ground truth across the tested views and artifact families. The default curve is provisional pending calibration and qualification against blinded photographer ratings. Compare scores within the same profile and cohort.

The scorer uses the delivered image and the RGB target. It compares them in the target coordinates, without fitting an alignment, exposure, or color correction to the output. It makes three identically rendered comparisons.

| View | Default weight | Comparison |
| --- | ---: | --- |
| Fit | 30% | SDR comparison after antialiasing and downsampling to at most 640 pixels on the longest side. |
| Native | 50% | Whole frame at 1:1, including borders. Range checks retain the original linear samples. |
| Zoom | 20% | Up to five 96×96-pixel native crops at the center and corners, displayed at 2× using bilinear interpolation. |

The fit and crop renderings limit raw samples to ±8 before resampling to keep interpolation stable for finite extreme inputs. All native linear values participate in the range check. Boundary and structure detectors measure observable RGB discrepancies. They can flag a suspect region for inspection. Assigning causes, checking exact filament topology, and comparing depth or materials require additional annotations or human review.

| Family | Default weight | Measures |
| --- | ---: | --- |
| Tone | 17% | Multiscale CIELAB lightness residuals from exposure and contrast shifts, uneven lighting, or seams. |
| Color | 16% | CIELAB opponent-plane distance from casts, saturation shifts, fringing, or localized color errors. |
| Lost detail | 18% | Reduced or reversed directional contrast in four directions at three scales, including two-pixel-period texture. |
| Added structure | 15% | Excess or reversed directional contrast and high-frequency residuals from noise, sharpening, ghosting, banding, blocking, or invented texture. |
| Boundary | 23% | Edge-neighborhood residuals and overshoot from displacement, softness, halos, ringing, or duplicate contours. |
| Range | 11% | Changed out-of-SDR samples and differences flattened toward an SDR channel endpoint. |

Within a view, each family combines mean damage (40%), a tail of the worst 1% of nonoverlapping tiles (35%), and the strongest supported tile (25%). The tail and peak use 8- and 32-pixel windows and take the worst value over four tile phases. This gives localized defects leverage while requiring pixel support. The zoom view averages mean and worst crop burden per family. The default scene score is `100 × exp(−5 × weighted burden)`, with bounded family damages and the weights above. Each dataset gets equal weight, and each independent catalog ancestry group gets equal weight within its dataset. Scenes within a group are averaged first.

An eligible stacker failure or a missing or invalid delivery scores zero. An eligible reference, input decode, or scoring error withholds the aggregate because the evaluation failed. A scene without a supported reference is listed as unscored and excluded from the aggregate. The command prints `unscored` if the aggregate is unavailable. A nonzero exit reports a runtime failure even when some scene scores were computed.

`report.json` contains the cohort fingerprint, runtime versions, eligibility and exit details, score features, and diagnostics. For each view and family it gives affected fraction and the worst tile in both view and native coordinates. Boxes use half-open `[left, top, right, bottom]` pixel coordinates. RGB RMSE and signed lightness, Lab chroma, and exposure offsets are diagnostic.

## Calibrate against human judgments

`bokehforge.calibration.fit_calibration(feature_rows, grades, groups)` fits a regularized ordinal model using the report's `score_features`, blinded integer grades from 0 through 4, and independent ancestry group IDs. Give graders the reference, full image, and fixed crops. Use 0 for unusable, 1 for restack or major repair, 2 for substantial retouching, 3 for minor cleanup, and 4 for publish-ready. Reserve different scene and stacker families for qualification. Save the fitted model with `json.dumps(model.as_dict())` and pass it as `--score-calibration model.json` in both runs. The model learns separate nonnegative weights for each family and view, scores the sum of the four cumulative grade probabilities, and normalizes a perfect delivery to 100. BokehForge records the model file's SHA-256.
