# P0 shared-kernel qualification

## Numerical contract and safe launch

`blur_horizontal_u16` applies horizontal `[1, 2, 1] / 4` filtering to a row-major
grayscale plane. Borders repeat the closest pixel; integer division rounds down.
Input and output remain u16. Device storage and accumulation use u32: the maximum
sum is 262140, so every u16 input bit is retained without requiring optional
16-bit storage extensions. This storage choice costs twice the source bytes.

Dimensions must be nonzero, their product must fit u32 indexing, and the input
length must match. Readback validates byte length and rejects values above 65535.
These failures return errors. Safe `TensorHandle` construction and checked
`#[cube(launch)]` dispatch require no first-party unsafe Rust.

Three native tests check independent hand-calculated 5x3 results, boundary
clamping, 0/32768/65535 precision, width-one images, a 257x3 workgroup tail and
invalid dimensions. Three pure host tests check shape bounds, malformed readback,
out-of-range samples and native error propagation. CPU and GPU launch exactly the
same authored kernel; there is no second CPU algorithm.

## Actual execution

Local native integration passed on Apple M4 Pro CPU (LLVM 23.1.0 JIT) and physical
Apple M4 Pro integrated GPU through wgpu Metal. The GPU test rejects a CPU adapter
and writes `target/p0-kernel-adapter.txt` with adapter/backend/CPU identity. It does
not silently skip a missing GPU.

Parent integration also ran an earlier kernel snapshot on Linux x86_64 Ryzen 9
7950X3D CPU and RTX 3090 Vulkan. That evidence must be rerun against the final
commit before it qualifies this exact revision. Native CUDA execution was also
reported by parent against that earlier snapshot. These results do not qualify
AMD, Intel, Windows, native Metal or full image-pipeline parity/performance.

## Assurance boundary

The initial full integration Miri run failed at unsupported foreign function
`mach_host_self` in `sysinfo`, reached from CubeCL CPU runtime initialization.
Miri cannot interpret LLVM JIT or GPU drivers. The nightly gate therefore runs
all pure host library tests under strict provenance and symbolic alignment.
Native integration remains mandatory in debug/release tests, coverage, randomized
layout tests and Linux sanitizer lanes. No tests are ignored and no first-party
source files are excluded from coverage.

CubeCL's function macro originally retained a nonexecuted Rust Tensor DSL stub
alongside its IR expansion. The stub cannot serve as a host implementation;
frontend topology/tensor primitives are not host buffers. Its uncovered regions
initially failed strict coverage. The pinned three-line upstream macro patch
honors the existing `expand_only` flag for functions, omitting only this stub.
Compiler contracts first failed with the original symbol present, then passed
after the patch: original symbol absent, safe checked launcher present, default
unflagged behavior preserved. The authored device arithmetic is checked through
native numerical tests and mutation tests; LLVM coverage measures host Rust, not
executed GPU instructions.

After removing the stub and testing host failure contracts, strict host coverage
passed: 62/62 lines, 7/7 functions, 66/66 regions, and 8/8 branches. No coverage
threshold changed. All 35 mutations were caught. The initial equivalent mutation
changed an explicit tail guard from `<` to `<=`; checked launch already bounds
memory operations. Removing the redundant guard eliminated that mutation without
adding an ignore rule or disabling memory checks.

## Rowan layout boundary

The complete first nightly run exposed a separate seed-42 compiler dependency
failure: Rowan 0.17.0 asserted a 16-byte `GreenChild` enum while leaving its Rust
layout unspecified. One pinned upstream representation annotation, `repr(u32)`,
makes the tag/offset/pointer layout explicit and preserves the 16-byte invariant.
Four upstream tests passed with both layout seeds, including an added node/token
clone, drop, relative-offset and payload-access regression. No ARC implementation
changed. [Rust's representation rules](https://doc.rust-lang.org/reference/type-layout.html#primitive-representation-of-enums-with-fields)
define this enum layout.

An additional upstream Miri check failed at `arc.rs:264` in both the original
published archive and the patched crate. The same `HeaderSlice` unsizing retag
failure occurs before the enum-layout operation. The original ARC source SHA-256
is identical in both copies. This is a separately recorded upstream limitation,
not a passing Miri result. Related upstream reports remain open:
[Rowan #108](https://github.com/rust-analyzer/rowan/issues/108) and
[Rowan #192](https://github.com/rust-analyzer/rowan/issues/192).
Rowan enters this graph through Slint's compiler; required application host Miri
tests do not invoke it. This work does not certify all transitive unsafe code.
Complete [original](p0-kernel-evidence/rowan-original-miri.log) and
[patched](p0-kernel-evidence/rowan-patched-miri.log) traces are retained.

## Reproduction

```sh
cargo test --locked --lib --test kernel_contract
python3 -m unittest discover -s vendor/tests -v
bash scripts/check.sh all
```

The pinned LLVM bundler selects LLVM 23.1.0-4, verifies archive and extracted
content checksums, and installs its per-user bundle when absent. It does not use
the older system LLVM. Slint accessibility, winit, software rendering and wgpu
rendering features remain enabled. The lockfile unifies compatible
`hi_sparse_bitset` versions at 0.9.0 and removes `windows-sys` 0.59.0; incompatible
0.52/0.61 generations retain exact reviewed exceptions.

## Verification results and limits

`bash scripts/check.sh all` passed after the layout fix: strict lints/docs,
6 debug tests, 6 release tests, feature tests, complete host coverage, dependency
policy, 35 caught mutations, 3 strict host Miri tests, 6 native tests under each
layout seed, complete branch coverage, and all rejecting/accepting guardrail
fixtures. The added permanent vendor checks subsequently passed through
`scripts/check.sh deps`: 18 policy self-tests plus 7 source/macro contracts.

Final-revision remote hardware reruns and Linux sanitizer qualification remain
parent integration work. AMD, Intel and Windows hardware are not qualified.
Small numerical qualification is not an end-to-end speed or memory target result.
