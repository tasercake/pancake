# Integrated E9 on native CUDA and OpenCL

P6’s retained native CUDA release CLI used the exact integrated product
`12f2151c8a198b5209c8f4d506dbd17fd9aaa74f`, Cargo lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`, and executable
SHA-256 `45cb96ed124a3c8145e3c7cbbf836a3334580d14d63fde59596b393225910ec4`.
The producer built it with locked default features plus `cubecl/cuda` and
`CUDARC_CUDA_VERSION=12050`. The archive retains the original build and
device receipts. The CUDA runtime library hashes matched before and after
the session. The candidate used CUDA 12.5’s library directory while the
reference and its hardware preflight used their existing OpenCL loader
environment. The stage receipt records the measurement runner hash and patch
separately from immutable Pancake source.

Each application completed two warmups, ten warm, and five cold jobs. The
collector launched all 34 primary jobs in alternating order with fresh
processes. Two separate diagnostics confirmed actual CUDA and OpenCL on the
same physical device. Each diagnostic’s decoded hash matched all ten warm
jobs for that application.

| --- | ---: | ---: | ---: | --- |
| Warm median, ≤0.50 | 0.455621 s | 0.808702 s | 0.56340 | Fail |
| Cold median, ≤1.00 | 0.609114 s | 7.215529 s | 0.08442 | Pass |
| Process-tree RSS, ≤0.50 | 233,988,096 B | 550,260,736 B | 0.42523 | Pass |
| Attributable GPU memory, ≤0.50 | Pending | 320,864,256 B | Unavailable | Pending |

Every RSS trace met the 20 ms sampling limit. Worst candidate and reference
RSS gaps were 9.690 and 9.569 ms. All 17 candidate GPU traces exceeded 20 ms.
The worst gap was 34.554 ms from the last valid GPU observation to process
exit in warm job 7. A 28.849 ms NVML acquisition completed after that exit.
All 17 reference GPU traces qualified, with a maximum gap of 16.592 ms.
The observed CUDA lower bound of 295,698,432 bytes already exceeds half the
qualified reference peak of 320,864,256 bytes. The strict candidate GPU
peak and ratio remain pending under the frozen protocol.

Every export independently decoded as genuine 337 × 510 RGB16, with one
stable decoded hash per application. The first-warm matched pair had a
full-image MAE of 2,825.185 and a maximum error of 19,150 on the 65,535-code
scale. Three independent corresponding 48 × 48 regions yielded MAE of
3,706.53, 4,118.10, and 3,451.18. Equivalent output quality remains open.
The temporary image pair and all scoped caches were removed after review.

The same-source default release measurements cover CPU and Vulkan/OpenCL.
The native CUDA receipt identifies its distinct executable SHA-256.
[Default release measurements](../integrated-e9-12f2151/README.md)

After the final consumer released the E9 data, cleanup rehashed and removed
the source archive and nine original PNGs, totaling 71,495,015 bytes.
[Input cleanup](input-cleanup.json)
