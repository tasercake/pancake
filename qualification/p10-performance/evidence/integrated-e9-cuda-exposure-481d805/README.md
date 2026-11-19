# Scoped Exposure candidate E9 native CUDA replay

| --- | ---: | ---: | ---: | ---: | --- |
| Warm median completed job | 0.392271052 s | 0.793581813 s | 0.494304488 | 0.50 | Pass |
| Cold median completed job | 0.535177828 s | 7.194507244 s | 0.074387002 | 1.00 | Pass |
| Peak process-tree RSS | 224,755,712 B | 547,700,736 B | 0.410362260 | 0.50 | Pass |
| Peak attributable device memory | Pending | Pending | Unavailable | 0.50 | Pending |

All process-tree RSS observations met the 20 ms limit. The largest primary sample gaps were `9.608611 ms` on CUDA and `9.173424 ms` on OpenCL. Strict device-memory observations encountered gaps above 20 ms in all 17 CUDA jobs and four of 17 OpenCL jobs. Maximum device-memory gaps were `36.643832 ms` and `31.227925 ms`, respectively. The observed attributable lower bounds were `291,504,128 B` and `320,864,256 B`. Neither lower bound supplies an exact paired GPU peak ratio. The original reducer reproduced the pending gate offline.

The exclusive session ran at 00:12:44–00:14:06 Singapore time on November 19, 2026. The Ryzen 9 7950X3D ran both applications on CPUs `0–31` with 32 threads. The selected RTX 3090 at `0000:01:00.0`, UUID `GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver `580.178.04`, was shared by actual native CUDA and pinned OpenCL index 0. The CUDA witness reported physical GPU and no fallback, and neither application’s logs showed CPU or GPU alignment helper fallback. Seven reference launches reported a PyOpenCL compiler warning. The archived launcher binds the exact `875e8800fe95806b840bf062dc211940acc12191dec3a19a4561698a99fe355f` ELF and exact source and lock to P6's produced native host archive and separate physical parity. [Candidate build and scoped native parity](../e9-exposure-resident/README.md)

All nine E9 source hashes remained unchanged. Independently decoded warm-one outputs reproduce both historical encoded and pixel hashes of the earlier corrected GPU pair, at genuine `337×510` RGB16. The full-frame MAE is `515.077` in unsigned 16-bit channels. The independent review of those exact images records comparable detail and localized channel clipping differences. Current fixed-window measurements agree with the same pair. The original report carries its separate quality placeholder. Broader same-source release verification and device-memory assurance remain open. [Current fixed-window metrics](fixed-roi-review.json) · [Prior review of the same images](../integrated-e9-corrected-cuda-c1e2cac/independent-gpu-quality-review.md)
