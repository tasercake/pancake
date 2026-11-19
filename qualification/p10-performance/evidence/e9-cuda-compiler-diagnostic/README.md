# E9 CUDA compiler comparison

The CUDA C++ build took 737.959 ms warm median on real E9, compared with
452.260 ms for the same-source LLVM build. The C++ profile was 1.632 times
slower. Both profiles produced the same encoded TIFFs and decoded RGB16
pixels. The C++ profile stays on the experimental branch.

Both release executables came from the clean source and suite revision
`2143eda2b308497caee96334357dd23d324e1e8e` with Cargo.lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The locked feature trees verify that `cubecl/cuda-cpp` enabled CubeCL's C++
compiler on top of `cubecl/cuda`. The control binary matched the prior
`d6359e1` native CUDA release byte for byte.

| Profile | Executable SHA-256 | Warm median |
| --- | --- | ---: |
| LLVM | `830e09230773dc53050d40ce4b665581665f2964dd14c6ca1cab4646262e8a35` | 452.260 ms |
| C++ | `74d3cbd55539ca8b4dd4adb1d4a7b740f657fafca5259bde2c5391aac4b788cc` | 737.959 ms |

Both builds used Rust 1.100.0, two build jobs on logical CPUs 20 and 21, and
`CUDARC_CUDA_VERSION=12050`. From the `pancake` directory, build the C++ profile
with the following command. Use `--features cubecl/cuda` for the LLVM control.

```sh
CUDARC_CUDA_VERSION=12050 CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 \
  taskset --cpu-list 20,21 cargo +1.100.0 build --offline --locked --release \
  -p pancake -p pancake-cli --bin pancake-cli --features cubecl/cuda,cubecl/cuda-cpp
```

The C++ executable passed five frozen CPU and image-I/O cases spanning corrupt
input, failed export retention, grayscale NumPy, weighted averaging, and exposure
fusion. It passed both frozen native CUDA cases for weighted empty regions
and nine-exposure fusion. The subsequent benchmark used nine authentic Balloons
PNGs, frozen E9 options, separate app and driver caches per profile, and
alternating fresh-process launches. The timer used blocking Linux `wait4` with
a separate campaign deadline. All 14 app launches completed, two warmups and
five measured launches per profile. Both profiles ran on the RTX 3090 at PCI
`0000:01:00.0` with NVIDIA driver `580.178.04`, without fallback. The card
recorded 68°C and P2 before and after. The other active build recorded at
preflight was suspended throughout the run and accumulated zero CPU time.

Both profiles emitted nine identity transforms. Each TIFF decoded as genuine
337 × 510 RGB16. The largest absolute pixel difference was zero RGB16 codes
across all seven pairs. The original photos and source archive were rehashed
and 71,500,200 bytes of owned input files were removed. Generated build products
were removed after recording both executables' identities.
