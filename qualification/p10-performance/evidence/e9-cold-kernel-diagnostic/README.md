# E9 first-use spectral compiler experiment

Moving native sine and cosine from the FFT into the existing window dispatch
reduced the host-only first-use cost of the affected spectral modules by
7.77 ms at side 64 and 10.24 ms at side 128. The side-128 FFT itself saved
24.70 ms, while the window cost rose by 14.10 ms. The six native CPU test
fixtures matched exact inverse-surface bits, coarse seeds, candidate scores and
launch counts. Keep the prototype experimental. The measured savings could
close about one fifth of the [previous frozen warm gap](../integrated-e9-cuda-12f2151/README.md)
before the cost of GPU table lookups. This experiment used no GPU, real E9
launches, or change to the frozen protocol.

## First-use host compilation

| Side and source | Window | FFT | Cross phase | Whole set |
| --- | ---: | ---: | ---: | ---: |
| 64 unchanged | 39.525 ms | 51.465 ms | 3.112 ms | 94.656 ms |
| 64 table | 55.381 ms | 26.992 ms | 3.418 ms | 86.888 ms |
| 128 unchanged | 39.557 ms | 51.576 ms | 3.135 ms | 94.175 ms |
| 128 table | 53.660 ms | 26.881 ms | 3.206 ms | 83.935 ms |

The whole set is the median total per process. Per-module columns show the
sum of median expansion and LLVM emission. Each profile and side ran in five
fresh host processes pinned to logical CPU 0. The probe used Rust 1.100.0,
locked CubeCL 0.11.4 and CUDA Toolkit 12.5 libdevice. It builds checked
kernel definitions from the actual registration kernels and invokes CubeCL's
public CUDA compiler. No launcher initializes a device in the compile probe.
The Cargo graph and lock match `287828aac07ffad2252785b73a52a59fc07b3409`.
The two diagnostic-only `rustc --extern` references point to the exact LLVM
and runtime libraries already compiled transitively in that graph.

The [archived physical Nsight trace](../e9-cuda-phase-attribution/README.md)
recorded compute capability 8.6, driver API 13000 (PTX 9.0), 32×8 window/cross
blocks and 32×4 side-128 FFT blocks. The side-64 FFT uses 32×2. Grid-constant
argument layout follows the pinned CUDA runtime. Axis and direction are
runtime scalar arguments and share one compiled FFT per block shape. The
original 92.587 ms interval lies between window launch and FFT module-load
start. Our separate harness isolates expansion and LLVM emission. Neither
interval by itself is a whole-application latency measurement.

## Exact CPU behavior

The first window launch writes one 512- or 1024-byte table on the same
client. Its forward and inverse terms use the existing backend sine/cosine.
The final-stage angle grid contains every earlier angle as a dyadic subset.
An exhaustive Rust check matched all 380 signed f32 angles bit-for-bit,
including signed zero. The FFT keeps the existing bit reversal, butterfly
operation order, per-line arrays and row and column passes. Subsequent
kernels read the table only after its producer in the existing ordered stream.

The real native CPU spectral and score stage ran identity, shifted and flat
ambiguous inputs at both sizes. Each fixture kept two window, six FFT and one
cross-phase submissions and completed checked readback. The direct native
inverse-surface files and coarse/score records match byte-for-byte. The
prototype at `551ca6b` is recoverable from the isolated experimental branch.
No renderer, kernel policy, or shipping dependency changed.

## Alternatives examined

Pinned `cubecl-llvm` runs `default<O3>` in `nvptx/codegen.rs`, then emits with
`LLVMCodeGenLevelAggressive` in `shared/llvm_module.rs`. The supported compiler
options control target, PTX version and argument layout. There is no public
per-kernel optimization-level setting. The internal LLVM option bridge writes
global registered options and cannot provide a per-kernel pass pipeline.
The retained profile lacks compiled-kernel persistence. Enabling it would add
an unreviewed database dependency and would leave first-cold compilation.
Prior physical tests already rejected both the [twiddle loop interchange](../e9-fft-twiddle-diagnostic/README.md)
and the [alternate CUDA compiler](../e9-cuda-compiler-diagnostic/README.md).

The [summary](summary.json) binds source, binaries, toolchain, all samples,
CPU result hashes, and the [raw archive](raw-records.tar.gz). The archive
holds the test-first probes, experimental patch, LLVM and PTX output,
commands, parity records and cleanup.
