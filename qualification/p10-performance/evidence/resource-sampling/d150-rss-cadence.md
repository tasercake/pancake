# D150 RSS cadence investigation

The [sealed D150 Hybrid CPU run](../integrated-d150-12f2151/README.md)
placed the runner and both applications on logical CPUs 0–15 with 16 worker
threads. Replaying the archived 34 primary traces reconstructed all 272,244
observations that completed within their job, every original sample count, and
every recorded maximum gap.

|---|---:|---:|
| Jobs with a gap above 20 ms | 9/17 | 17/17 |
| Gaps above 20 ms between completed observations | 22 | 431 |
| Gaps above 20 ms at launch or exit | 0 | 0 |
| Over-limit gaps with more time inside the snapshot than before it | 18 | 364 |
| Over-limit gaps with snapshot wall time above 20 ms | 0 | 9 |
| Over-limit gaps with more than 20 ms before the snapshot began | 1 | 0 |
| Median snapshot wall time for all rows | 1.868 ms | 1.509 ms |
| Largest snapshot wall time | 16.590 ms | 26.258 ms |
| Largest completion-to-completion gap | 23.834 ms | 33.925 ms |

Dividing the recorded RSS thread CPU time by sample count gives 1.915–2.258 ms
across candidate jobs and 2.074–3.097 ms across reference jobs. Those whole-job
averages include sampler bookkeeping. The old traces time each procfs scan
in wall time, so an individual long scan could include preemption or kernel
blocking. The new raw fields
record the scheduled slot and snapshot thread CPU time for each observation.
The scheduled-to-actual start gap measures wake lateness. Comparing scan wall
time to thread CPU time exposes time off CPU within acquisition.

## Collector placement probe

On the same Ryzen 9 7950X3D host, Python 3.12.3 started a single-CPU process
that forked three workers. All four processes ran for about 0.34 seconds on
logical CPU 18. The collector inherited CPU 18 in the two controls and was
pinned separately to CPU 20 in the other two launches. The runs alternated
placement in control, pinned, pinned, control order. Every launched child
reported affinity `[18]`, and every trace observed all four session PIDs.
Logical CPUs 18 and 20 lie outside the D150 application mask 0–15. They are
SMT siblings of CPUs 2 and 4. This was a scheduler probe
with concurrent build and diagnostic work allowed.

| Collector mask | Maximum gap | Maximum wake lateness | Maximum snapshot wall / thread CPU |
|---|---:|---:|---:|
| 18 | 16.899 ms | 10.623 ms | 2.263 / 2.262 ms |
| 20 | 8.264 ms | 3.554 ms | 1.516 / 1.514 ms |
| 20 | 8.284 ms | 3.913 ms | 1.390 / 1.388 ms |
| 18 | 18.062 ms | 10.047 ms | 1.117 / 1.116 ms |

All four launches met the original 20 ms cutoff. Pinning the sampler reduced
wake lateness in the probe and preserved process attribution and the child
mask. On this host every logical CPU outside the 0–15 D150 mask shares a
physical core with an application CPU. A rerun with the same placement for
both targets would still need to show valid launch-through-exit cadence on
every job. The D150 RSS gate remains pending and its original samples retain
their verdicts. [Linux per-thread CPU affinity](https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html)

The collector continues to enumerate the full process session from procfs.
The delegated cgroup’s `cgroup.procs` lists direct members. A new descendant
can be put in a nested cgroup or migrate without changing its session ID.
Using that list alone would require verified containment for every worker
throughout the job. [Linux cgroup v2](https://docs.kernel.org/admin-guide/cgroup-v2.html)

## Replay

From the repository root, the first command reads the archived measurements
without extracting or changing any files. The second recreates the small
scheduler probe on an eligible Linux host. Its timings vary with the scheduler.

```sh
dir=qualification/p10-performance/evidence
python3 "$dir/resource-sampling/d150-rss-cadence.py.txt" \
  "$dir/integrated-d150-12f2151/raw-paired-runs.tar.gz"
taskset --cpu-list 18 python3 "$dir/resource-sampling/d150-rss-affinity-probe.py.txt" 20
```
