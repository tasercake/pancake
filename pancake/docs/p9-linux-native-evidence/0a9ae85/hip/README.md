# Linux HIP release linkage at `0a9ae85`

The production headless CLI compiled and linked from clean revision
`0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` with the committed Cargo lock
SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The offline build used Rust 1.100.0 and two direct Cargo jobs pinned to
logical CPUs 10 and 11 on the Ryzen 9 7950X3D. It took 4m53.90s and
reported a maximum child RSS of 2,180,024 KiB.

```sh
cd pancake
env -u CUDARC_CUDA_VERSION -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
  CARGO_INCREMENTAL=0 taskset -c 10,11 cargo +1.100.0 build \
  --locked --offline --release -p pancake -p pancake-cli --bin pancake-cli \
  -j 2 --features cubecl/hip
```

The 164,655,352-byte ELF has GNU Build ID
`309a2b2e72a2de622d6c1efd2a77a107d12eda90` and SHA-256
`8859fed3fd7772aaf98c7cf880b089f7fdd9eb801dca8c4ed0171e93eb13995b`.
It defines 270 HIP runtime and 78 HIP binding symbol matches. The
resolved CLI graph includes the shared CPU and wgpu backends, one HIP
profile, and no CUDA runtime or Slint frontend. The pinned
`cubecl-hip-sys` 7.14.6085001 build selected the single dynamic binding
ABI `hip_60850`, following the [earlier build recipe](../../c1e2cac/README.md).
The binding loads `amdhip64` and `hiprtc` at runtime. This local check
covers compilation and link inspection. It made no application or
device calls.

The initial graph-inspection command selected only the CLI package,
which cannot enable its core dependency's CubeCL feature. The corrected
inspection selected the same two package roots as the successful build
and checked the graph in both directions. The compiler build log and
receipt, both graph attempts, binding build-script output, ELF metadata
and linkage symbols are retained. The 2.05 GB disposable build target
was removed after verifying the retained ELF and archive.

[Machine-readable receipt](receipt.json)
[Raw build and link records](raw-build-records.tar.gz)
[Archive checksum](raw-build-records.sha256)
