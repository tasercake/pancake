# Separate limited T50 Weighted CPU RSS evidence

| Actual measurement | Pinned reference | Packaged 11c |
| --- | ---: | ---: |
| Peak process-tree RSS | 3,471,663,104 B | 1,831,284,736 B |
| Greatest actual RSS gap | 18.417888 ms | 14.349394 ms |
| RSS observations through exit | 26,146 | 10,993 |
| Completed-job elapsed | 65.045475839 s | 25.501249585 s |

The ratio is **0.5274949444**, below **0.55**, with both arms eligible at
the unchanged **20 ms actual** rule including spawn and reap edges.
Peak estimates are sampled process-tree RSS. The pair's wall times
are descriptive. The [separate ten-job wall and quality replay](../t50-weighted-11c-limited-measured/README.md)
passed its warm and cold thresholds and retains its own strict RSS
cadence miss. This new evidence supplies the separately authorized RSS
criterion and gives no frozen-full P10 credit.

Both fresh outputs were 5998×3998 native RGB16 and matched the prior
encoded and decoded image hashes exactly. Both explicit CPU paths
completed without fallback. The reference emitted 50 keyed identity
maps, ordered externally witnessed inputs and a one-pixel crop. Pancake
emitted all 50 ordered keyed identity maps and 50 ordered sequential
identity matrices. The independent truth oracle read all 50 regions in
both fresh images. Both producers and all 52 Stage6 files authenticated
before and after. The runner released the common lock at 12:00:25.902 SGT
on 2026-11-19 with exit 0. [Quality and producer details](summary.json)
