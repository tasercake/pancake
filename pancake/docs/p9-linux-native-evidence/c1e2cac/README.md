# P9 Linux native release profiles at `c1e2cac`

The Linux x86-64 CUDA and HIP production CLI profiles compiled and linked on
clean revision `c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb`. The Git tree was
`a10967050f9579d7e82f8b6b4287be6997d1f885` and the Cargo.lock SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The reviewed production source inventory, both dependency policies, offline
advisory, ban and source checks, and all five vendor-source tests passed.

| Production profile | Verification | Binary SHA-256 |
| --- | --- | --- |
| `cubecl/cuda` | Release compile and link, two native CPU and two physical CUDA frozen cases | `8bc02636ff0439d23a74b40ddc56c3b7b84a21f7147a7b2ae7b30dd9c77efd57` |
| `cubecl/hip` | Release compile and link, ELF and defined native runtime symbols | `35f465351906a222f5eef81097d131d1b27d6041e2bbc3bd47d2e381c4289bf5` |

Rust 1.100.0 built both profiles in sequence with one owned release target,
two jobs pinned to separate Ryzen 9 7950X3D cores (logical CPUs 12 and 13),
and a registry-only build cache. Each dependency graph excludes the other
native optional runtime and retains the production CPU and wgpu paths.
The CUDA executable was preserved before the HIP profile built. CUDA took
6m 57s, HIP took 3m 27s, and the shared target held 2.777 GB after both builds.
The highest child RSS reported by GNU time was about 1.17 million KiB.

```sh
CUDARC_CUDA_VERSION=12050 CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 12,13 cargo build --locked --offline --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/cuda
env -u CUDARC_CUDA_VERSION CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 12,13 cargo build --locked --offline --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/hip
```

| Frozen case | Verified completion |
| --- | --- |

Both smoke lanes passed through the same hashed CUDA release executable with
matching source and suite provenance. The actual NVIDIA CUDA jobs used the
GeForce RTX 3090 at PCI `0000:01:00.0`, NVIDIA driver `580.178.04`, CUDA 12.5.40
NVRTC and `ptx_sm86`. Every job completed with zero fallback and produced a
nonempty export. Their four SHA-256 values match the
[prior native replay](../8694b55/README.md). HIP qualification covers the
build and link. This replay did not run AMD, Intel, or Windows hardware lanes.
