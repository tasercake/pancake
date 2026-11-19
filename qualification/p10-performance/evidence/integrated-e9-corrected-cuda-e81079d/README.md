# Integrated e81079d E9 CUDA and OpenCL

| --- | ---: | ---: | ---: | ---: | --- |
| Warm median completed job | 0.398120214 s | 0.787651164 s | 0.505452455 | 0.50 | Fail |
| Cold median completed job | 0.539666212 s | 7.209522842 s | 0.074854637 | 1.00 | Pass |
| Peak process-tree RSS | 230,371,328 B | 547,536,896 B | 0.420741195 | 0.50 | Pass |
| Peak attributable device memory | Pending | Pending | Unavailable | 0.50 | Pending |

The warm median exceeded the matched limit by `4.294632 ms`. All process-tree RSS observations met the 20 ms maximum interval, with maximum actual gaps of `9.350922 ms` for CUDA and `11.132232 ms` for OpenCL. Device-memory observation gaps exceeded 20 ms in all 17 CUDA jobs and four of 17 OpenCL jobs, with maximum gaps of `34.745124 ms` and `27.157166 ms`. The attributable observed lower bounds were `291,504,128` and `320,864,256` bytes. The frozen paired reducer leaves both strict device-memory peaks pending.

The exclusive campaign ran at 23:42:47–23:44:09 Singapore time on November 18, 2026. Both processes used CPUs `0–31` and 32 threads on the Ryzen 9 7950X3D. The RTX 3090 at PCI `0000:01:00.0`, UUID `GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver `580.178.04` was selected explicitly by the CUDA candidate and pinned OpenCL device index 0. The physical CUDA witness recorded no fallback and the independently matched same-device OpenCL logs showed no CPU or GPU-alignment-helper fallback. Seven reference jobs recorded ordinary PyOpenCL compiler-output warnings. Resource samples, original timing, backend receipts, thermal settings and raw build logs are preserved. No rerun or interpolation was applied.

The first warm TIFF pair and all nine source hashes reproduce the earlier independently reviewed corrected GPU pair byte-for-byte, including encoded and decoded RGB16 hashes. Current independent TIFF decoding reproduced its three fixed window metrics and whole-frame MAE of `515.077` unsigned 16-bit channel levels. The prior independent representative review documented preserved sampled detail and localized endpoint differences. Its exact narrative and measurements are linked by their SHA-256 values in the summary. The original runner quality placeholder and overall warm failure remain intact. [Current fixed-window metrics](fixed-roi-review.json) · [Prior review of the same exact images](../integrated-e9-corrected-cuda-c1e2cac/independent-gpu-quality-review.md)

The raw archive’s 240 members matched the staged originals byte-for-byte. It includes the allowlisted execution metadata and resource traces, with no ambient environment dumps, TIFFs, original photo bytes or application binary. The native CLI is retained at `artifacts/pancake-linux-native/e81079d/pancake-cli`. The case-owned originals remain staged for the separately authorized exact-package CPU observation. [Waterloo MEF source and permissions](https://ivc.uwaterloo.ca/database/MEF/MEF-Database.html)
