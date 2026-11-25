# Exact integrated Linux stable CPU coverage

Clean `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` ran the unchanged
`bash scripts/check.sh coverage-cpu` gate on 2026-11-19 local time.
All 148 tests passed across 19 harnesses, with zero failed, ignored,
measured or filtered-out tests. The classifier selected 16 integration
harnesses and the normal workspace libraries and binaries. The canonical
command exited 1 after emitting `CPU assurance: uncovered CPU lines`.
The zero-uncovered thresholds and comparator were preserved.
[Original command log](coverage-cpu.log.gz), [exit status](coverage-cpu.status),
[classifier output](selected-targets.txt), [classifier input](metadata.json.gz).

| Stable coverage metric | Covered / total | Uncovered |
| --- | ---: | ---: |
| Lines | 4,730 / 6,516 | 1,786 |
| Functions | 485 / 760 | 275 |
| Regions | 4,849 / 7,328 | 2,479 |

There are zero branch records at the file and function levels. The stable
0/0 branch totals supply no branch-coverage verdict. The original LLVM
JSON is preserved byte for byte after decompression. [Original map](coverage-cpu.json.gz),
[indexed profile](coverage-cpu.profdata.gz), [summary](summary.json).

The newly attributed GTK child hits the real panel open, build,
GTK setup, and setup-error closure once, at lines 27, 64, 65 and 66. The
full scoped `panel/linux.rs` result is 59/176 lines, 8/15 functions and
64/197 regions. The largest remaining source gaps include 776 lines in
`session.rs`, 193 in `desktop_menu.rs` and 117 in `panel/linux.rs`.
No physical GPU case or desktop session ran.

Rust `1.100.0 (0ef1bc386)` with LLVM 23.1.5 and cargo-llvm-cov 0.9.1
ran on Linux `6.8.0-147-generic` x86-64 with an AMD Ryzen 9 7950X3D.
The cold instrumented run took 2m59.31s with two jobs on CPUs 0 and 1.
Compiler wrappers, display endpoints, desktop bus endpoints and any
inherited profile pattern were unset. [Exact command](command.txt),
[run timings](coverage-cpu.time), [checksums](SHA256SUMS).

Source Git tree `fde57a0b461e3ea031ea969e1e841927bead17f9`.
Lockfile SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Original JSON SHA-256 `1738331fe01e6d396df47345c4315ca4a4a8d06232806cd6f7dd828d316dbd17`.
