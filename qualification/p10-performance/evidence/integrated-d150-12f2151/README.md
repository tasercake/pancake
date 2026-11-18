# D150 Hybrid CPU on the exact Linux release

The coordinated benchmark window ran 13:41:43–14:22:06 SGT on
2026-11-18. Both targets used the Ryzen 9 7950X3D CPU, logical IDs 0–15
(one per physical core), the same 16-thread settings and inherited 32 GiB
per-process address-space limit. The complete frozen profile used breathing
alignment, first-frame reference, slabs 40/overlap 4, kernel 6, eight pyramid
levels, scale 6, auto-crop and RGB16.
[Start conditions](window-start.txt) [End conditions](window-end.txt)

## Frozen numeric results

All 34 primary launches completed successfully in the prescribed alternating
order. Each target completed two warmups, ten warm measurements and five
fresh-cache cold measurements. Both additional diagnostic jobs confirmed CPU
execution and reproduced their target's ten warm decoded-pixel hashes.
[Summary](summary.json) [36 samples](samples.csv)

| --- | ---: | ---: | ---: | --- |
| Warm median, limit 0.50 | 43.356797 s | 88.299508 s | 0.491020 | Pass |
| Cold median, limit 1.00 | 42.756521 s | 95.643264 s | 0.447042 | Pass |
| Process-tree peak RSS, limit 0.50 | Pending | Pending | Unavailable | Pending |

## Geometry and image quality

Both targets exported genuine RGB16. Pancake's 2130×1530 crop retained
84.0% of the reference's 2288×1696 exported area. In common first-frame
coordinates, the distinct rectangles were `[87,99,2217,1629]` and
`[8,16,2296,1712]` as left, top, right, bottom. The reference rectangle comes
from its observed diagnostic crop event. Pancake's origin follows from its
symmetric crop and the independently decoded extent. The raw diagnostic
receipts retain 150 source and sequential matrices per target.

The warm-pair comparison examined the common canvas without resampling.
RGB MAE was 3,220.34 u16 codes, about 12.53 u8 codes, and RMS was 4,805.35
u16 codes. In four matched texture-bearing specimen regions, Pancake's
90th-percentile absolute luma Laplacian was 0.686–0.706 times the reference's.
Visual inspection showed softer detail and doubled edges in the candidate
around the same anatomical structures. Different crop and registration
outcomes remain under investigation. This profile's speed gates passed.
Its RSS and equal-quality acceptance remain open.
[Crop and focus-region metrics](focus-comparison.json)

## Reproduction and temporary data

Use a clean `12f2151` checkout, its exact package-extracted CLI at
`pancake/target/release/pancake-cli`, the pinned reference checkout and
`uv sync --frozen` in its adapter runtime. The paired runner came from
`fbf01d1`, whose only change against this sealed harness was retaining both
requested first-warm review exports after completed gate misses. Existing
timing, sample order, resources and thresholds were used as implemented in
the frozen harness. Artifact paths and digests are in the build attestation.
[Exact cleanup-only difference](review-retention.diff.gz)

The timing and analysis commands, custody records and logs are retained.
The acquisition ZIP, derived previews, runtime caches and isolated staged
package copy were cleaned. The 150 originals and exact first-warm TIFF pair
were transferred with verified hashes into the registration follow-up's
temporary lease. That task owns cleanup after the bounded corrected run.
[Lease and witness hashes](staging-transfer.json) [Local cleanup](cleanup.json)
