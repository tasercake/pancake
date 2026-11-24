# Unattended Zerene Stacker experiments

[BokehForge DMap sweep results](bokehforge-bayes-20261124/README.md) include held-out validation, per-stack scores, and the code used for the run.

The [runner](run_stack.py) uses the installed macOS app and Zerene's [Batch API](https://zerenesystems.com/cms/stacker/docs/batchapi). It passes `-Dapple.awt.BackgroundOnly=true` to Java so Zerene can stack without taking keyboard focus. Zerene saves the results and exits automatically.

## Setup

- Use Python 3.9+ and a logged-in macOS desktop session. Zerene needs an active license or trial.
- After a fresh install or app move, open Zerene once and quit it. This creates or refreshes `~/Library/Preferences/ZereneStacker/zerenstk.launchcmd`. The runner uses the app's recorded Java command.
- Keep the interactive Zerene app closed and run batches serially. The command below uses `caffeinate -i` to prevent idle system sleep during the batch.

## Run one stack

From the repository root, pass a folder containing the TIFF or JPEG frames for one stack. You can also pass individual file paths. Use sequential filenames for focus order. Export raw frames to TIFF first. Keep previous results and ground truth outside the input folder, or pass an explicit file list.

```sh
caffeinate -i python3 research/experiments/zerene/run_stack.py "/path/to/frames" \
  --output artifacts/zerene/experiment-01
```

This writes `pmax.tif` and `dmap.tif` at 16 bits per channel. Use `--method pmax` or `--method dmap` for one method. Zerene aligns frames by default. Pass `--already-aligned` if the inputs are already registered.

## Sweep DMap settings

DMap uses a fixed contrast percentile to avoid the interactive threshold dialog. Defaults are percentile **50**, estimation radius **10**, and smoothing radius **5**. Other processing settings follow the installed app's preferences. Use a fresh output directory for each parameter combination.

```sh
for percentile in 35 50 65; do
  caffeinate -i python3 research/experiments/zerene/run_stack.py "/path/to/frames" \
    --method dmap --dmap-percentile "$percentile" \
    --dmap-estimation-radius 10 --dmap-smoothing-radius 5 \
    --output "artifacts/zerene/dmap-p${percentile}" || break
done
```

## Check results and retry

Each run saves `batch.xml`, `inputs.json`, and `zerene.log` alongside the TIFFs. Keep logs private because they can contain local paths and license diagnostics. The runner returns a nonzero exit status on failure, timeout, or missing or invalid expected TIFFs. Use `--timeout SECONDS` to change the one-hour limit. If a batch fails, check its log and use a fresh directory for the retry. A failed run can leave partial outputs.

Host verification on 20 Nov 2026 used Zerene 1.04 build T2026-05-04-0750. One unattended run processed 10 TIFF frames into two 5184 × 3888, 16-bit TIFFs in 33.5 seconds. The input and app preference hashes matched before and after the run.
