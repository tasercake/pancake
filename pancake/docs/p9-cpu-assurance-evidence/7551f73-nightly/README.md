# Integrated pinned CPU branch coverage

The unchanged `branches-cpu` gate on clean
`7551f73dbe5132f7e9723a9ac4b0c9144504a27a` ran 145 passing tests across
19 nonempty CPU targets and an empty doctest harness. The comparator
printed `CPU assurance: uncovered CPU lines` and the gate exited 1.
The reported totals preserve the existing zero-uncovered requirements.
[Raw gate log](branches-cpu.log.gz) [Status](branches-cpu.status)

The Linux x86-64 Ryzen 9 7950X3D run used pinned `nightly-2026-11-15`,
LLVM 23.1.5 and cargo-llvm-cov 0.9.1. The locked all-feature scope,
CPU integration selection, gate and comparator match 287828a.
The two build jobs ran on CPUs 0 and 1 in a fresh task-owned target.
GNU time recorded 3m11.58s with the original exit status.
[Baseline](../287828a-nightly/README.md) [Environment](environment.txt)
[Command](command.txt) [Time](time.txt)

| Metric | 287828a covered / total | 7551f73 covered / total | Change in uncovered |
|---|---:|---:|---:|
| Lines | 4,466 / 6,250 | 4,644 / 6,456 | +28 |
| Functions | 454 / 728 | 474 / 753 | +5 |
| Regions | 4,551 / 7,025 | 4,766 / 7,271 | +31 |
| Branch outcomes | 552 / 686 | 573 / 762 | +55 |

The byte-identical `src/fusion/focus.rs` gained 15 covered lines,
one function and 22 regions. Byte-identical
`src/registration/estimate.rs` gained one covered region. These are
the only byte-identical mapped files with a change in covered code
coordinates. No previously covered coordinate was lost in those files.

The tiled decoder gained 126 covered lines and 16 branch outcomes
as its denominators increased by 143 lines and 26 branch outcomes.
The changed Linux panel gained 24 covered lines and three branch
outcomes while adding 55 mapped lines and 42 branch outcomes. The
native desktop session remains the largest line gap at 776.
[All mapped files](per-file-comparison.json.gz) [Source changes](source-comparison.txt)
