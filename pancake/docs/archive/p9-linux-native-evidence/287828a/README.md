# P9 Linux native release profiles at `287828a`

The Linux x86-64 CUDA and HIP production CLI profiles compiled and linked on
clean revision `287828aac07ffad2252785b73a52a59fc07b3409`. The Git tree was
`6f704c3b117767700f81e905f9f9d5ba90795b2b` and the Cargo.lock SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Both dependency graphs passed the repository policy and offline cargo-deny
advisory, ban, and source checks. All five vendor-source tests passed.

| Production profile | Verified scope | Binary SHA-256 |
| --- | --- | --- |
| `cubecl/cuda` | Release compile and link; two CPU and two physical CUDA frozen cases | `cb5237e030f0ccbca2328a16c79e27ae30dc40606d5f45c0882598bfb9909cc1` |
| `cubecl/hip` | Release compile and link; ELF and native runtime symbols | `a785dbc5c485e20812b8774c83f187a09a523a960a24eac7b873050b7bde9020` |

The build used Rust 1.100.0, a registry-only build cache, and two Cargo jobs
on logical CPUs 6 and 7, on separate Ryzen 9 7950X3D physical cores. The two
profiles shared one owned target directory and ran sequentially. CUDA built
in 5m 36s and HIP in 2m 59s. GNU time reported a maximum child RSS of about
1.17 million KiB for each profile. The target held 2.78 GB after both builds
and was cleaned after the audit.

```sh
CUDARC_CUDA_VERSION=12050 CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 6,7 cargo build --locked --offline --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/cuda
env -u CUDARC_CUDA_VERSION CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 6,7 cargo build --locked --offline --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/hip
```

| Frozen case | Verified completion |
| --- | --- |

Both replay lanes passed with matching source and suite provenance. GPU cases
used the GeForce RTX 3090 at PCI `0000:01:00.0`, NVIDIA driver `580.178.04`,
CUDA 12.5.40 NVRTC, and `ptx_sm86`. Each case emitted a completed stack with
zero fallbacks and a nonempty export. All four exports matched the exact
SHA-256 values from the [prior native replay](../d6359e1/README.md).
HIP is retained as a build and link check. AMD and Intel hardware and Windows
runtime execution were outside this replay's scope.
