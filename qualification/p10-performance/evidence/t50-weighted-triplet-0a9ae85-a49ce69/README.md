# Exact 0a and a49 T50 Weighted Average CPU diagnostic

The 15 worker CPUs were logical 0–14, each on a distinct Ryzen 9 7950X3D
core. The RSS sampler used CPU15 on the remaining core. Each job started
with its own empty application and compiler cache. OS caches followed
normal host behavior. The monotonic blocking timer covered spawn to
successful process exit with a closed export. The runner began at
02:36:39.661 SGT on 2026-11-19. All children were terminal by 02:38:57.546,
and the runner released the common lock after postflight at 02:39:00.534.
Every app received the same 50 ordered native 6000×4000 RGB16 TIFFs and
completed the unchanged T50 Weighted Average CPU profile.

| --- | ---: | ---: | ---: |
| Whole-job elapsed time | 65.067805 s | 43.381687 s | 25.506298 s |
| Process-tree RSS samples | 7,932 | 5,421 | 3,184 |
| Maximum RSS observation gap | 22.476502 ms | 15.554330 ms | 19.693276 ms |
| Observed peak process-tree RSS | 3,466,428,416 B | 2,119,286,784 B | 1,831,522,304 B |
| Cadence-eligible peak tree RSS | Pending | 2,119,286,784 B | 1,831,522,304 B |
| Whole-output MAE against generated sharp truth | 1083.931 u16 | 1080.736 u16 | 1080.736 u16 |

The resident CLI took 0.587951× the packaged stock wall time, a 41.20%
reduction, and 0.391996× the reference wall time in this fixed order.
The two Pancake RSS observations met the 20 ms interval rule with a
resident/stock ratio of 0.864216×. The reference gap exceeded the limit,
so both reference-relative memory ratios remain pending. A separate
postflight recomputed each peak, the count and the maximum gap directly
from the raw observation timestamps, including the spawn and exit edges.
The sampled resident/reference RSS ratio was 0.528360×. Its strict
verdict remains pending because the sampled reference peak is incomplete.
[Independent typed audit](independent-typed-audit.json)

The three first-launch observations confer zero frozen performance or
release qualification credit.

All three exports were genuine 5998×3998 RGB16. The reference emitted
50 first-reference identity maps and a one-pixel crop on all sides. It
reported CPU for alignment, warp and fusion. Both Pancake runs emitted
the same 50 ordered, keyed reference maps and 50 ordered sequential
identity matrices, consistent with a symmetric one-pixel crop. The stock
and resident TIFF byte digests are both `60a45fe4…` and their independently
decoded pixel digests are both `f8a00c59…`. All 71,940,012 decoded channel
samples match between the two Pancake arms. Each differs from the genuine
reference by 11.099 u16 MAE, 20.702 u16 RMS and a maximum of 227 codes.
The unchanged sharp-truth oracle covered all original 50 depth strips.
Reference vertical detail ratios range from 0.840468 to 0.882494, and
both Pancake exports repeat 0.840504 to 0.882531. The outputs represent
final weighted RGB. An independent comparison decoded all three TIFFs,
rechecked the full pixel equality, and reviewed the same 50 center regions
and eight fixed crops at a common display mapping.
[Independent review](independent-triplet-review.md) · [Image audit](independent-image-audit.json)

The Stage6 owner granted a read-only loan and retained cleanup custody.
The runner authenticated its ownership marker, materialization and all
52 exact original records before and after the three jobs and truth
review. The borrower returned the shared stage after postflight. The
three private derived TIFFs were rehashed after the independent reviewer
released their reads. The exports and case caches were then removed. Stable producer
artifacts and Git source refs were preserved.
[Shared stage](../t50-stage6-e81079d/README.md) · [Owned cleanup](cleanup.json)

The archive includes only typed launch and result records, raw RSS traces,
typed backend/crop/maps, all 150 truth-region records, static staging,
and the scoped quiet/census receipts. Recorded environment controls are
limited to thread limits and case-owned cache paths. The archive was
checked member for member. It contains zero original or derived TIFFs
and zero ambient environment records. [Checksums](SHA256SUMS)
