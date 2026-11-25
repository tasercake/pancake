# Integrated Linux CPU randomized layouts for 11cfcf8

The unchanged `bash scripts/check.sh layouts-cpu` gate passed on exact
clean revision `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6`. Each pinned
seed ran all 20 selected result groups.

| Layout seed | Passed | Failed | Ignored | Filtered | Cargo exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 164 | 0 | 0 | 0 | 0 |
| 42 | 164 | 0 | 0 | 0 | 0 |

The complete gate exited 0 with zero compiler or test diagnostics. Each
seed's executed suite/name roster matches all 164 tests in the exact
CPU dev JUnit run, including the Weighted, Hybrid, Laplacian and recent
registration checks. The 17 integration targets match that CPU selector
byte for byte. [Full log](layouts.log.gz) · [Seed 1](seed-1.log.gz) · [Seed 42](seed-42.log.gz) ·
[Exit and group counts](receipt.json) · [Named rosters](test-rosters.json.gz) · [Exact CPU dev JUnit](nextest-junit.xml.gz)

Pinned `nightly-2026-11-15` built and ran both seeded layouts on the Linux
x86-64 Ryzen 9 7950X3D host. Cargo ran directly and offline, using CPUs
12 and 13 and two jobs. The gate ran 11:28:30–11:39:23 SGT on 2026-11-19.
GNU time recorded 10m53.22s through both fresh builds and executions.
[Host and tools](environment.txt) · [Command](command.txt) · [Run time](layouts.time)

The two runs used 40 distinct native test executable paths. The identity
manifest records SHA-256, size and GNU Build ID for each. The isolated
target occupied 35 GiB before cleanup. This receipt covers the Linux CPU
layout gate. [ELF identities](native-artifacts.json) · [Disk](disk-before-cleanup.txt) · [Checksums](SHA256SUMS)
