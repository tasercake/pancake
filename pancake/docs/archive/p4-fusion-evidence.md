# P4 multiscale fusion development evidence

2026-11-18, Linux x86-64 6.8.0-147-generic. Fusion source
`da04c99e454548442e3b8e2727ec9c4ea6d66622`, Rust 1.100.0 (`0ef1bc386`),
`Cargo.lock` SHA-256
`52095cb40f098ea2c59e54c36a6bcf634db233759e1aff7d57f28ecc2ff02a7c`.
CPU: AMD Ryzen 9 7950X3D, native CubeCL LLVM. GPU: physical NVIDIA GeForce
RTX 3090, PCI `0000:01:00.0`, driver 580.178.04, CubeCL wgpu Vulkan. The
registered device reported a single allocation limit of 2,147,483,644 bytes
and 64-byte alignment. This admits a 288,000,000-byte packed 24 MP f32 RGB
plane. Aggregate memory and completed job timing require separate measurement.

## Failing tests observed before implementation and fixes

- `cargo test --locked --test cpu_advanced_fusion --no-run` failed at
  `34dd609` with unresolved multiscale API (`E0432`).
- Early checked launch without explicit active-lane bounds corrupted the
  odd-grid CPU focus and precision oracles. The output exceeded its input
  color range and varied by channel. Constraining each kernel's logical lanes
  made the same oracles pass.
- `cpu_slab_geometry_and_scale_aware_parameters` rejected the early planner
  because it accepted one-frame slab size when there was only one input.
- `native_rgb_must_fit_one_device_buffer` initially failed with unresolved
  buffer preflight. The final test admits a 180-byte f32 buffer exactly and
  rejects a 179-byte page limit.
- The one-pixel 4+4+1 exposure regression returned RGB 0.6 before the batch sum
  owned independent device storage. The expected mean of the three batch means
  is 0.5. The final test accepts at most one u16 code of error.

## Scoped results

```sh
env -u WAYLAND_DISPLAY -u DISPLAY WGPU_BACKEND=vulkan \
  cargo test --locked --lib --test cpu_advanced_fusion \
  --test gpu_advanced_fusion -- --test-threads=1
cargo fmt --all -- --check
CLIPPY_CONF_DIR="$PWD/target/dependency-policy/clippy" \
  cargo clippy --locked --lib --test cpu_advanced_fusion \
  --test gpu_advanced_fusion -- -D warnings
```

All eight library tests, three CPU integration tests and one physical Vulkan
integration test passed. Format and the focused Clippy run with the generated
dependency policy passed. Both runtimes executed the same production kernels
against the same independent tests. Inputs included 33×19 opposing focus
planes, 1×1, 3×3 and 33×19 constant RGB grids at multiple depths, a native
16-bit gradient and nine distinct constant exposures. The host tests also
exercised disabled/overlapping slabs, invalid overlap/size, automatic
parameters, dimensions, and budget rejection before runtime initialization.
