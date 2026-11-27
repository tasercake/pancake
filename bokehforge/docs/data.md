# Reproducible evaluation datasets

## Download verified originals

Run from the `bokehforge/` project directory. The project pins Python 3.12.12 and dependency wheel hashes. The default command downloads InfinigenDefocus (D29), about 9.09 GB in native files.

```sh
uv sync --frozen
uv run --frozen bokehforge data download
```

Repeat `--dataset` to select individual configured sources. Use `--reserve-gib 15` to control the disk reserve. The downloader, benchmark, Stackroom, and `load_dataset` default to one shared per-user data directory.

| Platform | Default dataset directory |
|---|---|
| Linux | `$XDG_DATA_HOME/pancake/datasets` or `~/.local/share/pancake/datasets` |
| macOS | `~/Library/Application Support/pancake/datasets` |
| Windows | `%LOCALAPPDATA%/pancake/datasets` |

Set `BOKEHFORGE_DATA_ROOT` to an absolute path shared by your shells and worktrees. If you previously downloaded data to a checkout’s `datasets/` directory, point the variable there or move that directory to the new default to reuse the files. A CLI `--root` overrides the environment variable for that invocation. With neither override, a custom source configuration can set `defaults.data_root` relative to its own file.

The [bundled source policy](../src/bokehforge/data/sources.json) includes the dataset revision, original file paths, and SHA-256 hashes. The writable lock for bundled sources lives at `<data-root>/.bokehforge/sources.lock.json`. For a custom `--config`, the lock defaults to `sources.lock.json` next to that configuration. Pass `--lock` to specify a different file. To resolve custom source revisions without fetching payloads, use `--refresh-lock --resolve-only`. Older entries in the lock and on disk are used only when listed in the active source configuration.

```text
<data-root>/
  raw/Dxx/
    download-manifest.json
    …                          # native files
  .bokehforge/
    logs/download.log
    reports/download-report.json
    probes/
    locks/data.lock
    sources.lock.json          # writable lock for the bundled policy
```

The downloader reports transfer progress and checksum verification in the terminal and saves a local log. It resumes partial HTTP transfers, verifies checksums before reusing files, and publishes verified downloads atomically. Reports distinguish completed, disabled, unavailable, and incomplete sources. A disabled source is an intentional skip. An unavailable or incomplete selected source causes exit code 2. Completed datasets are usable from their download manifests.

## Read data from Python

Add BokehForge to your Python 3.12 environment as a local project or wheel dependency. `load_dataset` accepts `DATASET_INFINIGEN_DEFOCUS`, a list of dataset constants, or `DATASET_ALL` for every built-in dataset. It uses the shared data directory and downloads missing datasets on the first call. Use `download=False` to require local data. Catalog verification and indexing happen when the function opens the data. Pixel decoding happens when you iterate.

A sample contains one full ordered focus stack in `sample.frames`, its scene ID in `sample.scene_id`, and its all-in-focus image in `sample.reference` when exactly one reference is available. Use `dataset.batches(batch_size)` for iterable, indexable batches. The last batch may be shorter. The arrays contain native-resolution float32 linear-sRGB pixels.

| Batch method | Result |
|---|---|
| `batch.frames()` | `(B, F, H, W, C)` when focal counts and frame shapes match |
| `batch.references()` | `(B, H, W, C)` when every sample has a reference of the same shape |

The batch methods raise `ValueError` when the corresponding shapes are incompatible. Individual samples remain accessible through iteration or indexing.

### Low-level catalog and sample views

Use `SceneCatalog` to select individual frames, frame windows, or fusion pairs. Pass the downloader’s data root and optionally provide a custom source config.

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
