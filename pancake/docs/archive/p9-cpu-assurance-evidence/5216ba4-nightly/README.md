# Nightly CPU branch assurance on the sealed Linux source

The unchanged `bash scripts/check.sh branches-cpu` ran from the clean
release source `5216ba4227e14033483a21bb5eca1c6c168ef840`. It used the pinned
`nightly-2026-11-15`, LLVM 23.1.5, and cargo-llvm-cov 0.9.1 on an Ubuntu 24.04.5
Ryzen 9 7950X3D host. It selected the configured CPU unit, binary,
and integration targets plus the separate documentation pass. Allocation was
limited to two low-priority build jobs on distinct physical CPU cores
14 and 15 while the private-desktop release checks ran separately. The
branch configuration, assertions and instrumentation were unchanged.

All 97 selected executable tests passed across 19 suites, with zero failures
or ignored tests. The documentation pass contained zero doctests. The
19 suites used their checked-in fixtures and generated data. The retained
log gives the complete test selection and results.

| LLVM counter | Covered | Total | Uncovered |
| --- | ---: | ---: | ---: |
| Lines | 3,913 | 5,773 | 1,860 |
| Functions | 385 | 670 | 285 |
| Regions | 3,988 | 6,549 | 2,561 |
| Branches | 466 | 612 | 146 |

The required gate returned exit 1 after 5:17.41. Its coverage comparator
reported uncovered CPU lines. The emitted nightly report also records
146 uncovered branches. Exact commands, revision and lock identifiers,
complete output, indexed profile data, and source-mapped JSON are retained
beside this receipt.
