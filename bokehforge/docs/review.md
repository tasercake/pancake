# Local focus stack review

From the repository root, start the app and open one of the HTTP URLs it prints.

```sh
uv run --project bokehforge --frozen bokehforge review
```

The server listens on all IPv4 interfaces by default. From another device, open `http://<hostname>:8765` using the server's hostname or its LAN or Tailscale IPv4 address. The Tailscale MagicDNS short name works when it matches the server's hostname. Any device that can reach the port can review images, save ratings, and start downloads or evaluations. Use `--host 127.0.0.1` for access from the host computer only, or bind to a specific interface by giving its IPv4 address.

Review uses `datasets/` under the server's repository root. Pass `--root /path/to/datasets` to use a different directory. The shipped configuration selects InfinigenDefocus (D29). Use `--config` for a different source configuration, repeat `--dataset` to select specific configured datasets, or pass `--lock` for a pinned source snapshot.

When configured data is missing, select **Download dataset** in Review. The meter shows bytes received and estimated time remaining once there is enough transfer history. You can browse and rate existing scenes during the download. As each dataset is verified and indexed, its scenes appear automatically. Review receives catalog, transfer, and evaluation updates over one Server-Sent Events connection. If it disconnects, the browser reconnects and receives the current state. Downloads continue while the browser is closed. Retrying resumes partial files when the provider supports byte ranges. See the [dataset guide](data.md) for manual downloads and local logs.

Use the sidebar to scroll through scene previews and filter by dataset. Select a scene, enter a scene number, or press Up and Down to switch scenes. Scroll the filmstrip horizontally or press Left and Right to scrub the source frames in focus order.

Select an evaluation and a stacker to compare its output with a source frame and the available reference. The reference panel indicates when ground truth is unavailable. Click any comparison image or the expand control on a thumbnail to open it in the zoomable lightbox. In the lightbox, Left and Right wrap through the source frames, available outputs from the selected evaluation, and the displayed reference. Zoom and pan stay fixed as you switch images. Grade an output from 1 to 5, with 5 best, and save it with an optional comment. **Export ratings** downloads the saved grades and comments for every run as CSV.

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
