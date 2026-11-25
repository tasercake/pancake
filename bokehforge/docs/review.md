# Local focus stack review

From the repository root, start the app and open one of the HTTP URLs it prints.

```sh
uv run --project bokehforge --frozen bokehforge review
```

The server listens on all IPv4 interfaces by default. From another device, open `http://<hostname>:8765` using the server's hostname or its LAN or Tailscale IPv4 address. The Tailscale MagicDNS short name works when it matches the server's hostname. Any device that can reach the port can review images, save ratings, and start evaluations. Use `--host 127.0.0.1` for access from the host computer only, or bind to a specific interface by giving its IPv4 address.

Use `--root /path/to/datasets` if the verified downloads are outside the default `datasets/` directory. The [dataset guide](data.md) covers downloading the sources. The app uses their download manifests and indexes every scene in the local catalog by default. Repeat `--dataset` to limit a session, or pass the catalog's `--config` and `--lock` overrides. The first index can take a while for large archives. Once initialized, the app loads individual images on demand and works offline. Add `--offline` to `uv run` if its environment and Python are already installed.

Use the dataset filter, scene number, or Previous and Next to move through the carousel. The source filmstrip follows the original focus order. Select an evaluation and a stacker to compare its output with a source frame and the available reference. The ground-truth panel distinguishes all-in-focus ground truth from other source references and indicates when there is no reference. Click any image to inspect it at full resolution. Grade an output from 1 to 5, with 5 best, and save it with an optional comment. **Export ratings** downloads the saved grades and comments for every run as CSV.

## Set up the stackers

Browsing and rating saved runs work with the BokehForge environment alone. To generate new outputs, initialize the pinned sources and set up the tools you plan to run.

```sh
git submodule update --init .submodules/focus-stack .submodules/OpenFocus .submodules/shinestacker

# Pancake, with the native tools from the repository's build guide
(cd pancake && cargo build --locked --release -p pancake-cli)

# focus-stack, with OpenCV development files and pkg-config installed
make -C .submodules/focus-stack build/focus-stack

# OpenFocus and ShineStacker use independent environments
uv venv --python 3.12 .submodules/OpenFocus/.venv
uv pip install --python .submodules/OpenFocus/.venv ./.submodules/OpenFocus
uv venv --python 3.12 .submodules/shinestacker/.venv
uv pip install --python .submodules/shinestacker/.venv ./.submodules/shinestacker
```

The app discovers these build and environment paths. It also looks for `pancake-cli`, `focus-stack`, and `openfocus` on `PATH`. Use `--pancake`, `--focus-stack`, `--openfocus`, or `--shine-python` to point to an existing executable or the Python interpreter with ShineStacker installed. Restart the app after setting up a stacker so it can discover the executable.

Select **New evaluation**, pick the current scene, its dataset, or all indexed datasets, and choose the tools. Each job runs serially in the background while you browse. It prepares full-resolution 16-bit sRGB TIFFs from BokehForge's linear frames, clipping to the SDR range for the external tools. Each tool uses its own processing precision. Runs use Pancake's default fusion on CPU, focus-stack's default complex-wavelet method, OpenFocus's StackMFF V3 on CPU, and ShineStacker's alignment and pyramid stack. Large stacks can take substantial time, memory, and temporary disk space. Stop a run from its progress banner. Any outputs already saved remain reviewable.

## Local files

By default the app writes to `artifacts/bokehforge-review/`. Use `--store` to choose a different location. A saved rating belongs to one run, scene, and stacker, so re-evaluating a scene creates a separate result to rate.

```text
ratings.sqlite3
runs/<run-id>/
  run.json                                      # Scope, cohort, methods, and interchange format
  progress.json                                 # Job state and output totals
  <stacker-id>/
    images/<scene-id>.tif                       # Full-resolution float32 linear-sRGB RGB
    results/<scene-id>.json                     # Outcome, input fingerprint, and relative file paths
    logs/<scene-id>.log                         # Tool command and output
```

A scene ID includes the dataset ID. The `images/` directory for any stacker can be passed to [`bokehforge bench --candidate-dir`](benchmark.md) for automated scoring. Select the matching scenes or dataset and a scoring profile appropriate to the output dimensions. The CSV uses `run_id`, `scene_id`, and `stacker` to join grades to the saved results.
