# Stackroom

Stackroom is BokehForge’s local app for comparing and rating focus stacks. Launch it from the repository root and open one of the HTTP URLs it prints.

```sh
bokehforge/.venv/bin/bokehforge review
```

Stackroom uses attention-weighted scores by default. Install the [inspection dependencies](benchmark.md#install-the-inspection-models) once. Stackroom downloads missing model files on first use. Start Stackroom with `--no-attention` for the unweighted score.

The server listens on all IPv4 interfaces by default. From another device, open `http://<hostname>:8765` using the server's hostname or its LAN or Tailscale IPv4 address. The Tailscale MagicDNS short name works when it matches the server's hostname. Any device that can reach the port can review images, save ratings, and start downloads or evaluations. Use `--host 127.0.0.1` for access from the host computer only, or bind to a specific interface by giving its IPv4 address.

Stackroom uses `datasets/` under the server's repository root. Pass `--root /path/to/datasets` to use a different directory. The shipped configuration selects InfinigenDefocus (D29). Use `--config` for a different source configuration, repeat `--dataset` to select specific configured datasets, or pass `--lock` for a pinned source snapshot.

When configured data is missing, select **Download dataset** in Stackroom. The meter shows bytes received and estimated time remaining once there is enough transfer history. You can browse and rate existing scenes during the download. As each dataset is verified and indexed, its scenes appear automatically. Stackroom receives catalog, transfer, and evaluation updates over one Server-Sent Events connection. If it disconnects, the browser reconnects and receives the current state. Downloads continue while the browser is closed. Retrying resumes partial files when the provider supports byte ranges. See the [dataset guide](data.md) for manual downloads and local logs.

Open **Scenes** in the command bar to scroll through scene previews and filter by dataset. Select a scene, enter a scene number, or press Up and Down to switch scenes. Scroll the filmstrip horizontally or press Left and Right to scrub the source frames in focus order.

Use the **Evaluation** inspector to select a run and a stacker. Stackroom places its output beside the source frame and the available reference. The inspector indicates when ground truth is unavailable. Click any comparison image or the expand control on a thumbnail to open it in the zoomable lightbox. Scroll the lightbox strip to browse the source frames, ready outputs from the selected evaluation, and any references. Hover over a thumbnail to preview it and move away to return to the selected image. Click or tap a thumbnail to select it, or use Left and Right to wrap through the strip. Zoom and pan stay fixed as you browse. Select **Swipe reference** and drag the divider to compare the image with its reference, or hold the period key (`.`) to see the reference across the whole view. You can pan while holding the key and the image resumes at the same position when you release it. In the inspector, grade an output from 1 to 5, with 5 best, and save it with an optional comment. **Export ratings** downloads the saved grades and comments for every run as CSV.


Use **Appearance** to set the accent. Stackroom remembers it in your browser and adjusts dark colors for contrast. **Reset accent** restores white.

For scenes with one supported RGB ground-truth image, each ready output shows the benchmark’s per-scene score out of 100 below its image, on its stacker tab, and in the lightbox header while you view or preview the output. Stackroom uses the [benchmark scorer](benchmark.md#the-score). It infers where the output belongs on the sharp reference and grades retained area and native-resolution fidelity. Exact crops earn their share of the reference area. Select the **i** beside **Invalid delivery** for the reason. In the lightbox, press **I** to open it while previewing a thumbnail. Scores are computed in the background when you open a scene and cached for subsequent visits.

Open **Explain score** to see the inferred area, resolution cap, overlap quality, and match confidence. **How alignment was inferred** shows search evidence and the score contribution of each retained placement. An unmatched delivery still has an explanation with the reason.

Expand a damage family to see the pooling values and weights. Turn on **Show damage overlay**, choose the 8- or 32-pixel grid, and use the 0–1 legend. The overlay uses the leading placement. Zero-damage and unmeasured tiles are transparent. In the lightbox, the overlay follows zoom and pan, clips to the output side of a swipe, and disappears when you show only the reference. Stackroom calculates the tile data when you request the overlay.

## Set up the stackers

Choose the stackers in **New evaluation**. When you start the run, Stackroom uses the available open-source stackers and prepares any others you selected before processing scenes. First-time setup may download dependencies. The run banner shows what is being installed, and setup failures include a log for the affected stacker. Other selected stackers can continue when one setup fails.

Stackroom builds Pancake Stacker with its pinned Rust toolchain and lockfile. The host needs [Rust with rustup and the platform's build tools](../../pancake/DEVELOPING.md#build-from-source) for this step. Focus-stack uses the publisher's checksum-verified release where it matches the pinned submodule and host platform. The source-build fallback uses the host's C++ tools and OpenCV development files. OpenFocus and ShineStacker install from their pinned submodules into separate Python environments under the Stackroom store’s `tools/` folder. Stackroom reuses completed setups on subsequent runs.

Use `--pancake`, `--focus-stack`, `--openfocus`, or `--shine-python` to use an existing executable or the Python interpreter with ShineStacker installed. Stackroom also recognizes the repository's standard build and submodule environment paths, and looks for the Pancake Stacker, focus-stack, and OpenFocus commands on `PATH`.

On macOS and Windows, **Zerene Stacker** uses the licensed or trial installation on the computer running Stackroom. Sign in to the desktop on that computer, open Zerene once to record its launch settings, then quit the interactive app before starting an evaluation. Stackroom runs its local Java batch process and saves the result alongside the other stackers. The Zerene option is hidden on Linux. See the [Zerene Batch API](https://zerenesystems.com/cms/stacker/docs/batchapi).

## Configure an evaluation

Select the current scene, its dataset, or all indexed datasets. Each stacker has a **Settings** accordion. Leave it closed for a quick run with the defaults, or open it to choose processing options.

| Stacker | Quick-run settings | Options |
| --- | --- | --- |
| Pancake Stacker | Hybrid fusion, alignment off, automatic device | Fusion method, alignment, cropping, CPU or GPU selection |
| focus-stack | Aligned complex-wavelet fusion, OpenCL when available | Alignment, cropping, consistency filter, denoise strength, CPU or automatic OpenCL |
| OpenFocus | StackMFF V3 with soft blending and exposure matching, alignment off, automatic device | Fusion method, alignment, exposure matching, V3 blend mode, CPU or automatic acceleration for supported methods |
| ShineStacker | Aligned pyramid fusion | Pyramid or depth map fusion, alignment, pyramid window size |
| Zerene Stacker | Aligned PMax fusion | PMax or DMap, alignment, DMap contrast percentile and filtering radii |

ShineStacker and Zerene use their built-in processing without a GPU setting. Each job runs serially in the background while you browse. Stackroom prepares full-resolution 16-bit sRGB TIFFs from BokehForge's linear frames, clipping to the SDR range for the external tools. Large stacks can take substantial time, memory, and temporary disk space. Stop a run from its progress banner. Any outputs already saved remain reviewable.

## Local files

By default Stackroom writes to `artifacts/bokehforge-review/`. Use `--store` to choose a different location. **Export ratings** downloads `stackroom-ratings.csv`. A saved rating belongs to one run, scene, and stacker, so re-evaluating a scene creates a separate result to rate.

```text
ratings.sqlite3
previews-v2/                                    # Bounded display cache, rebuilt as needed
tools/                                          # Automatically installed stackers
runs/<run-id>/
  run.json                                      # Scope, cohort, settings, and interchange format
  progress.json                                 # Job state and output totals
  <stacker-id>/
    setup.log                                  # First-run setup output, when needed
    images/<scene-id>.tif                       # Full-resolution float32 linear-sRGB RGB
    results/<scene-id>.json                     # Outcome, input fingerprint, and relative file paths
    scores/<scene-id>.json                      # Cached per-scene benchmark result
    logs/<scene-id>.log                         # Tool command and output
```

A scene ID includes the dataset ID. The `images/` directory for any stacker can be passed to [`bokehforge bench --candidate-dir`](benchmark.md) for automated scoring. Select the matching scenes or dataset and a scoring profile appropriate to the output dimensions. The CSV uses `run_id`, `scene_id`, and `stacker` to join grades to the saved results.
