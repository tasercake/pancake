# BokehForge

BokehForge downloads verified focus-stacking datasets and benchmarks delivered stacks against all-in-focus references. The Rust engine handles alignment, damage measurement, scoring, and Inspection inference. The Python package provides the CLI, NumPy API, dataset loaders, and stacker plug-in interface.

## Install from source

Use Python 3.12, `uv`, Rust 1.100 or later with Cargo, and a C++17-capable toolchain. A matching platform wheel includes the compiled engine.

| Platform | Source build tools |
| --- | --- |
| Linux x86-64 | A C/C++ compiler and linker, such as GCC and binutils |
| macOS 15 or later on Apple Silicon | Xcode Command Line Tools |
| Windows x64 | Rust’s MSVC toolchain, Visual Studio C++ Build Tools, and a Windows SDK |

The source build downloads and statically links the pinned Tracel LLVM 23.1.0-4 bundle for CubeCL. From this directory, install the locked environment.

```sh
uv sync --frozen
```

## Download data

Download InfinigenDefocus (D29), the shipped dataset. The loaders provide full-resolution float32 linear-sRGB pixels, optional TIFF bytes, and source provenance.

```sh
uv run --frozen bokehforge data download
```

The downloader, Python library, benchmark, and Stackroom share one per-user dataset directory across installations and Git worktrees. Set `BOKEHFORGE_DATA_ROOT` to reuse an existing directory. See the [data guide](docs/data.md) for platform defaults and custom sources.

## Benchmark stacks

Run a Python stacker against the downloaded targets or score rendered TIFFs from an external tool. BokehForge automatically selects CUDA, Apple Metal, or CPU. The benchmark prints one reference-fidelity score and saves per-scene diagnostics.

```sh
uv run --frozen bokehforge bench --algorithm mean --output ../artifacts/bench-mean --no-attention
```

Install the Inspection extra and omit `--no-attention` to use the default attention weighting. The first Inspection run converts the pretrained models once, then later runs reuse the verified model cache. [Benchmark setup, stacker contract, and saved-output replay](docs/benchmark.md)

## Browse and rate stacks in Stackroom

Stackroom is BokehForge’s local focus stack review app. Launch it from the repository root.

```sh
uv run --project bokehforge --frozen bokehforge review
```

See the [Stackroom guide](docs/review.md) for dataset selection, stacker setup, and rating export.

## Compare image-interest heatmaps

Open the image-interest viewer to compare all-in-focus reference photos and their raw or processed heatmaps.

```sh
uv run --frozen bokehforge heatmaps
```

Open the printed URL, normally `http://127.0.0.1:8765/`. Select the datasets you want to browse and click **Generate heatmaps** if any are missing. The app downloads verified originals as needed and shows progress while it prepares the datasets and generates the maps. Completed galleries are cached under `<data-root>/.bokehforge/heatmaps/` in the [shared BokehForge data directory](docs/data.md). They open automatically on later launches from any worktree.

The standard install generates **Texture & light** maps and a center-only comparison. The viewer’s **Inspection models** group uses its Python CUDA generator to display the blend and its component maps. Install the Inspection extra to enable that group on a CUDA machine. Use `--port` to choose a different port and Ctrl+C to stop the server. [Inspection setup](docs/benchmark.md#install-the-inspection-models)

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

Run local CUDA and Metal tests with `uv run --frozen pytest -m hardware_accelerator -v`.
Add `--extra inspection` to the `uv run` command on an Apple Silicon Mac to include
the Core ML ONNX fixture. These tests report which accelerators ran and why the
others were skipped.
