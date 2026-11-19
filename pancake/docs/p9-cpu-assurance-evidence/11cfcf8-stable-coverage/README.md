# Stable CPU coverage on the Weighted Average release source

Clean `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6` ran the unchanged
`bash scripts/check.sh coverage-cpu` gate on 2026-11-19. All 164 tests passed
across 20 harnesses, with zero failures, ignored tests or filters. The
canonical inventory selected 17 integration targets and the workspace
libraries and binaries. The new CPU resident allocation test, repeated
registration regression, and headless GTK error probe all ran.
[Canonical log](coverage-cpu.log.gz), [exit status](coverage-cpu.status),
[selected targets](selected-targets.txt), [classifier input](metadata.json.gz).

The strict gate exited 1 with `CPU assurance: uncovered CPU lines`. The
zero-uncovered thresholds and comparator were unchanged.

| Stable coverage metric | Covered / total | Uncovered |
| --- | ---: | ---: |
| Lines | 4,799 / 6,596 | 1,797 |
| Functions | 494 / 771 | 277 |
| Regions | 4,923 / 7,416 | 2,493 |

The original LLVM report has zero branch records in both file and function
arrays. The stable 0/0 branch summary supplies no branch verdict. The map
and indexed profile are retained as emitted. Compared with the earlier
`0a9ae85` measurement, all three authored denominators changed.
[Original map](coverage-cpu.json.gz), [indexed profile](coverage-cpu.profdata.gz),
[summary](summary.json), [earlier scope](../0a9ae85-stable-coverage/README.md).

On the Linux x86-64 AMD Ryzen 9 7950X3D host, Rust 1.100.0 (`0ef1bc386`)
used LLVM 23.1.5 and cargo-llvm-cov 0.9.1. The command ran offline with
at most two build workers on CPUs 0 and 1, and took 3m00.01s. No physical
GPU case or desktop session ran.
[Exact command](command.txt), [timings](coverage-cpu.time),
[input pins](input-hashes.txt), [checksums](SHA256SUMS).

Source tree `0ea17aeb65a346e3cb6b5593d8c8b4a02e827832`.
Original report SHA-256 `16628567472eff9c53c83d791373ce41f8f6114f07a2d5c485ed7869c5fe1898`.
