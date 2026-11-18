# Sealed E9 diagnostic on NVIDIA Vulkan and OpenCL

All 34 primary process launches produced a genuine 337 × 510 RGB16 image
from the nine original hash-verified Balloons PNGs. Each app completed two
warmups, ten warm samples and five cold samples. The separate post-sample
diagnostics verified explicit physical execution without fallback and matched
each app’s warm pixel hash. Exact CLI arguments, output hashes, isolated
caches, and driver and runtime provenance are retained.

| --- | ---: | ---: | ---: | --- |
| Warm median | 0.733234 s | 0.769889 s | 0.952389 | Misses ≤0.50 |
| Cold median | 0.931113 s | 7.036801 s | 0.132320 | Meets ≤1.00 |
| Process-tree RSS | Pending | Pending | Unavailable | Observation gap exceeded 20 ms |
| Attributable GPU peak | Pending | Pending | Unavailable | Observation gap exceeded 20 ms |

Seventeen candidate and twelve reference runs exceeded the maximum gap,
with worst gaps of 48.634 and 48.275 ms. Observed GPU residence lower bounds
were 204,443,648 and 320,864,256 bytes. Individual process kernel RSS high
water was 397,557,760 and 549,371,904 bytes. Neither field supplies the
required paired memory ratios.

The same decoded per-app output hash repeated in all 17 launches. Independent
full-image comparison yielded 2,825.18 MAE and 3,482.73 RMS on the RGB16 scale
of 65,535. The maximum channel difference was 19,150. Equal output quality
remains unresolved. The collector preserved the failed warm result and
deleted all TIFF outputs, including the prospective review pair, and all
per-run caches. The original ordered inputs rehashed unchanged.
