# Reproducible evaluation datasets

## Download verified originals

Run from the `bokehforge/` project directory. The project pins Python 3.12.12 and dependency wheel hashes. The default command downloads InfinigenDefocus (D29), about 9.09 GB in native files.

```sh
uv sync --frozen
uv run --frozen bokehforge data download
```

Repeat `--dataset` to select individual configured sources. Use `--root /path/to/data` to choose the data directory and `--reserve-gib 15` to control the disk reserve. The default root is `datasets/` at the repository root. `defaults.data_root` is relative to the source config file when it is a relative path.

[Source policy](../config/data/sources.json) describes the providers and conversion layouts. The downloader generates a local `sources.lock.json` alongside the source configuration with resolved revisions, original paths, and SHA-256 hashes. Use `--config` and `--lock` to choose these files explicitly. To resolve a different source revision without fetching the payloads, use `--refresh-lock --resolve-only` for the selected datasets. Normal downloads preserve successful pins. Older entries in the lock and on disk are used only when listed in the active source configuration.

```text
config/data/
  sources.json
  sources.lock.json
../datasets/
  raw/Dxx/
    download-manifest.json
    …                          # native files
  .bokehforge/
    logs/download.log
    reports/download-report.json
    probes/
    locks/data.lock
```

The downloader resumes partial HTTP transfers, verifies checksums before reusing files, and publishes verified downloads atomically. Reports distinguish completed, disabled, unavailable, and incomplete sources. A disabled source is an intentional skip. An unavailable or incomplete selected source causes exit code 2. Completed datasets are usable from their download manifests.

## Read data from Python

Add BokehForge to your Python 3.12 environment as a local project or wheel dependency. A catalog takes the downloader's data root and uses completed download manifests for configured sources by default.

```python
from bokehforge import FrameWindowDataset, FusionPairDataset, SceneCatalog

catalog = SceneCatalog("/path/to/datasets")
scene = catalog.scene(catalog.scene_ids[0])
frame = catalog.frame(scene.frame_ids[0])
decoded = catalog.decode(frame.id)
windows = FrameWindowDataset(catalog, window_size=8, stride=2, scene_ids=[scene.id])
pairs = FusionPairDataset(catalog, scene_ids=[scene.id], output="tiff")
```

| API | Result |
|---|---|
| `catalog.scene(id)` | Ordered `frame_ids` and `reference_ids`, plus scene metadata and a leakage group |
| `catalog.frame(id)` | Native member path, source SHA-256, page, ordinal, role, and focus metadata when available |
| `catalog.decode(frame_id)` | One frame or reference with `.data`, optional `.alpha`, and `.provenance` |
| `windows[i]` | `scene_id`, `frame_ids`, and consecutive focal `frames` within one scene |
| `pairs[i]` | The same fields for an adjacent focal pair, plus `reference_id` and `reference` |

Both views return full-resolution float32 HWC RGB NumPy arrays by default. Their frame payloads are tuples. Set `output="tiff"` on a view or on `catalog.decode` for canonical TIFF bytes. Payloads with source alpha include a separate alpha field. Fusion pairs come from scenes with exactly one reference. Windows advance by one frame when `stride` is omitted.

Catalog construction verifies native hashes, hashes optical archive members, and counts TIFF pages. Reuse the catalog across views to pay that cost once. It reads native files and archives directly, and decodes only the frames you request. Pass `lock=` to read source pins from a snapshot, `config=` to select a different source configuration, or `datasets=` to select specific configured downloads. Inspect `catalog.quarantine` for layout exclusions. Pixel decode failures surface when the affected frame is read, and each window or pair checks its focal dimensions.

For PyTorch, `DataLoader(windows, batch_size=4)` collates equal-shaped arrays by frame position. Use `collate_fn=list` for mixed resolutions or alpha layouts. Use `multiprocessing_context="spawn"` when decoding camera RAW in worker subprocesses. Keep batches and prefetching small for large frames.

## Color and provenance

Decoded RGB pixels use linear sRGB at native resolution. The decoder uses declared transfer functions and records its explicit sRGB assumption for undocumented inputs. It transforms embedded ICC profiles through LittleCMS and develops camera RAW with LibRaw, camera white balance, linear output, and AHD demosaicing. Float input headroom and negative values are preserved. TIFF encoding uses lossless DEFLATE compression and omits wall-clock timestamps.

Frame provenance records native identity, page, source hashes, color or RAW decode metadata, and the conversion recipe hash. TIFF results include an output SHA-256. Sources with alpha return it separately. Existing all-focus images, depth maps, scale bars, and published outputs retain their source roles. Recipe metadata records the decoder and loader code hashes, package versions, and runtime. TIFF byte reproducibility is verified on the pinned runtime and platform.

Leakage groups join exact source and reference byte overlaps and known specimen families across the selected corpus. Scene metadata retains known source partition and reference semantics for downstream evaluation.

## Pinned source

| ID | Source | Download size |
|---|---|---:|
| D29 | InfinigenDefocus | 9.09 GB |

InfinigenDefocus has 200 views. Each contributes five fixed-aperture stacks with nine frames in increasing focus-distance order from 0.8 m to 8.0 m. Frame metadata records the f-number and focus distance. Each aperture has an all-in-focus RGB reference, and views that share a scene seed share a leakage group. Depth maps and camera arrays are available in the native archive.
