# Reproducible evaluation datasets

Only two entry points acquire or convert dataset files:

```sh
uv sync --frozen
uv run --frozen bokehforge data download
uv run --frozen bokehforge data process
```

Run from the repository root. Commands are provided by the Bokehforge project in [../pyproject.toml](../pyproject.toml). Python is pinned to 3.12.12 by the root `.python-version`; the root `uv.lock` pins Python dependencies and wheel hashes. `config/data/sources.json` describes all 28 research candidates. `config/data/sources.lock.json` freezes resolved source revisions, original file paths, available provider hashes, and SHA-256 hashes acquired on first successful download. Keep the lockfile with the code.

## Configuration

Edit [source policy](../config/data/sources.json) before source resolution. Configuration and pinned source identities live outside the disposable data directory. Current defaults:

```json
{
  "defaults": {
    "data_root": "../../datasets",
    "reserve_gib": 15,
    "max_download_bytes": 2147483648,
    "standard_format": "float32_RGB_TIFF"
  }
}
```

`data_root` is relative to the configuration file, or an absolute path. Both scripts also accept `--root /path/to/data` and `--reserve-gib 15`. Default storage is the repository's `datasets/` directory, relative to `config/data/sources.json`. The scripts never remove unrelated files to make space.

The default **total native-data budget is 2 GiB** (`2147483648` bytes). `bokehforge data download` writes `.bokehforge/state/download-plan.json` beneath the data root before acquiring payloads. It selects whole datasets or intact independently usable Figshare stack archives; it never keeps an arbitrary prefix of a focal sweep. Source sizes must be established before selection. `--max-download-gib 4` overrides the global cap for that run. This is a cap on unique selected native payloads; metadata requests and network retries add transfer overhead.

Selection uses stable source order and explicit per-source policy. Mite archives have a 150 MiB allocation; LFSD selects its complete original `focus_stack.rar`, with auxiliary LFP/algorithm-result archives excluded from the small default corpus. The previous pinned selection contained **2,037,407,175 bytes (1.897 GiB)** across 12 sources. Full oversized datasets are reported as `budget_excluded`. Per-source `acquisition_inputs`, `acquisition_max_bytes`, `acquisition_min_bytes` and `acquisition_priority` control allocation without changing source pins.

When a smaller plan is applied, existing out-of-plan originals move to `.bokehforge/cache/raw` and can be restored without network transfer if later selected. Native archives and images retain their original formats. No extra data are downloaded merely to populate the cache.

LSFD (`D01`) is **disabled by user decision** because it is not a feasible acquisition source. Its download route and remote-ZIP subset implementation have been removed. Normal runs, explicit `--dataset D01` runs and lock refreshes skip it without network requests. The disabled policy overrides stale source-lock entries, including previously available routes. Its catalogue ID remains only for provenance and exclusion reporting.

For example, set `defaults.data_root` to `/Volumes/Evaluation/focus-data` to use another disk. A smaller total cap can select zero units if no complete group fits; frames are never silently removed to meet the limit.

Select sources without changing the lock:

```sh
uv run --frozen bokehforge data download --dataset D05 --dataset D08
uv run --frozen bokehforge data process --dataset D05 --dataset D08 --workers 2
```

To deliberately resolve a different version or apply changed source selection:

```sh
uv run --frozen bokehforge data download --dataset D05 --refresh-lock --resolve-only
```

Only selected sources are refreshed; all other pins remain. Normal runs preserve successful pins and retry sources whose metadata resolution previously failed. Do not use `--refresh-lock` to reproduce an existing successful snapshot. Available provider versions and byte hashes, not a floating branch name or the research paper's count, define the acquired data.

Conversion-only fields (`layout`, `transfer`, `conversion_inputs`, and `conversion_priority`) can be changed without repinning original bytes. They become part of the conversion policy. For LFSD, canonical focal inputs come from `focus_stack.rar`; native light-field captures, saliency/depth data and published results remain original auxiliary assets, not extra focal planes. Dataset-specific priorities process ordinary regression sets before very large stress corpora. No frames are silently thinned.

Run the two data commands sequentially. The converter holds the acquisition lock to prevent source/cache moves during decoding. It processes exactly the native files selected by `.bokehforge/state/download-plan.json`; excluded older canonical scenes are retired. Changing the global budget requires running the downloader again before conversion. Selecting a dataset with `--dataset` does not bypass the total budget.

## Layout

```text
config/data/                    # durable source policy and exact source pins
  sources.json
  sources.lock.json
datasets/                       # generated; may be removed and rebuilt
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
    state/download-plan.json    # active subset and total native-byte cap
    probes/                     # provider discovery records
    cache/                      # retained out-of-plan originals
    work/                       # extraction, inventories, resume state and retired output
    locks/data.lock             # shared acquisition/processing lock
```

The entire generated `datasets/` directory is Git-ignored. Keep `config/data/` and the root dependency lock with the code. Downloaded data are not implicitly licensed for redistribution; source rights evidence is retained in config and manifests.

## Logs and reports

Both commands automatically append console output to `.bokehforge/logs/` beneath the selected data root. Reports hold the latest results in `.bokehforge/reports/`. Shell redirection is unnecessary. `--root` and `defaults.data_root` relocate all generated files together.

Source policy and source pins stay in `config/data/`. Removing the data directory discards downloaded/generated data and runtime history; the commands recreate its layout on their next run. Logs and reports are not retained across an explicit directory reset.

## Canonical signal contract

Frames are full-resolution, three-channel **float32 linear-sRGB TIFFs**, with lossless DEFLATE compression and no embedded wall-clock timestamp. No resizing, scene alignment, histogram matching, automatic exposure normalization, sharpening, or denoising is performed.

Native integer code values are normalized using their type range, never the observed image minimum/maximum. Declared sRGB inputs are linearized with the sRGB transfer function. Undocumented RGB sources use the explicit `srgb_assumed` contract; that assumption is preserved per frame. Embedded ICC profiles are transformed through LittleCMS to float32 sRGB before linearization. Source alpha is retained separately and is not claimed to be a focal-coverage mask. Unsupported orientation, color or array layouts fail instead of silently changing geometry.

Camera RAW development is explicit: LibRaw/rawpy, camera white balance, linear gamma, AHD demosaicing, no auto brightening, 16-bit RGB development followed by float32 normalization. This is a documented rendering of the original sensor data, not a claim of an unbounded HDR or exact radiance reference. Original RAW bytes remain untouched. Clipped sensor values cannot be recovered by this conversion.

Integer sources do not gain extra measured precision because the output container is float32. Actual floating-point input headroom and negative values are retained. Per-frame native dtype, ICC/decode metadata, source and output hashes, pinned package versions, and converter/recipe hashes remain in the manifest.

References, existing EDF composites, depth data, scale bars, saliency masks and published method outputs are never silently promoted to focal inputs or exact ground truth. Unrecognized layouts and incomplete pairs are quarantined with reasons. No new training/validation/test split is invented during format conversion. Native partition evidence and known specimen families remain available for a later audited split.

Waterloo MEF (`D28`) is an adjacent exposure-fusion control. Only its 17 `source image sequences` directories become canonical sequences. Its 136 fused images, score matrices and text remain auxiliary originals. `evaluation_domain: exposure_fusion_only` and `frame_role: exposure` distinguish these sequences from focal stacks. Published outputs and MOS are not all-focus pixel truth.

The global index joins exact overlapping source/reference bytes and known/conservative specimen families transitively. It does not prove that all semantic scene or asset overlap has been found. Use the index's active scene list. Obsolete generated scene directories are moved into `.bokehforge/work/retired` when grouping changes; original datasets are never deleted. Interrupted unpublished directories do not define dataset membership.

## Previously verified capped selection

Before the data directory was reset, the pinned 1.897 GiB native selection produced 1,949 focal stacks and 17 adjacent exposure sequences. Processed float32 data occupied approximately 39 GiB. These are historical counts; use new run reports for current local availability. The source count describes selected units, not an assertion that every archive in the original release was acquired.

| Source | Canonical sequences | Input frames | Scope |
|---|---:|---:|---|
| D03 Amber mite | 2 | 219 | Two complete optical ZIP stacks plus scale bars |
| D04 Amber spider | 22 | 1,014 | Optical archive and scale bars |
| D05 Lytro | 24 | 52 | Pairs and triples |
| D08 MFIFB | 105 | 210 | Native source pairs |
| D09 MFI-WHU | 120 | 240 | Pairs; clear-source references separate |
| D10 EDMF | 950 | 1,900 | 50 incomplete pairs quarantined |
| D11 HRMFF | 20 | 40 | Full high-resolution pairs |
| D19 Mobile Depth | 13 | 653 | Published aligned sequences |
| D20 FoD500 | 500 | 2,500 | Rendered five-frame stacks; depth remains auxiliary |
| D22 Cervix93 | 93 | 1,602 | EDF references separate |
| D25 LFSD | 100 | 845 | Complete published focal-stack archive |
| D28 Waterloo MEF | 17 | 130 | Exposure controls, not focal stacks |

No new train/validation/test assignment is implied. Existing source roles and conservative/exact-overlap groups remain in the manifests; semantic ancestry, rights eligibility and final split audit remain open.

## Idempotency and verification

Downloader verifies content hashes before reuse, resumes partial HTTP transfers, restarts safely when Range is ignored, rejects HTML access pages, and publishes files atomically after size/checksum verification. A complete verified partial file is promoted without a new transfer. Initial sources without a publisher checksum use the first successful acquisition as a recorded SHA-256 snapshot; this is not independent publisher authentication.

Converter verifies original file hashes, extracts archives with path/link checks, writes images and metadata atomically, and reuses outputs only after verifying source identity, recipe and output hashes. Per-original receipts allow interrupted long scenes to resume. Repeated runs with unchanged inputs and code preserve completed image bytes and modification times. Changing the conversion recipe invalidates affected cached results; identical pixels may be retained when the encoder contract is unchanged.

Byte reproduction is checked in the pinned runtime/platform. Cross-platform codec and RAW-development parity is not asserted without verification. The lockfile records source bytes independently of conversion-platform differences.

Implementation lives in `src/bokehforge/data/`; CLI entry point lives in `src/bokehforge/cli.py`. Tests live in `tests/data/` and `tests/test_cli.py`. Project metadata, Python pin and dependency lock live at repository root.

Keep code unchanged during a run. Conversion recipes record the code hashes loaded at process start, package versions, Python/platform identity, LibRaw and LittleCMS versions. An updated recipe is a new conversion snapshot, even when verified identical pixels permit TIFF byte reuse.

Run tests:

```sh
uv run --frozen pytest
```

Tests use generated fixtures and a local HTTP server. They cover zero-network reuse, permanent source exclusion with stale/fresh locks, corruption repair, incomplete transfers, HTTP Range behavior, complete-archive budget selection, archive path escape, original/reference separation, grouping, signal precision/headroom, repeat-run byte/mtime stability, and serial/parallel output equality.

The previous capped corpus passed a full repeat-run audit before the user reset its directory. That report and the data were not restored. Source pins were recovered unchanged from a local checkpoint; new acquisition validates against those pins.

## Unavailable and incomplete sources

Reports distinguish complete acquisition/conversion from disabled sources, inaccessible/gated sources, unresolved archive URLs, unrecognized layouts, quarantined inputs and insufficient storage. Disabled sources are intentional skips and do not cause failure. An all-source run exits with code 2 if any other selected dataset is unavailable or incomplete; this is intentional. Completed datasets remain usable and resumable. The two commands above are separate so conversion can process acquired sources even when another source is unavailable.

Some research candidates require an access form, have no published archive, or currently expose broken routes. The scripts do not submit contact-sharing forms, sign in through a browser, or substitute unrelated data. LSFD is excluded outright and is not retried if its endpoint changes.

Large float32 conversions can exceed the space needed for their original compressed files. The disk reserve is enforced during acquisition, archive extraction and conversion. A storage-blocked scene is not published as a complete stack. Set a larger `data_root` to build the full configured corpus.

The 2 GiB limit applies to native data, not processed TIFF size. Float32 linearization preserves signal values but can expand JPEG/PNG archives substantially. Canonical output remains float32 TIFF; original JPEG, PNG, TIFF, RAW and archive bytes are not rewritten.

LearnAF (`D17`) and the DFV-preprocessed DDFF HDF5 (`D21`) have public binary routes, but both exceed this machine's available storage. Their canonical adapters remain unimplemented: dual-pixel/estimated-depth arrays and HDF5 captures require explicit signal and grouping contracts. The converter fails on these layout names rather than guessing. Acquisition can still preserve their native archives on a larger disk. Other inaccessible catalogue entries retain their source-specific reason in `config/data/sources.lock.json` and the reports.
