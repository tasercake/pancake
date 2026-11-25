# Integrated Linux CPU randomized layouts

The unchanged `bash scripts/check.sh layouts-cpu` gate passed on exact,
clean revision `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`. Each pinned
seed ran all 19 executable groups, including 28 native image I/O tests
and 18 software and offscreen GUI tests.

| Layout seed | Passed | Failed | Ignored | Filtered | Cargo exit |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 148 | 0 | 0 | 0 | 0 |
| 42 | 148 | 0 | 0 | 0 | 0 |

The complete gate exited 0 with no compiler or test diagnostics.
The full shell trace retains both original flags and command boundaries.
The 16 classified CPU integration targets match the earlier `8694b55`
replay byte for byte. [Combined log](layouts.log.gz) · [Seed 1](seed-1.log.gz) · [Seed 42](seed-42.log.gz) · [Accounting](receipt.json) · [Target list](selected-targets.txt)

The pinned `nightly-2026-11-15` compiler ran offline and directly on Linux
x86-64 and the AMD Ryzen 9 7950X3D host, with CPUs 14 and 15 and two Cargo
jobs. GNU time recorded 8m13.75s through both fresh layouts. The inspected
native runtimes produced 38 distinct executable paths with SHA-256,
size and ELF build IDs. The isolated target occupied 33 GiB before cleanup.
[Exact command](command.txt) · [Host environment](environment.txt) · [Process result](layouts.time) · [Executable identities](native-artifacts.json) · [File checksums](SHA256SUMS)
