# Slint + CubeCL + wgpu performance audit

Date: 2026-11-16, Asia/Singapore. Independent audit of the prototype built by a separate agent.

## Verdict

The shared-device architecture works on this Apple M4 Pro. CubeCL executes a real exposure-and-packing kernel, wgpu copies its output to a texture, and Slint imports that same texture. Normal viewer updates have no application-side pixel readback. The measured warm 24 MP processing pass takes a median **2.224 ms**, followed by a median **0.581 ms** buffer-to-texture copy, including host submission and GPU completion waits.

This establishes a useful technical integration proof, not a production performance guarantee. The kernel is a simple per-pixel preview operation, not focus stacking. Memory use, startup work on the UI thread, and missing presentation-latency measurement are more significant remaining concerns than the warm exposure kernel.

## Environment and reproducibility

| Item | Audited configuration |
|---|---|
| Hardware | Apple M4 Pro, 20 GPU cores, 48 GB unified memory |
| OS | macOS 26.2, build 25C56 |
| Build | Rust 1.100.0, release profile, thin LTO |
| Application packages | Slint 1.19.0; CubeCL and cubecl-wgpu 0.11.3 |
| GPU dependency | One resolved wgpu 30.0.2, shared by Slint and CubeCL |
| Execution adapter | Apple M4 Pro, IntegratedGpu, Metal |
| Numerical benchmark revision | `6b9db62`; subsequent smoke instrumentation does not change the processing kernel or copy path |

The lockfile also resolves several CubeCL internal crates to 0.11.4 through their permitted dependency ranges. The lockfile, rather than only top-level version strings, identifies the tested dependency set. CubeCL, Slint's version-specific wgpu API, and their unsafe resource boundaries need version pinning and review before adoption in Pancake.

Run from any directory:

```sh
cd /Users/krishna/Documents/focus-stacking/.worktrees/slint-cubecl-viewer-poc/experiments/slint-cubecl-viewer
cargo +1.100.0 run --release --locked
```

The default UI contains four generated 1920 × 1080 images. It supports image selection, exposure adjustment, pan, zoom, and Fit. JPEG, PNG, and TIFF paths can be supplied after `--`.

Repeat the headless numerical benchmark:

```sh
cd /Users/krishna/Documents/focus-stacking/.worktrees/slint-cubecl-viewer-poc/experiments/slint-cubecl-viewer
cargo +1.100.0 build --release --locked
mkdir -p artifacts/audit
for run in 1 2 3 4 5; do
  /usr/bin/time -l ./target/release/slint-cubecl-viewer --benchmark --iterations 30 > "artifacts/audit/benchmark-$run.json" 2> "artifacts/audit/benchmark-$run.stderr"
done
```

Tracked [AUDIT_MEASUREMENTS.json](AUDIT_MEASUREMENTS.json) contains all aggregated measurements and SHA-256 hashes of the retained raw JSON files. Raw logs remain under ignored `artifacts/audit/` in this worktree. They are local evidence, not published assets.

## Measurement method

Five separate release processes ran serially. Each process used three generated image sets: three 257 × 193 images, four 1920 × 1080 images, and two 6000 × 4000 images. Each set received one initial processing call and 30 measured iterations, rotating through its images. This gives 150 completed compute samples and 150 completed copy samples per size, 450 of each overall.

`Instant` measures host wall time. Each measured compute and copy stage is followed by `wgpu::Device::poll(Wait)`; these are completed-work measurements, not asynchronous enqueue timings. CubeCL's actual `ComputeClient::flush()` uses a blocking server call, and the wgpu server submits its command encoder to the same queue before the application's completion wait. This was verified in the installed crate sources.

The times include CPU dispatch, command preparation, driver overhead, queue delay, and completion waits. They are **not GPU timestamp-query durations**. The benchmark forces a wait between compute and copy, unlike the normal GUI path, so summing their medians does not measure the GUI update path. Presented FPS, event-to-photon latency, refresh-rate stability, and sustained thermal behavior are unmeasured.

Statistics below pool the 150 samples. P95 uses the empirical nearest-rank definition. These short runs are useful repeatability evidence; they are not independent 150-machine trials or a sustained-load study. Shader caches were not deleted. “First call” means first processing call for that shape in the process, not guaranteed empty-cache compilation.

## Completed-work results

| Image size | Compute median | Compute P95 | Compute maximum | GPU copy median | GPU copy P95 | GPU copy maximum |
|---|---:|---:|---:|---:|---:|---:|
| 257 × 193 | 0.160 ms | 0.412 ms | 0.456 ms | 0.149 ms | 0.458 ms | 0.526 ms |
| 1920 × 1080 | 0.817 ms | 0.904 ms | 0.932 ms | 0.273 ms | 0.783 ms | 0.940 ms |
| 6000 × 4000 | **2.224 ms** | **2.460 ms** | 3.456 ms | **0.581 ms** | **3.530 ms** | 4.467 ms |

The 24 MP copy tail is materially slower than its median. First use of a texture/allocation can involve driver work and residency initialization; the measurements do not isolate that cause. It would be incorrect to report only the minimum copy time or turn kernel time into an application FPS claim.

The first call and setup costs are substantial compared with warm processing:

| Work, median across five processes | Odd-size set | Four 1080p images | Two 24 MP images |
|---|---:|---:|---:|
| CPU synthetic generation | 0.462 ms | 23.807 ms | 136.971 ms |
| Allocation, initialization, upload, and waits | 0.798 ms | 26.424 ms | 189.217 ms |
| First completed compute for shape | 5.634 ms | 7.846 ms | 61.679 ms |
| Final buffer readback plus scalar CPU validation | 0.387 ms | 9.729 ms | 113.898 ms |

GPU runtime initialization alone has a median of 10.324 ms. The complete headless benchmark processes took approximately 0.75–0.77 seconds according to `/usr/bin/time`; these totals include all three sets, validation, setup, and shutdown. They are not GUI startup timings.

The measured CPU-oracle maximum RGB error was **zero 8-bit code values** in every validated case and process. Validation checks the final selected image for each shape, not every image and parameter combination. The generated images use opaque alpha; alpha preservation, negative exposure extremes, full-precision output, and the final rendered texture's numerical pixels are not covered by this oracle. Odd width 257 exercises padded row layout and tail invocation handling.

## Resource-sharing and synchronization audit

The implementation creates a `WgpuSetup` once through CubeCL. `WGPUConfiguration::Manual` gives Slint clones of that setup's instance, adapter, device, and queue. `slint::Image::try_from(texture.clone())` imports the exact persistent texture populated by `copy_buffer_to_texture`.

The installed Slint FemtoVG renderer's external-texture branch clones the native wgpu texture handle and passes it to `create_image_from_native_texture`; it does not render the input to a CPU buffer. The application's normal viewer callbacks contain no `read_one()` call. Readback is restricted to explicit benchmark/smoke validation. This supports the narrower claim **“no application pixel readback during normal display updates,”** rather than universal zero-copy rendering.

There is one full-image GPU copy per exposure or source-selection update. Compute outputs a padded RGBA8 buffer; the copy places it into a texture. This is not shared texture storage between the CubeCL buffer and Slint image. The input, packed output, and texture persist per source, so warm updates avoid full-size resource allocation. They still allocate command encoders, per-launch runtime state, status text, and an imported image wrapper.

CubeCL flush submits the producer before the application submits the copy. Slint uses the same queue, so ordered submissions provide the dependency. Each image's output `Handle` stays alive for the entire `Engine` lifetime, and the queued copy retains wgpu buffer references. The currently implemented persistent-allocation ownership avoids recycling an output slice while it is displayed. A future pooled/evicting implementation will need a renewed lifetime audit.

The GUI path does **not** call `Engine::wait()` on each update; its displayed “submission” time correctly does not claim GPU completion. However, update callbacks execute on the UI thread, and CubeCL flush/get-resource include blocking server interactions. First-use compilation and allocation can therefore stall input. Fast repeated slider events have no latest-value coalescing or cancellation, so they can enqueue obsolete full-image work. A worker and bounded latest-request queue are appropriate before production use.

Pan and zoom modify Slint geometry without rerunning the CubeCL kernel. Source selection currently reruns exposure and copy even if that image's matching exposure result is already resident. Caching a rendered result by source/exposure could eliminate that repeated work.

## Memory audit

| Retained resource | Bytes per pixel, excluding padding and allocator overhead |
|---|---:|
| CPU source `Vec<f32>` RGBA | 16 |
| GPU source f32 RGBA buffer | 16 |
| GPU packed output buffer | 4 |
| GPU display RGBA8 texture | 4 |
| Total logical resources | **40** |

Row padding makes the output slightly larger when a row is not a multiple of 256 bytes. The benchmark's GPU estimate is accurate for those logical resources: 3.717 MB for three odd-size images, 199.066 MB for four 1080p images, and 1.152512 GB for two 24 MP images. It does not measure allocation residency or reserved pools.

Two 24 MP sources additionally retain 768 MB in CPU vectors, giving approximately **1.921 GB** total logical source/output/texture storage before runtime overhead. Fifty 24 MP images would require approximately **48 GB** with this design before pools, staging, UI, or operating-system memory. This is not a scalable image-cache strategy.

Across the five complete benchmark sweeps, observed maximum RSS was 3.586–3.587 GB and peak physical footprint was approximately **4.912 GB**. These are whole-process high-water marks, including all size sets, staging, runtime pools, and oracle readback. They cannot be attributed solely to the final two images. Apple unified-memory accounting overlaps CPU/GPU memory; do not add RSS to the logical GPU estimate as if they were separate physical pools.

Each source load also creates a zero-filled host vector for output initialization and uploads it. That is unnecessary data movement if a fully writing kernel can safely initialize the whole required output. Sources are all decoded/generated before first window construction; the app retains their CPU vectors afterward primarily to support validation. Release CPU pixel storage after upload where possible, allocate display resources only for selected/cached images, and add a memory budget/LRU before large stacks.

## Display and validation limits

The exact Slint 1.19 convenience import used here accepts only `Rgba8Unorm` or `Rgba8UnormSrgb` textures and requires `TEXTURE_BINDING` plus `RENDER_ATTACHMENT` usage. This restriction was verified in installed `i-slint-core-1.19.0/graphics/wgpu_30.rs`, `TryFrom<wgpu::Texture>`. A floating-point compute source therefore does **not** prove a high-bit-depth or HDR viewer. HDR may require a different custom rendering integration.

Native CUA could not initially attach to the raw command-line executable. An ignored artifact-only `.app` wrapper registered as `dev.pancake.viewer.audit`, but attachment attempts timed out. No native physical click/drag/slider latency or accessibility-contract success is claimed by this audit. The programmatic rendered snapshots below are a separate narrower test.

## Ranked follow-up work

## Rendered evidence and steady-viewer memory addendum

The builder's four-image rendered smoke produced `artifacts/render-smoke/{before.png,after.png,render-smoke.json}`. Parent visually inspected both PNGs: the complete Slint UI and synthetic image rendered, Scene 2 became selected, and the exposure change was visible. The recorded 2360 × 1600 snapshots had 944,000 changed pixels in the central viewer region. This proves Slint actually consumes changed GPU output; it does not measure physical input or presentation timing.

The auditor independently loaded two generated uncompressed 6000 × 4000 RGB TIFF files through the real file-loader path, ran a 10-second rendered smoke, and sampled process RSS approximately every 0.5 seconds. The process exited 0; GPU/CPU oracle validation passed. The auditor visually inspected `artifacts/audit/gui-24mp/after.png`: the second TIFF and +1 EV setting were visible in the rendered UI. Raw stderr, samples, before/after PNGs, and smoke JSON are retained under `artifacts/audit/`.

| Two-24-MP GUI measurement | Observed value |
|---|---:|
| Settled RSS before first snapshot | 2.571 GB |
| Settled RSS after first snapshot and programmatic update | 2.611 GB |
| Maximum RSS across startup, snapshots, and oracle | 3.531 GB |
| Peak physical memory footprint across complete run | 4.791 GB |
| Rendered snapshots | 2360 × 1600; 944,000 changed central pixels |
| Result | Exit 0; programmatic rendered smoke passed |

The smoke performs two snapshot readbacks and one output-buffer oracle readback. These are explicit validation overhead and are absent from ordinary viewer operation. The UI's “readbacks: 0” status describes its ordinary update path; the smoke JSON separately records its two snapshot readbacks. The two source TIFF files are local generated artifacts, not downloaded photographs. This run is one observation, not a repeated GUI memory distribution.

To rerun after generating or providing two 24 MP TIFFs:

```sh
cd /Users/krishna/Documents/focus-stacking/.worktrees/slint-cubecl-viewer-poc/experiments/slint-cubecl-viewer
/usr/bin/time -l ./target/release/slint-cubecl-viewer --smoke-seconds 10 --smoke-output artifacts/audit/gui-24mp artifacts/audit/24mp-1.tif artifacts/audit/24mp-2.tif
```

Numerical benchmark statistics remain tied to the unchanged compute/copy path from `6b9db62`; the rendered and GUI memory observations use `e52df36`. All owned GUI/benchmark processes were stopped after measurement.
