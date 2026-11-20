# Repository instructions

## Zerene Stacker experiments

Use the [Zerene runner](research/experiments/zerene/run_stack.py) for unattended PMax and DMap experiments on the macOS host. Run from the repository root in a logged-in desktop session with a valid Zerene license or trial. Close the interactive Zerene app before starting a batch.

```sh
caffeinate -i python3 research/experiments/zerene/run_stack.py "/path/to/frames" \
  --output artifacts/zerene/experiment-01
```

Pass a TIFF/JPEG frame directory or an explicit list of frame files. Use a fresh output directory for each run and run batches serially. The runner saves its batch XML and log alongside the 16-bit TIFFs. Keep generated images and logs out of Git.

See the [Zerene experiment guide](research/experiments/zerene/README.md) for setup, parameter sweeps, and failure handling. Keep scripts for research experiments under `research/experiments/`.
