# Corrected E9 CUDA and OpenCL performance

| --- | ---: | ---: | ---: | ---: | --- |
| Warm median completed job | 0.408888675 s | 0.784562701 s | 0.521168 | 0.50 | Fail |
| Cold median completed job | 0.564428067 s | 7.160841019 s | 0.078821 | 1.00 | Pass |
| Peak process-tree RSS | 233,172,992 B | 546,979,840 B | 0.426292 | 0.50 | Pass |
| Peak attributable device memory | Pending | Pending | Unavailable | 0.50 | Pending |

The exclusive campaign ran from 22:28:56.452757 to 22:30:18.221747 Singapore time on November 18, 2026. Both targets used CPUs `0–31`, 32 threads and the same RTX 3090 on the Ryzen 9 7950X3D Linux host. The reference preflight verified OpenCL index 0 was the same UUID `GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac` as the candidate's CUDA device at PCI `0000:01:00.0`, driver `580.178.04`. The instrumented CUDA completion recorded physical execution and no fallback. The reference used the NVIDIA OpenCL loader and the verified equal-final-batch `4+4+1` adapter. No full CPU fallback or GPU alignment helper fallback appeared in the job logs. Seven reference jobs recorded ordinary PyOpenCL compiler-output warnings. The preflight recorded ordinary desktop helper activity and 0% GPU use at entry before either job.

Process-tree RSS was observed at an 8 ms nominal interval. The maximum actual gaps were 9.616001 ms on CUDA and 10.362233 ms on OpenCL. The separate attributable GPU-memory metric had gaps over 20 ms in all 17 CUDA jobs and four of 17 reference jobs. Maximum gaps were 34.084586 and 39.220688 ms. Observed process-attributed device-memory lower bounds were 295,698,432 B and 320,864,256 B. The frozen reducer leaves both strict peaks and the pair ratio pending. No numerical retry, accounting-mode change or interpolation was applied.

The first warm pair had full-frame MAE 515.077 unsigned 16-bit channel levels. Independent native-scale and enlarged review found comparably preserved sampled detail in all three fixed windows, each with zero clipping. The comparison retained 364 additional clipped blue-channel samples near dark right-balloon artwork and 731 additional upper-endpoint channel samples around the sun and nearby clouds. The reviewer found no discernible texture loss at native display scale. This is a positive representative review of the actual pair from one exposure fixture. The original measured report records its separate quality placeholder and the profile remains failed on warm performance. [Independent quality review](independent-gpu-quality-review.md) · [Independent metrics](independent-gpu-quality-metrics.json) · [Original fixed window measurements](fixed-roi-review.json)

## Identity and reproduction

The private binding independently verifies the producer ELF, its native build and replay, CUDA runtime files, corrected reference launcher and locked runner core. It delegates the original paired measurement, sampler, reducer and command parsing in private function namespaces. No-device controls rejected different types of candidate source drift and verified only candidate provenance is rebound. The real preflight verified the exact reference and 34 entries without device queries or launches.
