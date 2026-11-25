# P9 Linux native release profiles at `8694b55`

The Linux x86-64 CUDA and HIP production CLI profiles compiled and linked on
clean revision `8694b555666a1ddf4051e2839bbced4bcc625859`. The Git tree was
`8956e0b0e2d2b715960fedd17b394b300e681f91` and the Cargo.lock SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The production source inventory, both dependency policies, offline advisory,
ban and source checks, and all five vendor-source tests passed.

| Production profile | Verification | Binary SHA-256 |
| --- | --- | --- |
| `cubecl/cuda` | Release compile and link; two native CPU and two physical CUDA frozen cases | `9c79b53cb058951eb143d413e01646b9561451def3dd68451306e908aa33b745` |
| `cubecl/hip` | Release compile and link; ELF and native runtime symbols | `f793766b1e59bf89d4502dd9f2ed0678053a87845048028fbd29ecff16ced394` |

The builds used Rust 1.100.0, a registry-only cache, and two Cargo jobs on
logical CPUs 14 and 15, which are separate Ryzen 9 7950X3D cores. One owned
target served both profiles in sequence; the CUDA executable was preserved
before HIP built. Each feature graph excludes the other native backend.
Both profiles share the retained CPU and wgpu paths. CUDA linked in
4m 36s and HIP in 2m 55s. The highest child RSS reported by GNU time
was about 1.17 million KiB for each profile. The shared target held
2.776 GB after both builds.

```sh
CUDARC_CUDA_VERSION=12050 CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 14,15 cargo build --locked --offline --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/cuda
env -u CUDARC_CUDA_VERSION CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 14,15 cargo build --locked --offline --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/hip
```

| Frozen case | Verified completion |
| --- | --- |

The release CLI passed both frozen smoke lanes with matching suite and source
provenance. CUDA ran on the GeForce RTX 3090 at PCI `0000:01:00.0`, NVIDIA
driver `580.178.04`, CUDA 12.5.40 NVRTC, and `ptx_sm86`. Each case recorded a
completed stack with zero fallback and a nonempty export. All four files
matched the export SHA-256 values from the [prior replay](../287828a/README.md).
HIP qualification covers build and link. AMD and Intel hardware and Windows
execution are outside this replay's scope.
