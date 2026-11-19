# Exact 11cfcf8 E9 CPU evidence

## Completed fixed 8 ms run

| --- | ---: | ---: | ---: | --- |
| Warm completed-job median | 0.373322134 s | 0.903272565 s | 0.413299538 | Pass |
| Cold completed-job median | 0.372137981 s | 6.608340910 s | 0.056313375 | Pass |
| Peak process-tree RSS | Pending | 345,837,568 B | Pending | Pending |

All 17 primary reference RSS traces met 20 ms; the worst gap was
9.377806 ms. Sixteen of 17 primary candidate traces met the bound. Candidate
cold-05 had an interior gap of 28.073483 ms. Its second observation began
14.208167 ms after its scheduled time, and that acquisition took
1.180651 ms. The raw observed 108,158,976 B for that job remains recorded
as a sampled value. The required strict campaign memory result remains
pending, and this exact E9 CPU profile remains unqualified.

Independent decoding confirmed true RGB16 at 510×337 and exact equality
to the earlier independently reviewed corresponding corrected CPU files
and pixel hashes for both apps. Whole-image difference against the
reference was MAE 510.171 u16 and RMS 634.898 u16. The three original
fixed-window metrics and zero endpoint clipping inside those windows
reproduced exactly. The previously recorded localized extra endpoint
incidence still applies. [Current independent review](8ms-full/independent-first-warm-review.json)
· [Earlier qualitative review of these exact pixels](../integrated-e9-corrected-cpu-c1e2cac/independent-quality-review.md)

## Interrupted 3 ms attempt

The two raw packets contain 192 and 130 verified files. They include
allowlisted launch data and scoped console output, with no original
photo bytes, derived TIFFs, application binaries or ambient process
environment dumps. The two independently reviewed private TIFFs were
rehash-checked and removed, and both work scopes held zero derived
outputs and case caches after cleanup. The sole nine-file owner stage
remains available for separate authorized read-only work.
[Cleanup receipt](cleanup.json) · [Checksums](SHA256SUMS)
