# Hybrid band parity on native CUDA

The original `b820da05d3982a7912cb7c7c9d8aa926ece14a9c` and candidate
`903836ca831ba1e92b7d4f006836320dd8cbe402` produced byte-identical raw f32
Hybrid output on the same RTX 3090. That covers 235,962 channels across
18 fixtures, with zero differing bits and zero maximum absolute error.
Both files were 943,848 bytes and had SHA-256
`19d204268113dcb8178898d06aa523f6a9fae38233b180539a74bec252e38422`.
This qualifies the targeted CUDA parity check. The comparison collected
no performance or peak-memory measurements.

Both tested libraries used Rust 1.100.0 and the same checked Cargo lock
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
They were built from clean sequential checkouts in one owned target, with
two jobs pinned to CPUs 12–13 and the native `cubecl/cuda` feature. Each
ELF contains actual CubeCL native CUDA and LLVM NVPTX symbols. The feature
trees are identical, including the same portable GPU and CPU dependencies.
There was no HIP execution. The Rust fixture driver source matches the
[original Vulkan comparator](../compare-native-f32.rs.txt) byte-for-byte.

| Source | Executable SHA-256 |
| --- | --- |
| `b820da05` | `01600352e259d7d682ff805ca05d6cc722c8142d74b15264d0f12e44c922139b` |
| `903836ca` | `ff22412a7bdac5e9fb806b9040955adcf732a585deddebbf6a6047b7f83e4ffa` |

The fixture driver explicitly requested `RuntimeOverride::Cuda` and visible
ordinal zero. Each call verified a physical device and rejected any fallback.
Both reported compute fingerprint `ptx_sm86` on the GeForce RTX 3090 at PCI
`0000:01:00.0`, UUID `GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, and driver
`580.178.04`. The CUDA 12.5.40 toolkit, libdevice and actual driver-library
hashes were recorded. Capture ran 2026-11-18 20:28:17–20:28:19 SGT and
closed both comparator processes before handoff.

The same public accumulator processed deterministic chromatic data,
equal positive contrast and zero contrast at six size/depth pairs with odd
extents and single-pixel axes. Every output was checked for its shape and
finite channels. The native CUDA output hashes also match the retained
CPU differential. The prior Vulkan differential compares each Vulkan arm
against its own reference and remains distinct.
