# Exact integrated Linux nightly CPU branch coverage

Clean `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` ran the unchanged
`bash scripts/check.sh branches-cpu` gate on 2026-11-19 local time.
All 148 tests passed across 19 nonempty harnesses. The separate doctest
harness reported zero tests. There were no failed, ignored, measured or
filtered-out tests. The canonical command exited 1 after reporting
`CPU assurance: uncovered CPU lines`. The zero-uncovered requirements for
lines, functions, regions and branches were preserved.
[Canonical log](branches-cpu.log.gz), [exit status](branches-cpu.status),
[selected targets](selected-targets.txt), [classifier input](metadata.json.gz).

| Nightly coverage metric | Covered / total | Uncovered |
| --- | ---: | ---: |
| Lines | 4,730 / 6,516 | 1,786 |
| Functions | 485 / 760 | 275 |
| Regions | 4,849 / 7,328 | 2,479 |
| Branch outcomes | 577 / 764 | 187 |

The ordinary exporter supplied actual branch records at both the file and
function levels. Lines, functions and regions match the same-source stable
run. The native GTK setup error still executed once at each of lines 27,
64, 65 and 66. `panel/linux.rs` covered nine of its 42 branch outcomes. The
next largest measured file branch deficits were 20 each in `tiled.rs` and
`estimate.rs`. No physical GPU case or desktop session ran.
[Original map](branches-cpu.json.gz), [indexed profile](branches-cpu.profdata.gz),
[summary](summary.json), [same-source stable measurement](../0a9ae85-stable-coverage/README.md).

The Linux x86-64 `6.8.0-147-generic` AMD Ryzen 9 7950X3D host used
`nightly-2026-11-15` (`5f4effd57`, LLVM 23.1.5) and cargo-llvm-cov 0.9.1.
The run took 3m09.49s on CPUs 0 and 1 with at most two build jobs. Cargo
ran offline. Compiler wrappers, desktop endpoints and inherited profile
patterns were unset. [Exact command](command.txt), [timings](branches-cpu.time),
[pin and input hashes](input-hashes.txt), [checksums](SHA256SUMS).

Source Git tree `fde57a0b461e3ea031ea969e1e841927bead17f9`.
Original JSON SHA-256 `19f8a0866849b63f750cc3477e683345dae29554bb9d8eb0f0fa4df745ae7c11`.
