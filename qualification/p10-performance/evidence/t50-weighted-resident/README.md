# Reuse aligned RGB for slab-free Weighted Average

The exact candidate `a49ce692371558ff40f72b0af77f5249d00dcedb` extends the
existing owner-thread registration RGB handoff to slab-free Weighted
Average. The base is `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`.
The Cargo.lock SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The [native e810 T50 phase trace](../t50-weighted-phase-measured/README.md) attributed 12.961 s to 50 Focus input
upload calls inside a 25.672 s Focus accumulation interval. It preserved
the completed export and 50 registration maps. The inclusive interval
includes more than a single memory copy, and the earlier stage
observation does not predict candidate wall savings.

The job enables reuse only for an aligned original that already matches
the canvas and has an exact identity map in the existing registration
path. The resident handle is immutable, belongs to the same CubeCL
service and owner thread, and is consumed by the active frame. The
private Focus entry validates extent, packed f32 length, stride and
service before inserting the first accumulator state. The old frame slot
still holds one input, and both budget checks remain mandatory. Host
validation order, public input requirements, transform/resize branches,
slab merging, focus kernels, waits and publication boundaries remain
unchanged.

## Native CPU proof

At clean base source plus a test-only observer, the real job reached exact
f32/TIFF/map/progress/backend parity and then failed the zero redundant
upload assertion. It issued nine Focus uploads after already preparing
nine full luma inputs. The translated fixture took the existing warp and
issued five Focus uploads.

The candidate removed all nine second uploads for identity frames.
The translated and mixed-canvas aligned controls kept the one full
Focus upload per warped frame. The unaligned same-canvas and real 3+2
slab controls kept their prior host uploads, including the two slab merge
inputs. The paired native jobs used the same synthetic TIFF originals
with exact f32 and RGB16 output, complete CPU receipts, maps and progress.
Each pair retained actual energy, combine and checked wait counts.

A separate native check generated a wrong-layout resident via registration.
Rejection left first-state uninitialized. Four subsequent valid frame
submissions matched every weight and score f32 bit and the final RGB
result, including exact zero first-frame weights. The uninstrumented
49-test CPU cohort passed, with a permanent aligned cancellation/late
corruption control. Pinned rustfmt, reviewed Miri source inventory and
scoped effective-policy Clippy with warnings denied passed. Runtime
instruments and readback probes remain outside product source.

An alignment-off, mixed-canvas control also took the original checked
resize and host upload. It passed all four instrumented job tests with
the complete fallback roster and retention and output checks.

## Build identities

The default, uninstrumented release CLI at exact `a49ce69` uses only
the normal CPU and wgpu application graph. Neither optional native
CUDA nor HIP is in this build. Its SHA-256 is
`43a05f9cfb6ac3f89c7153e6043cc9e79d33bfccd03054d20da54bd7d147c65e`,
with GNU Build ID `561328fa71c9ff497ce2040ebf619f24e1ec1f7e`. It was
built offline and locked with Rust 1.100.0, two direct jobs pinned to
CPUs 10 and 11, and the workspace core + CLI selections.

The independently linked portable Vulkan probe used the same graph
and a temporary test-only observer. It compiled with `--no-run`; SHA-256
`5059d392c9bfd9bbe3d257896ad1b38148c0e29eb12d89dc84708f6d68ada648`.
This evidence covers CPU execution and both binary builds. Physical
parity, foreign-service rejection on distinct runtimes and authenticated
full T50 timing require their own runs.

[Raw host and build records](raw-host-records.tar.gz)
[Archive checksum](raw-host-records.sha256)

[Physical Vulkan replay](physical-vulkan/README.md)

[Native CUDA probe build](native-cuda-build/README.md)

[Physical native CUDA replay](physical-cuda/README.md)
