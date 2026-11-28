# Focus-stacking benchmark

Run the mean-blend pilot on InfinigenDefocus from the repository root. Use a new output directory for each run. The Inspection extra enables the default attention-weighted score. With the base installation, add `--no-attention`. [Inspection setup](#install-the-inspection-models)

```sh
uv run --project bokehforge --frozen bokehforge data download
uv run --project bokehforge --frozen --extra inspection bokehforge bench --algorithm mean --output artifacts/bench-mean --save-images --progress
```

Repeat `--dataset` or `--scene` to select a fixed cohort. By default the benchmark selects all scenes from the configured downloads. Every selected scene must declare exactly one supported all-in-focus RGB reference. If any scene fails this requirement, the command rejects the entire selection before it runs a stacker or creates a report. The shipped configuration selects D29. `--root`, `--config`, and `--lock` have the same meanings as in the catalog. Check that the cohort fingerprint and score version match before comparing runs. Candidate images depend on the stacker environment, so pin the stacker and its dependencies too.

## Stacker interface

Pass `--algorithm package.module:callable` with the module installed in the BokehForge environment or on `PYTHONPATH`. The callable runs once per scene with a validated reference and returns one NumPy array containing finite float32 linear-sRGB samples in H×W×3 order. Deliver as much of the original field at native resolution as the stacker can preserve. The scorer infers the delivery’s placement from the RGB pixels. `StackInput` provides the scene ID, ordered focal frame IDs, target shape, and a single-pass frame iterator. A frame includes canonical RGB pixels, optional source alpha, and available capture metadata. Read focal frames during the call and copy arrays before modifying them.

```python
from bokehforge.stacking import StackInput
import numpy as np

def first_frame(stack: StackInput) -> np.ndarray:
    return next(stack.frames).pixels
```

For external tools, generate one canonical interleaved RGB float32 TIFF per scene using the path `<candidate-dir>/<scene-id>.tif`. Use orientation 1 and convert the original colors and transfer to linear sRGB before writing. A TIFF from `--save-images` already meets the contract. Replay decodes the delivered TIFFs and references directly. The native TIFF reader handles canonical deliveries and uses the existing Python TIFF adapter for compatible encodings that need it. Dataset and focal-frame decoding use the existing format adapters.

```sh
uv run --project bokehforge --frozen --extra inspection bokehforge bench --candidate-dir artifacts/bench-mean/images --dataset D29 --output artifacts/bench-replay
```

On Linux, saved deliveries with generated Inspection maps can score two scenes
at a time when their sizes and available host and CUDA memory allow it. Report
rows and output images follow catalog order. The report's total `seconds`
measures elapsed time, and per-scene `scoring_seconds` can overlap. Stacker runs
and full explanation exports process one scene at a time.

## Call the scorer from Python

`measure` takes two NumPy arrays in candidate, reference order. Each array has shape (height, width, 3) and contains 32-bit floating-point linear-sRGB samples. The return value provides the scene score, per-family features, diagnostics, alignment, and any requested explanation.

```python
from bokehforge import measure
from bokehforge.bench import bench

result = measure(candidate, reference, attention=False, device="cpu", details="summary")
print(result.score, result.features)

exit_code = bench(["--candidate-dir", "deliveries", "--output", "results", "--no-attention"])
```

`bench(argv=None)` takes the same argument list as the CLI following `bokehforge bench`. It writes the same report and returns the command's exit code. Existing imports from `bokehforge.scoring.measure` and `bokehforge.bench.main` continue to work.

## Scoring device

`--device auto` selects CUDA, then Apple Metal, then CPU. The Rust scorer runs a shared CubeCL damage program across all three backends. Auto can retry damage measurement on CPU if the accelerator runs out of memory.

`--device cuda` selects NVIDIA CUDA on Linux or Windows. CubeCL compiles the damage kernels with its bundled LLVM and runs them through the NVIDIA driver. `--device mps` selects Metal 3 on Apple Silicon. `--device gpu` requires an available GPU, and `--device cpu` selects CPU scoring. Device selection is independent of the stacker.

Native Inspection uses the same exported model graphs through ONNX Runtime’s CPU, CUDA, or Core ML provider. With `auto`, the engine can use CPU inference if the accelerator provider is unavailable or its sessions fail to load. On Apple Silicon, the `mps` selector pairs Metal damage measurement with Core ML inference. CUDA inference uses CUDA 12 and cuDNN 9 libraries in addition to the NVIDIA driver. Use `--no-attention` to skip Inspection entirely.

## The score

Standard output contains one cohort score from 0 to 100. The default score version is `reference-fidelity-v3-inspection-v2`. With `--no-attention` the score version is `reference-fidelity-v3`. A pixel-identical full view scores 100. An exact native-scale crop covering 81% of the reference scores 81, wherever it lies. Enlarging that crop back to the original file dimensions still leaves 19% of the reference missing. A half-size full view can cover the whole reference, but its delivery samples cap its score at 25 before any quality loss.

Each plausible placement contributes **match weight × effective coverage × overlap quality**. Effective coverage is the smaller of the reference area reached by the delivery and the fraction supported by contributing delivery samples. Area comes from the intersection of the transformed delivery footprint and the reference, including fractional edges. A square delivery sample covers `scale²` reference-pixel areas, so dividing the intersection area by `scale²` estimates the contributing sample count. Pixels beyond the reference earn no credit. Match weights sum to the confidence in correspondence. When multiple distinct placements have similar support, the score combines them using their image agreement.

The scorer compares the overlap at the reference’s original resolution, preserving the delivery’s exposure, colors and range. Native integer placements read the delivered samples directly. Fractional placement uses bilinear sampling, which can affect measured fine detail. RGB differences can point to a suspect region for inspection. Assigning causes and checking topology, depth or materials require additional annotations or human review.

| Family | Weight | Measures |
| --- | ---: | --- |
| Tone | 17% | Multiscale CIELAB lightness residuals from exposure and contrast shifts, uneven lighting, or seams. |
| Color | 16% | CIELAB opponent-plane distance from casts, saturation shifts, fringing, or localized color errors. |
| Lost detail | 18% | Reduced or reversed directional contrast in four directions at three scales, including two-pixel-period texture. |
| Added structure | 15% | Excess or reversed directional contrast and high-frequency residuals from noise, sharpening, ghosting, banding, blocking, or invented texture. |
| Boundary | 23% | Edge-neighborhood residuals and overshoot from displacement, softness, halos, ringing, or duplicate contours. |
| Range | 11% | Changed out-of-SDR samples and differences flattened toward an SDR channel endpoint. |

Each family combines mean damage (40%), the mean of the worst 1% of tiles (35%), and the strongest supported tile (25%). The local checks use 8- and 32-pixel squares and take the worst results across four grid offsets for each size. This gives localized defects leverage while requiring pixel support. Overlap quality is `100 × exp(−5 × weighted burden)`. The weighted burden sums the six measurements multiplied by their family weights. Each dataset gets equal weight, and each independent catalog ancestry group gets equal weight within its dataset. Scenes within a group are averaged first.

The scorer requires two images, the stacker's delivery and the ground-truth reference. Once the reference has been validated, a stacker failure or a missing or invalid delivery scores zero. A failure to decode or validate a declared reference, decode a focal input, or compute the score withholds the aggregate. The command prints `unscored` for these evaluation failures and exits with a nonzero status. An RGB delivery with weak match evidence can score zero with a successful evaluation status. A missing delivery can yield a numeric aggregate and a nonzero exit.

`report.json` uses schema version 4 and records the family weights and the curve's `damage_scale` under `scoring`. It also contains the cohort fingerprint, runtime versions, eligibility and exit details, six `native.*` score features, and diagnostics. For each family it gives the affected fraction and the worst tile on the reference grid. Boxes use half-open `[left, top, right, bottom]` pixel coordinates. RGB RMSE and signed lightness, Lab chroma, and exposure offsets are diagnostic.

## Inspection-weighted scoring

The BokehForge benchmark and Stackroom generate an Inspection blend from the clean all-in-focus reference as part of scoring each valid delivery. Every alignment uses the same reference map. The report records the SHA-256 of the generated map. The scorer keeps the last reference map in memory to reuse across repeated comparisons. Use `--no-attention` to skip model downloads, loading, map generation, and attention weighting. For a valid delivery, a download or inference error withholds the score.

The raw map is the soft union of **ReLIQS**, **Texture & light**, **SAM 2.1 Hiera Large**, and **TempSAL overall**. The inputs use the same sRGB preview with a long edge of at most 640 pixels. The scorer resizes the blend to the native reference grid. Texture & light measures local detail, lighting variation, and color contrast. SAM contributes a zero map when no region meets the mask size criteria.

For relevance `H` and each damage-family value `D`, the scorer uses `D × (1 + α × (H / mean(H) − 1))`. The mean is over the entire reference. The mix is 50/50 while the brightest relevance value is at most twice the mean. As the peak rises above that point, the boost bends smoothly from 1.5× toward a limit of 2×. The adjustment preserves a mean multiplier of 1 and a minimum multiplier of 0.5. Constant maps give a multiplier of 1. The resulting damage values, up to 2, feed the existing mean, worst-1% and worst-tile calculations.

### Install the inspection models

From the repository root, install the pinned ONNX Runtime 1.24.4 and checkpoint conversion dependencies.

```sh
uv sync --project bokehforge --frozen --extra inspection
```

The first run with generated attention downloads and verifies the pinned checkpoints, converts them to ONNX with PyTorch, and stores the graphs under `onnx-v2/`. Conversion runs on CPU. Allow about 5 GB for the checkpoints and converted models. Each new process verifies the model cache and loads the native ONNX Runtime sessions on first use. Later scenes reuse the loaded sessions and compiled scoring kernels.

The default model directory is `~/.cache/bokehforge/inspection-models`. Set `XDG_CACHE_HOME` to change the cache root or `BOKEHFORGE_INSPECTION_MODELS` to use a specific model directory. A complete verified cache can be reused offline, and interrupted downloads resume on the next attempt. Set `ORT_DYLIB_PATH` to point to a custom ONNX Runtime shared library that supports API 24. [ReLIQS code](https://github.com/hakan-emre-gedik/ReLIQS/tree/6dd687552cb471c6b834236d917a3c3e43a28d4d) · [ReLIQS KonIQ checkpoint](https://huggingface.co/hakanemre/ReLIQS/blob/48761756eb3ca6d2386e17780cf2faa2622a8742/KONIQ/model_weights.pth) · [TempSAL PNAS source](https://github.com/IVRL/Tempsal/tree/4bdf3d5f173f390da0f714dfd10873c701d7ca32/src) · [SAM 2.1 Hiera Large snapshot](https://huggingface.co/facebook/sam2.1-hiera-large/tree/665f8e2ad61cf5f53d65644ff27c8ee525124610)

In Python, `measure(candidate, target)` and `score_saved_output(catalog, scene, path)` also generate attention by default. Pass `attention=False` to use the unweighted score. `measure` can also accept an explicit finite `[0, 1]` heatmap on the native reference grid through `inspection_heatmap`.

The `--explain-score` flag includes the exact attention values used to weight damage. The explanation’s `attention` field records native width and height, the 0–1 range, map mean, effective mix, and the range of damage multipliers. Its `data` is Base64 over zlib-compressed, little-endian float32 samples in row-major order. Decode and reshape to height × width to recover the native reference grid.

## Alignment evidence

The stacker supplies a single image. BokehForge first looks for a pixel-identical crop on the native grid. Otherwise it searches for a single translation, rotation and uniform scale. Coarse patch correlation proposes translations across the reference, and fine fitting compares local luma and two color-opponent signals. Fit and confirmation use alternating regions in a four-by-four delivery grid. A broader band can confirm the same scene when damage obscures fine detail. A native integer placement is preferred when the evidence is similar. Up to three distinct placements contribute to the score, and weak agreement reduces confidence smoothly toward a zero-score no-match.

The alignment search covers rotations within ±5° and scales around 0.35–2.5 reference pixels per delivery pixel. The image sizes can widen those bounds, within 0.2–8. Search resolution and the number of refinement starts are bounded. Small approximate crops, repetitive or low-texture scenes, severe local movement, and weak overlap can leave the pose unresolved. A near-native, low-information delivery may use a deterministic centered native placement. An exactly flat full-size delivery matching a flat reference scores 100 because these two images cannot reveal any earlier crop. Stretching and local warping are outside the fit and can affect the score.

An oversized canvas can overlap some or all of the reference. The scorer checks exact runs at the outer borders for flat fill and copied edge samples. Excluding a run requires evidence from the reference that it extends beyond the field, along with a matching interior edge. Runs inside the original field can be indistinguishable from occlusion or damaged borders and stay in the quality comparison. Genuine dark image content can still contribute.

The report’s scene-level `alignment` object and the Python API expose the same compact, versioned evidence with or without `--explain-score`.

| Field | What it records |
| --- | --- |
| `stages` | Exact-search attempts and outcomes, proposed edge runs, transform search bounds, retained seeds, and the fit and confirmation setup. |
| `candidates` | Approximate transforms considered after refinement, fine and broad agreement on fit and held-out regions, rank, and selection or rejection decision. Cell IDs run from 0 to 15 in row-major order on the search content. |
| `status`, `reason`, `confidence` | Match, uncertain placement, or no match, with the reason and total available score weight. |
| `placements` | Each evaluated transform and its evidence source, content box, padding decisions, clipped overlap polygon, `geometric_coverage`, `resolution_cap`, effective `coverage`, `overlap_score`, `weight`, and `score_contribution`. Sum the contributions to recover the scene score. |

An exact or low-information path can have an empty candidate list because its evidence is recorded in the stages. The feature readings and Stackroom overlay use the leading placement. The saved image remains the stacker’s original delivery.

## Score explanations

Add `--explain-score` to write complete diagnostics for each scored scene. The flag includes the alignment and damage breakdown, all eight damage grids for each measured placement, and the native attention map when attention weighting is enabled. The CLI defaults to the standard report. With the flag, `score_explanation_path` links each scene’s JSON under `score-explanations/`, and `score_aggregation` records the dataset and group scores, weights and scene counts.

In Python, `measure(candidate, target)` returns an object with `.score` and `.alignment`. Pass `details="full"` for the same complete explanation as the CLI. The Python API also accepts the focused detail levels `summary`, `tiles`, and `attention`. For an existing canonical TIFF, use `score_saved_output(catalog, scene, path, details="full")` from `bokehforge.bench`. This returns the score, alignment, delivery status, and explanation. It raises `ValueError` if the scene lacks exactly one supported reference.

- **Damage terms.** The `comparison` in each explained placement gives weighted damage and each family’s burden, weight and weighted damage. Its `pool` entries give the mean, worst-1% average, and worst tile with their values, weights and contributions. `damage_range` gives the valid range. The inspection profile also reports its effective `inspection_mix`. A family with zero local damage has no worst box.
- **Tile layout.** The `tile_grids` object contains eight grids. Consecutive `x_edges` and `y_edges` define each tile. Values, counts, and selected tile indices follow row-major order. Each value is supported damage summed over a tile and divided by `nominal_pixels`. Tiles with zero support are excluded from the worst-1% selection. A null `covered_pixels` field means the entire actual area of each tile was measured.
- **Overlay coordinates.** Boxes use zero-based, half-open `[left, top, right, bottom]` edge coordinates. The two-row `candidate_to_reference_xy` matrix maps the saved delivery’s `[x, y, 1]` pixel centers to reference pixel centers, with the upper-left center at `[0, 0]`. To draw a reference box on the delivery, subtract 0.5 from each edge coordinate, apply the inverse placement, add 0.5, then clip to `candidate_content_box_xyxy`. The overlap polygon uses the same reference coordinate system as the centers, so its pixel edges can start at −0.5. Use one grid at a time. A no-match delivery has a reason and no damage overlay.

## Comparing older runs

Older JSON keeps its original `score_version` or `profile` label. Compare scores with matching scorer versions and cohort fingerprints. To compare an older saved delivery with a new run, replay the original TIFF into a new output directory. Stackroom saves diagnostics for new evaluations and scores older deliveries on demand. For exact comparison of inspection-weighted runs, match the map hashes as well. Different inference providers can produce slightly different maps from the same model graphs.
