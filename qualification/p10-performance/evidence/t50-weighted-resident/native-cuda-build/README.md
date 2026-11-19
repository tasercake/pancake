# Native CUDA Weighted resident probe build

The locked release test executable compiled and linked from exact
`a49ce692371558ff40f72b0af77f5249d00dcedb` plus a temporary test-only
observer. The Cargo.lock SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The build used Rust 1.100.0, `CUDARC_CUDA_VERSION=12050`, two direct Cargo
jobs on CPUs 10 and 11, and the existing native CUDA feature.

```sh
cd pancake
CUDARC_CUDA_VERSION=12050 taskset -c 10,11 cargo +1.100.0 test \
  --locked --offline --release -p pancake --lib --features cubecl/cuda \
  --no-run --message-format json
```

The exact probe SHA-256 is
`09f0d1058e49aa4d06871c04b491b3c60d40f0b66d385c9d0303b754fdbd2f3d`,
with GNU Build ID `4e7a84eff792063a371f96f520756451230e010e`.
ELF inspection found 281 CUDA runtime and 1,364 NVPTX symbol matches.
Build time was 4m52.77s. The full log, source hashes, test-only patch,
toolchain, commands and lease-gated run driver are archived. Both
tracked worktree and lock were clean after the build. The probe ELF
and checksum were retained outside the disposable build target.

This record verifies native CUDA linkage. A separate physical lease is
required to execute the synthetic stack and actual cross-service
controls. The default CPU/wgpu CLI and physical Vulkan result have their
own exact evidence.

[Raw build records](raw-cuda-build-records.tar.gz)
[Archive checksum](raw-cuda-build-records.sha256)
[CPU proof and default release](../README.md)
[Physical Vulkan proof](../physical-vulkan/README.md)
