# E9 CUDA phase attribution

Seven paired launches of the historical native CUDA CLI and a phase-instrumented
build completed on the RTX 3090. Each executable ran two warmups and five
fresh-process observations. All nine original Balloons exposures matched their
frozen input hashes before and after the jobs. Every export was a genuine
337 × 510 RGB16 TIFF, matched the historical encoded and decoded SHA-256 values,
and recorded nine identity transforms. The jobs reported physical native CUDA,
PCI `0000:01:00.0`, fingerprint `ptx_sm86`, and no fallback.

The trace build was `4cd35b981dc664bc8a6c2d1809a31ad5a4b7ee4b`, based on
`2143eda2b308497caee96334357dd23d324e1e8e`. Its executable SHA-256 was
`c3c88993cf2015cdeb7b97a3ad6d8f2bf5b5bcfdf9f0d63dca2eb46a9448981d` and its
Cargo.lock SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The control executable was the retained linked CUDA build from the
[previous campaign](../integrated-e9-cuda-12f2151/README.md). The diagnostic
build command used Rust 1.100.0 and the existing CUDA feature.

```sh
CUDARC_CUDA_VERSION=12050 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 \
  taskset --cpu-list 20,21 cargo +1.100.0 build --offline --locked \
  --release -p pancake -p pancake-cli --bin pancake-cli --features cubecl/cuda
```

| Diagnostic interval | Median |
| --- | ---: |
| Native control, full process | 464.342 ms |
| Trace build, full process | 464.357 ms |
| Runtime setup | 116.574 ms |
| Registration | 188.178 ms |
| Coarse seed, inside registration | 114.082 ms |
| Fusion accumulation | 50.732 ms |
| TIFF encode | 13.021 ms |

The first moving frame took 99.765–104.226 ms for coarse seeding. Each of the
other seven took 1.708–1.954 ms. The median total of the score readbacks inside
the seed was 9.148 ms. Both compiler cache directories had zero CubeCL files,
while NVIDIA's scoped driver cache held 15 files per variant. The affinity,
frozen arguments, driver, event receipts and thermal readings accompany
each run. A separate CPU diagnostic remained stopped throughout the paired
launches and accumulated no CPU time.

The original runner polled process exit with Python's timed wait and a 50 ms
polling cap. Its two full-process medians lack precision for a latency
comparison. The internal stage timestamps and the separate CUDA API trace
were captured independently of that wait.

One separate Nsight Systems 2024.2 CUDA API and GPU trace reused the warm
driver cache and checked the same output and transforms. The first `fft_lines`
module load began 92.587 ms after the first `window_luma` launch call, and the
module load took 0.350 ms. The first FFT ran for 0.281 ms on the device. All 48
FFT dispatches in this profiled run took 12.257 ms of device time in total.
This locates most first-use cost before module load. The CUDA API trace leaves
the individual host lowering and compilation costs unresolved.
