# R50 Hybrid CPU on the exact Linux release

The quiet window ran 15:03:28–15:45:15 SGT on 2026-11-18 with both targets
on the same AMD Ryzen 9 7950X3D. Both used logical CPUs 0–14 (one thread
from each physical core), 15 worker threads in the launcher settings,
32 GiB per-process address-space cap, and separation from the RSS collector
pinned to CPU 15. CPU 31, the collector's SMT sibling, was also outside the
worker affinity. The frozen settings were Hybrid, first-frame similarity
alignment, kernel 6, eight pyramid levels, alignment scale 6, auto crop,
no slabs, and RGB16. [Quiet window](quiet-lease.txt)
[Host and thermal start](window-start.txt) [End and exit status](window-end.txt)

## Frozen results

All 34 primary launches succeeded in schedule order, with two warmups,
ten warm measurements and five fresh-cache cold measurements per target.
Both diagnostic jobs confirmed CPU execution and reproduced all ten warm
pixel digests for their respective target. The strict profile verdict was
**fail**, with both speed gates missed. [Summary](summary.json)
[36 samples](samples.csv)

| --- | ---: | ---: | ---: | --- |
| Warm median, limit 0.50 | 94.402869 s | 40.597604 s | 2.325331 | Fail |
| Cold median, limit 1.00 | 95.289787 s | 46.766705 s | 2.037556 | Fail |
| Process-tree peak RSS, limit 0.50 | Pending | Pending | Unavailable | Pending |

## Geometry and focus review

## Replay and cleanup

The stage originals, TIFFs, previews, runtime caches and independent package
copy were removed after their hashes and logs were retained. Two originals
were transferred with verified hashes to the separate RAW decode parity
check, whose owner handles their cleanup.
[Cleanup](cleanup.json) [Two-file transfer](staging-transfer.json)
