# One matched E9 diagnostic on the registration-fixed Linux CLI

Exactly two complete warmups, ten measured warm runs, and five measured
cold runs completed for each application. Within each pair the target
order alternated. The primary runs used uninstrumented fresh processes,
separate scoped caches, and the full frozen profile. Independent
backend-provenance jobs ran after the primary samples. Both completed on
the explicit CPU path.

|---|---:|---:|---:|---|
| Warm completed-job median | 1.08585 s | 0.89903 s | 1.20781 | Misses the 0.50 maximum |
| Cold completed-job median | 1.07501 s | 6.53525 s | 0.16449 | Within the 1.00 maximum |
| Peak process-tree RSS | Pending | Pending | Unavailable | Collection exceeded the 20 ms interval |

The warm profile failed. Its 0.44951 s target median is 2.42 times below
the observed Pancake median. The run remained a diagnostic for one profile
and one reviewed source revision. The other twelve recipes/method sets
and release candidate measurements remain open.

The isolated warm candidate cache held zero files after its warmups.
The reference warm cache held ten files and 725,627 bytes before its
first measured warm job. That is an observed persistence difference.
Additional phase attribution remains separate from the completed-job
samples.

Every closed export independently decoded as genuine 337 × 510 RGB16.
Each application's decoded hash was identical across its timed runs and
matched its own earlier [paired quality inspection](../registration-recheck/README.md).
Full image MAE between applications was 2823.78 of 65535, and the inspected
regions still need equal-quality review. All output TIFFs and per-run
caches were removed. The raw archive contains commands, exact build and
source provenance, timing samples, resource traces, diagnostics, and
cleanup records.
