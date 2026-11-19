# Linux native CUDA release build at `11cfcf8`

The production CLI compiled and linked from clean `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6`, tree `0ea17aeb65a346e3cb6b5593d8c8b4a02e827832`, with Cargo.lock SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`. This includes the integrated Weighted Average resident RGB path.

The build used Rust 1.100.0, pinned CUDA API 12050 and CUDA 12.5 libraries, offline Cargo, two direct build jobs on logical CPUs 10 and 11, and a previously cached Tracel LLVM 23.1.0-4 archive. The archive SHA-256 matched its publisher checksum. GNU time recorded 4m59.56s elapsed and a maximum resident set size of 2,180,768 KiB. The exact dependency graph and defined ELF symbols confirm CUDA and CUDARC alongside CPU and wgpu. The archived ELF dynamic table records system loader dependencies. CUDA libraries load at runtime and were hashed as replay prerequisites.

```sh
CUDARC_CUDA_VERSION=12050 LD_LIBRARY_PATH=/usr/local/cuda-12.5/lib64 \
  taskset --cpu-list 10,11 cargo +1.100.0 build --locked --offline --release \
  -p pancake -p pancake-cli --bin pancake-cli -j 2 --features cubecl/cuda
```

The immutable ELF64 x86-64 artifact contains 130,313,160 bytes, has GNU Build ID `ed4534bb572228362a31ec4dcf2bc46f66595709`, and SHA-256 `91beeb528af87c12950fa50d236af77ffa2f2c2394d8d10a8bc50938b3862b73`. It remains in the local workspace at `artifacts/pancake-linux-native/11cfcf8/pancake-cli` with its adjacent checksum and build receipt.

This receipt establishes the exact Linux build and link. CPU and physical CUDA execution require a separate replay. No CLI, GPU, HIP or real input was used here.

[Build receipt](receipt.json) · [Raw commands, graph, symbols and host records](raw-build-records.tar.gz)
