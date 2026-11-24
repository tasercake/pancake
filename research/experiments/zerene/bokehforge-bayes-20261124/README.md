# Zerene DMap tuning on BokehForge

The 24 November 2026 sweep completed **50 of 100 optimization trials** plus the trial 0 baseline. Validation completed at checkpoints 0, 10, 20, 30, 40, and 50. The sweep stopped by request during trial 51. Its 50 scored stacks are preserved as partial results and excluded from trial means and rankings.

| Result | Trial | Contrast percentile | Estimation radius | Smoothing radius | Overall PSNR (dB) |
| --- | ---: | ---: | ---: | ---: | ---: |
| Best tuning | 43 | 56 | 18 | 9 | 33.531 |
| Best held-out validation, checkpoint 10 | 1 | 66 | 10 | 9 | 33.522 |
| Default DMap, held-out validation | 0 | 50 | 10 | 5 | 33.247 |
| PMax, held-out validation | 0 | | | | 30.270 |

The best held-out DMap setting improved overall PSNR by **0.275 dB** over default DMap. It gained 0.724 dB on MFI-WHU and lost 0.173 dB on InfinigenDefocus. The tuning winner at checkpoint 50 scored 33.489 dB on validation. The reported best held-out score is the maximum among the six scheduled checkpoints on the same validation split.

## Validation checkpoints

Each checkpoint evaluated the DMap setting with the highest overall tuning PSNR available at that point. The dataset columns show held-out scores.

| Checkpoint | Selected trial | Tuning (dB) | Validation (dB) | MFI-WHU (dB) | InfinigenDefocus (dB) |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | 0 | 33.205 | 33.247 | 31.550 | 34.944 |
| 10 | 1 | 33.422 | 33.522 | 32.274 | 34.771 |
| 20 | 12 | 33.488 | 33.440 | 32.156 | 34.723 |
| 30 | 24 | 33.515 | 33.488 | 32.260 | 34.716 |
| 40 | 24 | 33.515 | 33.488 | 32.260 | 34.716 |
| 50 | 43 | 33.531 | 33.489 | 32.258 | 34.720 |

## Method

- Each split contained 50 MFI-WHU and 50 InfinigenDefocus stacks. BokehForge leakage groups determined the split. Each selected InfinigenDefocus stack came from a different scene seed. The archived source manifests identify the historical MFI-WHU revision used for this experiment. [Experiment configuration and split membership](experiment.json)
- Zerene Stacker 1.04 build T2026-05-04-0750 ran on Apple silicon macOS. This run treated the input frames as pre-aligned, kept native resolution, and passed them to Zerene as 16-bit sRGB TIFFs.
- Optuna multivariate TPE used tuning scores only. Its integer search space was 0–100 for contrast percentile, 1–32 pixels for estimation radius, and 0–24 pixels for smoothing radius. Trial 0 evaluated default DMap and the fixed PMax baseline on both splits.
- The objective was mean per-stack linear-RGB PSNR against each source's all-in-focus reference, with unit peak, a 120 dB cap, and equal weight for the two datasets. The record also includes linear-RGB mean absolute error.

## Result files

| File | Contents |
| --- | --- |
| [stack-scores.jsonl](stack-scores.jsonl) | 5,950 individual stack and method scores, including the 50 partial observations for trial 51. For validation, `trial` is the checkpoint number. |
| [evaluation-settings.json](evaluation-settings.json) | Parameters, methods, and observed and expected stack counts for every evaluated trial and checkpoint. |
| [tuning.json](tuning.json) | Aggregate scores for trials 0–50, overall and by dataset. |
| [checkpoints.json](checkpoints.json) | Validation aggregates, the tuning winner evaluated at each checkpoint, and the best validation result so far. |
| [summary.json](summary.json), [stopped.json](stopped.json) | Final checkpoint summary, baselines, and the stop record. |
| [preflight.json](preflight.json), [background-verification.json](background-verification.json) | Single-stack preflight scores and background-launch verification. |
| [code](code), [mac-repo-revision.txt](mac-repo-revision.txt) | Sweep and runner snapshots with hashes in the experiment manifest, plus the BokehForge checkout revision. |

Regenerate the three-panel score graph from the repository root.

```sh
uv run --project bokehforge --frozen --with optuna==4.6.0 --with matplotlib==3.10.8 \
  python research/experiments/zerene/bokehforge-bayes-20261124/render_scores.py \
  --output artifacts/zerene/bokehforge-bayes-20261124/scores.png
```
