# Exact integrated Linux CPU branch coverage

The unchanged `bash pancake/scripts/check.sh branches-cpu` gate exited 1
on `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6`. All 164 CPU tests passed
across 20 executables, including 17 selected integration targets. The
separate documentation phase completed with zero doctests. There were
zero failed, ignored, measured, or filtered tests.

The stock export completed, and the strict comparator reported
`CPU assurance: uncovered CPU lines`. The retained nightly LLVM
coverage map reports the following counts.

| Mapping | Covered | Total | Uncovered |
| --- | ---: | ---: | ---: |
| Lines | 4,799 | 6,596 | 1,797 |
| Functions | 494 | 771 | 277 |
| Regions | 4,923 | 7,416 | 2,493 |
| Branches | 589 | 778 | 189 |

The run used the pinned `nightly-2026-11-15` and `cargo-llvm-cov 0.9.1`,
with two direct offline Cargo jobs on logical CPUs 2 and 3. It ran on
Ubuntu 24.04.5 LTS from 2026-11-19 10:57:21 to 11:02:36 SGT. The working
tree was clean at both ends. The command, target roster, full log,
unmodified coverage JSON, and indexed profile are preserved. The owned
target clean removed 19.5 GiB.
