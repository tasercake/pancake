# Local focus stack review

From the repository root, start the app and open one of the HTTP URLs it prints.

```sh
uv run --project bokehforge --frozen bokehforge review
```

The server listens on all IPv4 interfaces by default. From another device, open `http://<hostname>:8765` using the server's hostname or its LAN or Tailscale IPv4 address. The Tailscale MagicDNS short name works when it matches the server's hostname. Any device that can reach the port can review images, save ratings, and start evaluations. Use `--host 127.0.0.1` for access from the host computer only, or bind to a specific interface by giving its IPv4 address.

Use `--root /path/to/datasets` if the verified downloads are outside the default `datasets/` directory. The [dataset guide](data.md) covers downloading the sources. The app indexes downloaded scenes for the configured datasets. The shipped configuration selects D29 (InfinigenDefocus), including on data roots with older downloads. Repeat `--dataset` to select specific configured datasets, pass `--config` to use a different source configuration, or pass `--lock` to use pinned files from a source snapshot. The first index can take a while for large archives. Once initialized, the app loads individual images on demand and works offline. Add `--offline` to `uv run` if its environment and Python are already installed.

Use the sidebar to scroll through scene previews and filter by dataset. Select a scene, enter a scene number, or press Up and Down to switch scenes. Scroll the filmstrip horizontally or press Left and Right to scrub the source frames in focus order.

Select an evaluation and a stacker to compare its output with a source frame and the available reference. The reference panel indicates when ground truth is unavailable. Click any comparison image or the expand control on a thumbnail to open it in the zoomable lightbox. Grade an output from 1 to 5, with 5 best, and save it with an optional comment. **Export ratings** downloads the saved grades and comments for every run as CSV.

## Set up the stackers

Every stacker is selectable in **New evaluation**. When you start the run, Review uses any stackers it can already find and prepares the others before processing scenes. First-time setup may download dependencies. The run banner shows what is being installed, and setup failures include a log for the affected stacker. Other selected stackers can continue when one setup fails.

Review builds Pancake with its pinned Rust toolchain and lockfile. The host needs [Rust with rustup and the platform's build tools](../../pancake/DEVELOPING.md#build-from-source) for this step. Focus-stack uses the publisher's checksum-verified release where it matches the pinned submodule and host platform. The source-build fallback uses the host's C++ tools and OpenCV development files. OpenFocus and ShineStacker install from their pinned submodules into separate Python environments under the review store's `tools/` folder. Review reuses completed setups on subsequent runs.

Use `--pancake`, `--focus-stack`, `--openfocus`, or `--shine-python` to use an existing executable or the Python interpreter with ShineStacker installed. Review also recognizes the repository's standard build and submodule environment paths, and looks for the Pancake, focus-stack, and OpenFocus commands on `PATH`.

Select **New evaluation**, pick the current scene, its dataset, or all indexed datasets, and choose the tools. Each job runs serially in the background while you browse. It prepares full-resolution 16-bit sRGB TIFFs from BokehForge's linear frames, clipping to the SDR range for the external tools. Each tool uses its own processing precision. Runs use Pancake's default fusion on CPU, focus-stack's default complex-wavelet method, OpenFocus's StackMFF V3 on CPU, and ShineStacker's alignment and pyramid stack. Large stacks can take substantial time, memory, and temporary disk space. Stop a run from its progress banner. Any outputs already saved remain reviewable.

## Local files

By default the app writes to `artifacts/bokehforge-review/`. Use `--store` to choose a different location. A saved rating belongs to one run, scene, and stacker, so re-evaluating a scene creates a separate result to rate.

```text
ratings.sqlite3
previews-v2/                                    # Bounded display cache, rebuilt as needed
tools/                                          # Automatically installed stackers
runs/<run-id>/
  run.json                                      # Scope, cohort, methods, and interchange format
  progress.json                                 # Job state and output totals
  <stacker-id>/
    setup.log                                  # First-run setup output, when needed
    images/<scene-id>.tif                       # Full-resolution float32 linear-sRGB RGB
    results/<scene-id>.json                     # Outcome, input fingerprint, and relative file paths
    logs/<scene-id>.log                         # Tool command and output
```

A scene ID includes the dataset ID. The `images/` directory for any stacker can be passed to [`bokehforge bench --candidate-dir`](benchmark.md) for automated scoring. Select the matching scenes or dataset and a scoring profile appropriate to the output dimensions. The CSV uses `run_id`, `scene_id`, and `stacker` to join grades to the saved results.
