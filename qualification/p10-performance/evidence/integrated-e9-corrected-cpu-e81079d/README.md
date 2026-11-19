# Exact e81079d Debian E9 CPU qualification

| --- | ---: | ---: | ---: | ---: | --- |
| Warm median completed job | 0.372614709 s | 0.884656876 s | 0.421196872 | 0.50 | Pass |
| Cold median completed job | 0.379837135 s | 6.533016621 s | 0.058141155 | 1.00 | Pass |
| Peak process-tree RSS | 109,506,560 B | 346,472,448 B | 0.316061380 | 0.50 | Pass |

The exclusive CPU run took place on November 19, 2026 at 00:06:28–00:07:37 Singapore time. The Ryzen 9 7950X3D used logical CPUs `0–31` and 32 threads for both applications. All primary RSS observations met the 20 ms cap. The maximum candidate and reference acquisition gaps were `10.985892 ms` and `9.861492 ms`. The original reducer replay reproduced the recorded result from all sample records. Device memory is outside this explicit CPU profile.

All nine source files remained unchanged. An independent TIFF decoder checked the same `337×510` true RGB16 geometry and reproduced both encoded and decoded hashes from the previously independently reviewed C1 CPU pair. The fixed quality conclusions apply to these identical pixels and sources. That review found adequately comparable sampled detail and preserved the localized differences of 364 lower-endpoint blue-channel samples and 731 additional upper-endpoint channel samples in Pancake. The evidence is specific to this native exposure fixture, device mode and package. [Current fixed-window metrics](fixed-roi-review.json) · [Prior review of the same images](../integrated-e9-corrected-cpu-c1e2cac/independent-quality-review.md)
