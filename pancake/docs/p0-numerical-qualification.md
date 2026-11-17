# Native variance, budget and reuse qualification

The new independent `numerical_cases` oracle passes on **native Apple Metal**
and **native NVIDIA CUDA**. The retained Linux HIP profile also compiles and
links without a ROCm installation; HIP execution was not attempted.
The same new oracle also passes on physical **NVIDIA Vulkan through wgpu**.

| Native profile | Actual result |
| --- | --- |
| Metal: Apple M4 Pro, `msl_Apple M4 Pro` | All shared variance/budget/reuse checks pass. Live usage stays 0 allocations / 0 bytes after warm-up and eight repeated completed rounds. |
| CUDA: RTX 3090, `ptx_sm86`, driver 580.178.04, NVRTC 12.5 | Same unchanged oracle passes. Live usage stays 8 allocations / 128 bytes after warm-up and eight repeated completed rounds. |
| wgpu Vulkan: RTX 3090, `DiscreteGpu`, driver 580.178.04 | New variance/reuse oracle and separate core GPU blur both pass. Live usage stays 8 allocations / 512 bytes; reserved bytes stay 13,778,496. Adapter receipt explicitly confirms Vulkan and rejects software GPU fallback. |
| Linux HIP compile/link | `cargo test --locked --features cubecl/hip --test cpu_numerical_contract --no-run -j2` exits 0 and produces a linked x86-64 ELF. No missing dynamic dependencies. No `hipconfig`, `/opt/rocm`, `HIP_PATH` or `ROCM_PATH`; no SDK installation or AMD hardware claim. |

The oracle checks centered variance `[8,0,2]`, bright low-bit variance `1.25`,
width-one and odd-width flat rows, shape-error propagation, exact admission
budgets, and a 1025×257 constant plane. Both blur and variance complete before
memory readings. This executes the new reduction; historical blur-only results
are not reused as evidence for it.

The request guard is **8 MiB of logical working data**. Both native allocators
retain **12 MiB reserved** before and after reuse. The guard is not a physical
GPU/driver reservation cap, process RSS limit or half-memory performance result.
The Vulkan allocator likewise retains 13,778,496 bytes; its live and reserved
usage are unchanged after the repeated completed jobs.

## Immutable source and exact profiles

Source archive: committed `cc316513d1d3855e58ec49843637f1f0d08876d3`, containing
the cherry-picks of parent `572e601` and `65185d9`. Application and shared oracle
sources were not edited for these profiles. Local and remote hashes match:

```text
src/lib.rs: 95a29677a5628ca417a73315f3b59e7692db8c775fd6650d945a98047c4c4b51
tests/common/numerical_cases.rs: a50430ef9f27f034102e6b38c3ca05be2cd3c48f1deb893cb4285d031d1ca379
Cargo.lock: 7c7c64dcbf4255efec0705b472765dd001b733999b122f67c78f36f79349ba7b
```

Each retained native harness calls those exact shared contracts and selects the
native runtime explicitly. Copy its source to
`pancake/tests/native_numerical_contract.rs` in a dedicated committed-source
archive, then run from that archive's `pancake/` directory:

```sh
cargo +1.100.0 test --locked --features cubecl/metal-native --test native_numerical_contract -j2 -- --test-threads=1
CUDARC_CUDA_VERSION=12050 LD_LIBRARY_PATH=/opt/pancake-cuda/lib64 cargo +1.100.0 test --locked --features cubecl/cuda --test native_numerical_contract -j2 -- --test-threads=1
cargo +1.100.0 test --locked --features cubecl/hip --test cpu_numerical_contract --no-run -j2
cargo +1.100.0 test --locked --test gpu_numerical_contract -j2 -- --test-threads=1
cargo +1.100.0 test --locked --test kernel_contract wgpu_blur_matches_independent_values -j2 -- --exact --test-threads=1
```

Linux used the existing task container, Rust 1.100.0, `CARGO_HOME=/opt/pancake-cargo`,
`RUSTUP_HOME=/opt/pancake-rustup`, its cargo bin directory on PATH,
`RUSTFLAGS=-Dwarnings`, and the existing normal build cache
`CARGO_TARGET_DIR=/work/pancake/target`. Staged source was separate at
`/work/p0-numerics/pancake`; `/work/pancake` and `/work/p0-ui` sources were not
overwritten. Container lifecycle remained with its owner and was left unchanged.

Each command had a 420-second timeout, 1-MiB retained-log cap and a live 15-GiB
disk-reserve guard. No limit fired or log bytes were discarded. The minimum
remote free space observed during HIP compilation was 18,397,663,232 bytes,
above the 16,106,127,360-byte reserve. Elapsed command times include compilation
and are not performance measurements.
