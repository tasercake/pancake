# Exact 0a9ae85 Debian E9 CPU qualification

| --- | ---: | ---: | ---: | ---: | --- |
| Warm median completed job | 0.369367719 s | 0.901075783 s | 0.409918595 | 0.50 | Pass |
| Cold median completed job | 0.356941017 s | 6.517181212 s | 0.054769233 | 1.00 | Pass |
| Peak process-tree RSS | 108,871,680 B | 348,958,720 B | 0.311990140 | 0.50 | Pass |

The exclusive CPU run took place on November 19, 2026 at 01:24:01–01:25:11 Singapore time. Both applications used CPUs `0–31` and 32 threads on the Ryzen 9 7950X3D Linux host. Each primary RSS observation met the 20 ms cap, with maximum gaps of `11.098331 ms` for the candidate and `9.258716 ms` for the reference. The CPU profile made no GPU-device queries. The original reducer reproduced its recorded result offline.

Independent TIFF decoding reproduced the exact prior corrected CPU pair in both encoded file and RGB16 pixel digests at `337×510`. The source hashes match the same earlier independently reviewed scene. Its fixed-window conclusions and localized channel clipping differences therefore apply to the observed images. The evidence is specific to this CPU profile and exact package. [Current fixed-window metrics](fixed-roi-review.json) · [Prior independent review of the same exact images](../integrated-e9-corrected-cpu-c1e2cac/independent-quality-review.md)
