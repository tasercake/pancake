# BokehForge

BokehForge downloads verified focus-stacking datasets and loads their frames on demand from Python. It provides full-resolution float32 linear-sRGB pixels, optional TIFF bytes, and source provenance.

## Download data

From this directory, use the pinned project environment to download the sources you need.

```sh
uv sync --frozen
uv run --frozen bokehforge data download --dataset D29
```

Repeat `--dataset` to select multiple sources. Omitting it downloads the full configured inventory. `--root` selects the dataset directory, which defaults to `datasets/` at the repository root. From a wheel installation, pass `--config /path/to/sources.json` to select the source policy.

## Use from Python

In a larger Python 3.12 project, install BokehForge as a dependency from its project directory or wheel. For example, run `python -m pip install /path/to/focus-stacking/bokehforge` in your project's environment. Point the catalog at the same dataset directory used by the downloader.

```python
from bokehforge import FrameWindowDataset, FusionPairDataset, SceneCatalog

catalog = SceneCatalog("/path/to/datasets", datasets="D29")
scene = catalog.scene(catalog.scene_ids[0])
frame = catalog.decode(scene.frame_ids[0])  # .data, .alpha, .provenance
window = FrameWindowDataset(catalog, window_size=8, scene_ids=[scene.id])[0]
pair = FusionPairDataset(catalog, scene_ids=[scene.id], output="tiff")[0]
```

Reuse a catalog across views. It checks native inputs when it opens and decodes pixels when you request a frame or sample. The views work with PyTorch's `DataLoader` when PyTorch is installed in your application.

See the [data guide](docs/data.md) for sample fields, color and provenance contracts, and source configuration. Run the project tests with `uv run --frozen pytest`.
