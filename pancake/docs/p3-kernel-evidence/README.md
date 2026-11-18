# P3 fusion and resampling kernel receipts

Measured 2026-11-18 Singapore time on Linux x86_64, kernel 6.8.0-147-generic,
AMD Ryzen 9 7950X3D CPU and NVIDIA GeForce RTX 3090 at PCI 0000:01:00.0,
driver 580.178.04, 24,576 MiB device memory. Native CUDA used the CUDA 12.5
NVRTC libraries and identified the device as `ptx_sm86`. The tested Rust
toolchain was 1.100.0. Source revision
`87af2992274bb43aa217a29a75572222f9963387` includes the headless P1 workspace.
Its `pancake/Cargo.lock` SHA-256 is
`52095cb40f098ea2c59e54c36a6bcf634db233759e1aff7d57f28ecc2ff02a7c`.

The [CPU](../../tests/cpu_fusion_contract.rs) and
[GPU](../../tests/gpu_fusion_contract.rs) tests execute the same
[analytic inputs and expectations](../../tests/common/fusion_cases.rs).
The oracles cover ordered weighted accumulation, first-source ties and
zero weights, both complementary focus planes, genuine f32 identity gradients,
multichannel upsampling and antialiased downsampling, single-pixel axes,
Laplacian energy, and resource rejection before runtime acquisition.

| Check | Actual result |
|---|---|
| Test-first weighted/depth/resize, revision `9135ea0` | Compilation fails on missing public fusion module. [Red](fusion-red.log) |
| Test-first sharpness, revision `b00d356` | Compilation fails on missing public sharpness function. [Red](sharpness-red.log) |
| Native selector control, revision `95213a4` | The previous Vulkan-only test could not establish native CUDA affinity. [Red](native-selection-red.log) |
| Rust formatting, effective-policy Clippy | Pass. [Clippy](clippy.log) |
| CubeCL CPU | 2/2 tests. Twelve additional fresh processes each passed the two tests. [Results](cpu-vulkan.log), [fresh processes](cpu-fresh-processes.log) |
| Physical wgpu Vulkan | 1/1 test on RTX 3090. [Result](cpu-vulkan.log), [adapter](vulkan-adapter.txt) |
| Native CUDA | 1/1 test on RTX 3090. [Result](cuda.log), [device](cuda-adapter.txt) |

Early CubeCL CPU tests on a 33×1 complementary stack exposed wrong samples in
partial launch groups. Final kernels use an explicit active-lane check, including
one authoritative pixel count for the fused score and derived RGB indices.
Frame transfers use one new contiguous upload at a time while retaining
working scratch and accumulating outputs across frames. Twelve fresh CPU
processes exercised the same odd-tail inputs after these changes.

Run from `pancake/` with the locked toolchain:

```sh
export CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache
mkdir -p target/dependency-policy
cargo metadata --locked --format-version 1 --all-features \
  > target/dependency-policy/metadata.json
python3 scripts/dependency_policy.py --config deny.toml \
  --ledger dependency-policy.json \
  --metadata target/dependency-policy/metadata.json \
  --effective-config target/dependency-policy/deny-effective.toml \
  --license-inventory target/dependency-policy/licenses.json \
  --clippy-config clippy.toml \
  --effective-clippy-dir target/dependency-policy/clippy
cargo fmt --all -- --check
CLIPPY_CONF_DIR="$PWD/target/dependency-policy/clippy" \
  cargo clippy --locked --lib --test cpu_fusion_contract \
    --test gpu_fusion_contract -- -D warnings
cargo test --locked --test cpu_fusion_contract --test gpu_fusion_contract
CUDARC_CUDA_VERSION=12050 LD_LIBRARY_PATH=/usr/local/cuda-12.5/lib64 \
  PANCAKE_TEST_GPU_RUNTIME=cuda \
  cargo test --locked --features cubecl/cuda --test gpu_fusion_contract
```

The optional wrapper is a local registry build cache. Omit it if unavailable.
A separate native Metal run uses `PANCAKE_TEST_GPU_RUNTIME=metal` and
`--features cubecl/metal-native` on a provisioned Apple host.

These kernel results leave the P3 exit pending. The frozen 27 primary
case IDs require the integrated release CLI, image I/O, reports, automatic
settings, and weighted/depth slab execution. Apple execution, retained native
platform builds, aggregate assurance and product performance remain unverified
by this receipt.
