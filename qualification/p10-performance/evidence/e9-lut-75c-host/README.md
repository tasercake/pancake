# E9 FFT coefficient reuse on the current CUDA graph

A private native CPU differential reproduced exact registration output for the
shared FFT coefficient table on current Pancake source. The table is produced
during the first existing window dispatch by the same backend that consumes
it. The candidate remains an isolated experiment.

| Identity | Revision |
| --- | --- |
| Current control | `75c3213a8e4c1d91298c4ba954102fd295eaa114` |
| Coefficient-table candidate | `a919045e8004e138df9fcc05650d9e87e50e0e36` |
| Retained native CUDA control executable source | `c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb` |

The control product source under `pancake/src` and `pancake/crates` and
`pancake/Cargo.lock` match the retained CUDA executable's source. Candidate
production code changes only `registration/spectral.rs`; its source bytes
match the [previous compiler-only prototype](../e9-cold-kernel-diagnostic/README.md).
The assurance inventory records its reviewed source SHA. The lock SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Pinned formatting and the Miri production-source inventory check passed.

The native CPU probe covered sides 64 and 128 with identity, signed shift,
and flat ambiguous fixtures. Across all six cases the actual inverse f32
surface, pair estimate and scores matched byte-for-byte. Every case submitted
two window, six FFT, and one cross-phase launch. A direct Rust proof also
matched all 380 f32 angles, including signed zero. The table is 512 or
1,024 bytes, retained through the synchronized, checked readback. The
window producer and consumers stay on the same client and ordered stream.
The largest number of data tensors bound to any dispatch in the spectral
path remains three.

The host ran Rust 1.100.0 with two Cargo jobs pinned to CPUs 12 and 13.
The raw archive includes the temporary test-only patches, source and lock
verification, provenance, command and toolchain records, test outputs,
raw surfaces, and the uninstrumented native CUDA build and link witnesses.

## Native CUDA link

The native release CLI linked from clean candidate source using the existing
locked `cubecl/cuda` profile and CUDA SDK 12.5. The build completed in
2m45s and contained 281 native CUDA and 1,364 NVPTX symbol matches.
The executable SHA-256 is
`95923ea36d753a348352b7eba91a1d14e08322617d16c5239f7b357eb6682671`.
GNU Build ID is `025bae3414e7580a420d7f610a85b0ea3b5a22ee`.
The retained `c1e2cac` control executable SHA-256 is
`8bc02636ff0439d23a74b40ddc56c3b7b84a21f7147a7b2ae7b30dd9c77efd57`.
[LUT executable build and CPU records](raw-host-records.tar.gz)

This host preparation used no physical GPU, real E9 input, or performance
samples. A separate paired physical run can compare the two linked
executables with fresh processes and independent private cache roots.

## Compiled-code persistence audit

Pinned CubeCL 0.11.4 can store compiled CUDA PTX across processes. With
`cubecl/persistence` and `compilation.cache` enabled, CUDA
`CudaContext::try_load_cached` attempts a disk hit before expanding the
kernel definition. The persistent entry still reaches the driver's module
loader. This capability is distinct from autotune caching. The tested
Pancake graph leaves compiled-code persistence disabled.

The cache namespace includes CubeCL version, device target, compiler,
and PTX version. Its first-line key combines stable kernel identity and the
application build ID. LLVM also links external toolkit `libdevice.10.bc`;
that runtime-selected file lacks an explicit identity in the first-line
key. A module-load error from a cached entry propagates without recompiling.
CubeCL's first configuration load can also reassign the cache root after
Pancake's environment setup. The state lives behind the CubeCL persistence
database, which is present in the lock but outside the tested execution
graph.

A production cache change needs a private application-owned root, exact
toolchain and compile-input invalidation, and recompilation on an invalid
entry. The isolated coefficient-table experiment changes neither the
cache configuration nor the dependency graph.
