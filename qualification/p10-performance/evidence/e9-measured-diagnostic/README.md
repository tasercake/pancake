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

## Separate phase and kernel RSS observation

One same-binary diagnostic captured the native CLI's published progress
lines and glibc `LD_DEBUG=statistics` output. Parent-controlled developer
builds could run during this observation. Its values are excluded from
the paired timing and the memory gate.

The observer saw the first completed-input line 226 ms after dispatch,
then the last line at 576 ms and successful process exit at 607 ms.
The three observable intervals were 226 ms through preparation and the
first input, 350 ms for the remaining input pipeline, and 31 ms for final
fusion, export and exit. The largest intervals between input markers
were 111 ms for frame 2 and 95 ms for frame 4. Those markers do not resolve
the execution-runtime qualification and initial kernel compilation
inside the first phases. The dynamic linker reported 1,146,070 startup
cycles.

Linux `wait4` reported 121,647,104 bytes of process RSS high-water,
4.487 s of user CPU, and 0.508 s of system CPU. This single-process metric
cannot fill the missing concurrent process-tree gate or supply a paired
reference ratio. The diagnostic completed with the same decoded E9
hash as every measured candidate sample. The original source inputs were
unchanged and its temporary TIFF and scoped cache were removed.
