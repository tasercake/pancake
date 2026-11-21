# Reproducible evaluation datasets

Only two entry points acquire or convert dataset files:

```sh
uv sync --frozen
uv run --frozen bokehforge data download
uv run --frozen bokehforge data process
```

Run from the `bokehforge/` project directory. Commands are provided by the Bokehforge project in [../pyproject.toml](../pyproject.toml). Python is pinned to 3.12.12 by the project `.python-version`; the project `uv.lock` pins Python dependencies and wheel hashes. `config/data/sources.json` describes the eight dataset sources. `config/data/sources.lock.json` freezes resolved source revisions, original file paths, available provider hashes, and SHA-256 hashes acquired on first successful download. The downloader generates this local file. Run download before processing in a fresh checkout.

## Configuration

Edit [source policy](../config/data/sources.json) before source resolution. Configuration and pinned source identities live outside the disposable data directory. Current defaults:

```json
{
  "defaults": {
    "data_root": "../../../datasets",
    "reserve_gib": 15,
    "standard_format": "float32_RGB_TIFF"
  }
}
```

`data_root` is relative to the configuration file, or an absolute path. Both scripts also accept `--root /path/to/data` and `--reserve-gib 15`. Default storage is the repository-root `datasets/` directory, resolved relative to `bokehforge/config/data/sources.json`. The scripts never remove unrelated files to make space.

The default command downloads every file in the pinned inventories, about **18.98 GB** before extraction. Use repeated `--dataset` flags to download specific sources. The disk reserve applies to downloads, extraction and conversion.

Select sources without changing the lock:

```sh
uv run --frozen bokehforge data download --dataset D04 --dataset D20
uv run --frozen bokehforge data process --dataset D04 --dataset D20 --workers 2
```

To deliberately resolve a different version or apply changed source selection:

```sh
uv run --frozen bokehforge data download --dataset D29 --refresh-lock --resolve-only
```

Only selected sources are refreshed; all other pins remain. Normal runs preserve successful pins and retry sources whose metadata resolution previously failed. Do not use `--refresh-lock` to reproduce an existing successful snapshot. Available provider versions and byte hashes, not a floating branch name or the research paper's count, define the acquired data.

Run download and process sequentially. The processor verifies all pinned files for each selected source before standardizing it. The index includes only sources in the current lock. Older source and output directories on disk are preserved.

## Layout

```text
config/data/                    # durable policy and pins inside bokehforge/
  sources.json
  sources.lock.json
../datasets/                    # generated at repository root; may be rebuilt
  raw/Dxx/                      # untouched provider files
  standardized/Dxx/scene/
    frames/000000.tif           # canonical focal inputs
    references/000000.tif       # separately labeled references
    scene.json                  # per-scene signal and provenance contract
  standardized/index.json       # authoritative active scenes and leakage groups
  .bokehforge/
    logs/download.log           # appended output, also shown in console
    logs/process.log
    reports/download-report.json
    reports/process-report.json
    probes/                     # provider discovery records
    work/                       # extraction, inventories, resume state and retired output
    locks/data.lock             # shared acquisition/processing lock
```

The entire generated `datasets/` directory is Git-ignored. Keep the source policy and dependency lock with the code. Downloaded data are not implicitly licensed for redistribution; source rights evidence is retained in config and manifests.

## Logs and reports

Both commands automatically append console output to `.bokehforge/logs/` beneath the selected data root. Reports hold the latest results in `.bokehforge/reports/`. Shell redirection is unnecessary. `--root` and `defaults.data_root` relocate all generated files together.

Source policy and source pins stay in `config/data/`. Removing the data directory discards downloaded/generated data and runtime history; the commands recreate its layout on their next run. Logs and reports are not retained across an explicit directory reset.

## Canonical signal contract

Frames are full-resolution, three-channel **float32 linear-sRGB TIFFs**, with lossless DEFLATE compression and no embedded wall-clock timestamp. No resizing, scene alignment, histogram matching, automatic exposure normalization, sharpening, or denoising is performed.

Native integer code values are normalized using their type range, never the observed image minimum/maximum. Declared sRGB inputs are linearized with the sRGB transfer function. Undocumented RGB sources use the explicit `srgb_assumed` contract; that assumption is preserved per frame. Embedded ICC profiles are transformed through LittleCMS to float32 sRGB before linearization. Source alpha is retained separately and is not claimed to be a focal-coverage mask. Unsupported orientation, color or array layouts fail instead of silently changing geometry.

Camera RAW development is explicit: LibRaw/rawpy, camera white balance, linear gamma, AHD demosaicing, no auto brightening, 16-bit RGB development followed by float32 normalization. This is a documented rendering of the original sensor data, not a claim of an unbounded HDR or exact radiance reference. Original RAW bytes remain untouched. Clipped sensor values cannot be recovered by this conversion.

Integer sources do not gain extra measured precision because the output container is float32. Actual floating-point input headroom and negative values are retained. Per-frame native dtype, ICC/decode metadata, source and output hashes, pinned package versions, and converter/recipe hashes remain in the manifest.

References, existing EDF composites, depth data, scale bars, saliency masks and published method outputs are never silently promoted to focal inputs or exact ground truth. Unrecognized layouts and incomplete pairs are quarantined with reasons. No new training/validation/test split is invented during format conversion. Native partition evidence and known specimen families remain available for a later audited split.

The global index joins exact overlapping source/reference bytes and known/conservative specimen families transitively. It does not prove that all semantic scene or asset overlap has been found. Use the index's active scene list. Obsolete generated scene directories are moved into `.bokehforge/work/retired` when grouping changes; original datasets are never deleted. Interrupted unpublished directories do not define dataset membership.

## Pinned sources

| ID | Source | Download size |
|---|---|---:|
| D02 | Sood 999-frame macro sweep | 8.96 GB |
| D04 | Amber spider optical stacks | 262.35 MB |
| D09 | MFI-WHU | 18.17 MB |
| D10 | EDMF | 128.61 MB |
| D11 | HRMFF | 225.12 MB |
| D19 | Mobile Depth | 56.66 MB |
| D20 | FoD500 | 232.85 MB |
| D29 | InfinigenDefocus | 9.09 GB |

### InfinigenDefocus

```sh
uv run --frozen bokehforge data download --dataset D29
uv run --frozen bokehforge data process --dataset D29
```

The pinned ZIP contains 200 rendered indoor views. Each view contributes five fixed-aperture stacks of nine frames, ordered by focus distance from 0.8 m to 8.0 m. Frame records retain f-number and focus distance. The processor writes the all-in-focus RGB render as a reference for each aperture. Depth maps and camera arrays stay in the native archive and extraction directory. Views and apertures that share a scene seed share a leakage group. Allow space for the ZIP, extracted files and float32 TIFFs.

Use these renders for synthetic indoor reference tests. Real macro coverage and asset-level split independence require separate checks. See the [publisher's dataset card](https://huggingface.co/datasets/venkatsubra/InfinigenDefocus).

## Idempotency and verification

Downloader verifies content hashes before reuse, resumes partial HTTP transfers, restarts safely when Range is ignored, rejects HTML access pages, and publishes files atomically after size/checksum verification. A complete verified partial file is promoted without a new transfer. Initial sources without a publisher checksum use the first successful acquisition as a recorded SHA-256 snapshot; this is not independent publisher authentication.

Converter verifies original file hashes, extracts archives with path/link checks, writes images and metadata atomically, and reuses outputs only after verifying source identity, recipe and output hashes. Per-original receipts allow interrupted long scenes to resume. Repeated runs with unchanged inputs and code preserve completed image bytes and modification times. Changing the conversion recipe invalidates affected cached results; identical pixels may be retained when the encoder contract is unchanged.

Byte reproduction is checked in the pinned runtime/platform. Cross-platform codec and RAW-development parity is not asserted without verification. The lockfile records source bytes independently of conversion-platform differences.

Implementation lives in `src/bokehforge/data/`; CLI entry point lives in `src/bokehforge/cli.py`. Tests live in `tests/data/` and `tests/test_cli.py`. Project metadata, Python pin and dependency lock live in `bokehforge/`.

Keep code unchanged during a run. Conversion recipes record the code hashes loaded at process start, package versions, Python/platform identity, LibRaw and LittleCMS versions. An updated recipe is a new conversion snapshot, even when verified identical pixels permit TIFF byte reuse.

Run tests:

```sh
uv run --frozen pytest
```

Tests use generated fixtures and a local HTTP server. They cover zero-network reuse, permanent source exclusion with stale/fresh locks, corruption repair, incomplete transfers, HTTP Range behavior, archive path escape, original/reference separation, grouping, signal precision/headroom, repeat-run byte/mtime stability, and serial/parallel output equality.


## Unavailable and incomplete sources

Reports distinguish complete acquisition/conversion from disabled sources, inaccessible/gated sources, unresolved archive URLs, unrecognized layouts, quarantined inputs and insufficient storage. Disabled sources are intentional skips and do not cause failure. An all-source run exits with code 2 if any other selected dataset is unavailable or incomplete; this is intentional. Completed datasets remain usable and resumable. The two commands above are separate so conversion can process acquired sources even when another source is unavailable.


Large float32 conversions can exceed the space needed for their original compressed files. The disk reserve is enforced during acquisition, archive extraction and conversion. A storage-blocked scene is not published as a complete stack. Set a larger `data_root` to build the full configured corpus.

Float32 linearization preserves signal values but can expand JPEG/PNG archives substantially. Canonical output remains float32 TIFF; original JPEG, PNG, TIFF, RAW and archive bytes are not rewritten.
