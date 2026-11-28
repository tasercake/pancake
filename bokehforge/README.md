# BokehForge

BokehForge downloads verified focus-stacking datasets and loads their frames on demand from Python. It provides full-resolution float32 linear-sRGB pixels, optional TIFF bytes, and source provenance.

## Download data

From this directory, use the pinned project environment to download InfinigenDefocus (D29), the shipped dataset.

```sh
uv sync --frozen
uv run --frozen bokehforge data download
```

The downloader, Python library, benchmark, and Stackroom share one per-user dataset directory across installations and Git worktrees. Set `BOKEHFORGE_DATA_ROOT` to reuse an existing directory. See the [data guide](docs/data.md) for platform defaults and custom sources.

## Benchmark stacks

Run a Python stacker against the downloaded all-in-focus targets or score rendered TIFFs from an external tool. The benchmark prints one reference-fidelity score and saves artifact diagnostics for investigation. With the [inspection dependencies](docs/benchmark.md#install-the-inspection-models) installed, the default scorer downloads missing model files on first use. Use `--no-attention` with the base installation.

```sh
.venv/bin/bokehforge bench --algorithm mean --output ../artifacts/bench-mean
```

See the [benchmark guide](docs/benchmark.md) for the stacker contract, scoring profile, and saved-output replay.

## Browse and rate stacks in Stackroom

Stackroom is BokehForge’s local focus stack review app. Launch it from the repository root.

```sh
bokehforge/.venv/bin/bokehforge review
```

See the [Stackroom guide](docs/review.md) for dataset selection, stacker setup, and rating export.

## Compare image-interest heatmaps

Open the image-interest viewer to compare all-in-focus reference photos and their raw or processed heatmaps.

```sh
uv run --frozen bokehforge heatmaps
```

Open the printed URL, normally `http://127.0.0.1:8765/`. Select the datasets you want to browse and click **Generate heatmaps** if any are missing. The app downloads verified originals as needed and shows progress while it prepares the datasets and generates the maps. Completed galleries are cached under `<data-root>/.bokehforge/heatmaps/` in the [shared BokehForge data directory](docs/data.md). They open automatically on later launches from any worktree.

The standard install generates **Texture & light** maps and a center-only comparison. Install the [inspection dependencies](docs/benchmark.md#install-the-inspection-models) on a CUDA machine to also generate the **Inspection models** group. It includes the Inspection blend and its component maps. BokehForge downloads missing model files on first use. Use `--port` to choose a different port and Ctrl+C to stop the server.

## Use from Python

Install BokehForge in your Python 3.12 environment from this project directory or a wheel. For example, run `python -m pip install /path/to/focus-stacking/bokehforge`. The package includes the source configuration and file hashes. `load_dataset` downloads missing originals with live progress and decodes scenes as you iterate.

```python
from bokehforge import DATASET_INFINIGEN_DEFOCUS, load_dataset

dataset = load_dataset(DATASET_INFINIGEN_DEFOCUS)
for batch in dataset.batches(8):
    for sample in batch:
        frames, reference = sample.frames, sample.reference
    # Optional NumPy arrays when the shapes match:
    frames, references = batch.frames(), batch.references()
```

Pass a list of dataset constants or `DATASET_ALL` to load multiple built-in sources. Set `download=False` to use only local data. The [data guide](docs/data.md) covers batch shapes and the lower-level catalog and PyTorch-compatible views.

See the [data guide](docs/data.md) for sample fields, color and provenance contracts, and source configuration. Run the project tests with `uv run --frozen pytest`.
