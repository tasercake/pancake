# Focus-window and PNG pixel-budget regressions

On 2026-11-18, clean revision `e63cadd0937728b5d946f561fb2112d47707e0ff`
passed 30 tests across `cpu_fusion_contract` (3) and `cpu_image_io` (27).
Both new tests passed, with zero failures, ignored tests or filtered tests.
Linux `6.8.0-147-generic` x86-64 executed on an AMD Ryzen 9 7950X3D, using
Rust `1.100.0 (0ef1bc386 2026-11-09)` and logical CPUs 2 and 3.
[Original test log](test.log.gz) and [artifact checksum](SHA256SUMS).

`weighted_focus_uses_the_requested_window` submits two distinct seven-pixel
RGB rows to the CubeCL CPU focus accumulator for windows 1, 4, 5 and 7. It
checks every RGB sample against the independent double-precision weighted
mean of the hand-derived contrast traces after reflection and convolution.
Window 4 must produce the five-tap response. The seven-tap Gaussian uses
sigma 1.4. A zero-weight pixel retains the first source. The maximum
per-channel absolute error permitted by the test is `1e-6`.

`png_pixel_budget_is_checked_before_incomplete_raster` passes a 69-byte
PNG with valid chunk CRCs and one compressed RGB pixel to public `decode`.
Each 8,192-pixel axis fits the per-axis ceiling. The decoder must identify
the input and report the 50,000,000-pixel budget before attempting to read the
incomplete raster declared at 67,108,864 pixels.

Replay from `pancake/` at the measured revision.

```sh
taskset -c 2,3 nice -n 5 env \
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache \
  cargo +1.100.0 test --locked --package pancake --test cpu_fusion_contract \
  --test cpu_image_io -- --test-threads=1
```

Lockfile SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Uncompressed test log SHA-256 `23c05dd3e673f3060435d6fb0f48cd8d753b10a40cd35a73d00670a9903143b8`.
