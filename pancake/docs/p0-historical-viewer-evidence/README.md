# Slint + CubeCL + wgpu image-viewer experiment

Standalone technical proof, not Pancake implementation. Uses pinned Slint 1.19,
CubeCL 0.11.3, one compatible wgpu 30 device and queue. Rust 1.100 is pinned.

From this worktree:

```sh
cargo +1.100.0 run --release --locked --manifest-path experiments/slint-cubecl-viewer/Cargo.toml
```

Four generated 1920×1080 images appear by default. Select an image, move exposure
slider to execute CubeCL, drag to pan, scroll or use +/− to zoom. Fit resets view.
Append JPEG/PNG/TIFF paths to load your own images:

```sh
cargo +1.100.0 run --release --locked --manifest-path experiments/slint-cubecl-viewer/Cargo.toml -- /absolute/photo1.jpg /absolute/photo2.tif
```

## Real data path

Sources decode/generate once as CPU RGBA f32 and upload into persistent CubeCL
buffers. CubeCL applies exposure/clipping and packs a padded RGBA8 display buffer.
A wgpu buffer→texture copy runs on that same device/queue. Slint imports the
texture without reading pixels to CPU. UI pan/zoom only changes Slint geometry.
Output buffers and textures persist for every source. No full-precision export,
stacking, CPU compute backend, streaming decode, or memory eviction is implemented.

The only unsafe application boundary describes correctly typed contiguous
CubeCL buffer arguments. The kernel guards tail invocations, and copy layout
uses 256-byte row alignment. Viewer retains each allocation for its entire life.

## Checks and measurement

```sh
cargo +1.100.0 test --locked --manifest-path experiments/slint-cubecl-viewer/Cargo.toml
cargo +1.100.0 run --release --locked --manifest-path experiments/slint-cubecl-viewer/Cargo.toml -- --benchmark --iterations 20
cargo +1.100.0 run --release --locked --manifest-path experiments/slint-cubecl-viewer/Cargo.toml -- --smoke-seconds 4
```

Benchmark emits JSON for 257×193 (odd width), 1920×1080, and 6000×4000.
Wall timings wait for GPU completion; they include CPU submission and queue wait,
not GPU timestamp-only execution. Initial call can include JIT; subsequent shapes
can reuse caches. Startup is not a guaranteed empty-cache measurement. Upload
includes allocation/initialization. Readback and scalar CPU oracle run only in
benchmark/smoke verification, never ordinary viewer updates. Maximum accepted
GPU/CPU preview difference is one 8-bit code value. Estimated GPU bytes are
logical resource sizes, not process RSS or driver/allocator residency. No reported
number is an end-to-end FPS claim. UI status shows submission time only.

## Programmatic rendering smoke

```sh
cargo +1.100.0 run --release --locked -- --smoke-seconds 4 --smoke-output artifacts/render-smoke
```

This saves `before.png` from Slint's real renderer, invokes normal image-selection
and exposure callbacks, then saves `after.png`. It requires over 1% of central
image pixels to change and verifies selected GPU output against CPU oracle.
`render-smoke.json` records this evidence. Snapshot capture performs two explicit
GPU readbacks; ordinary viewing still performs none. This verifies rendered
content, not physical keyboard/pointer input, window-manager presentation, or FPS.
Missing snapshots, unchanged pixels, failed oracle, or early GUI exit return a
nonzero process status. `--smoke-output /dev/null/invalid` exercises the error
path on Unix systems.
