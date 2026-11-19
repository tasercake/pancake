# P9 Linux native release profiles

The Linux x86-64 CUDA and HIP production CLI profiles compiled and linked on
clean revision `d6359e129737f1a9077fb4abc2a8f783fa1f7b38`. The Git tree was
`e7412552ba9be952fa8972bccce510af13680b3c` and the Cargo.lock SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Both dependency graphs passed the repository policy and offline cargo-deny
advisory, ban, and source checks. All five vendor-source tests passed.

| Production profile | Scope |
| --- | --- |
| `cubecl/cuda` | Release compile and link, plus four frozen cases against the resulting executable |
| `cubecl/hip` | Release compile and link with ELF and defined runtime symbol inspection |

The build used Rust 1.100.0, a registry-only build cache, and two jobs on
logical CPUs 2 and 3, which map to separate Ryzen 9 7950X3D physical cores.
The following commands use the locked graphs from the `pancake` directory.

```sh
CUDARC_CUDA_VERSION=12050 CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 2,3 cargo build --locked --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/cuda
env -u CUDARC_CUDA_VERSION CARGO_TARGET_DIR=target/native-release \
  taskset --cpu-list 2,3 cargo build --locked --release -p pancake -p pancake-cli \
  --bin pancake-cli -j 2 --features cubecl/hip
```

| Frozen case | Verified completion |
| --- | --- |

Both lanes passed with matching source and suite provenance. The CUDA CLI
SHA-256 was `830e09230773dc53050d40ce4b665581665f2964dd14c6ca1cab4646262e8a35`.
CUDA ran on the GeForce RTX 3090 at PCI `0000:01:00.0`, with NVIDIA driver
`580.178.04`, CUDA 12.5.40 NVRTC, and `ptx_sm86`. Each case emitted a completed
stack with zero fallbacks and published a nonempty export. All four exports
matched the SHA-256 values from the [previous native replay](../12f2151/README.md).
The initial CPU case command stopped at executable staging. Creating the
descriptor's expected release directory resolved the setup error before any
cases ran.
