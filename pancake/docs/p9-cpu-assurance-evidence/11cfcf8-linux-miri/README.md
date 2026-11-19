# Supplemental Linux Miri after Weighted Average reuse

At clean `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6`, pinned Miri passed all
27 workspace library tests with zero failed, ignored or filtered tests.
The full 32,768-pixel typed RAW allocation and sample test, TIFF cursor
and lane tests, worker channel test and spectral threshold test all ran.
The reviewed source inventory validated and Miri exited 0.
[Original log](miri.log.gz), [actual test selection](selected-tests.txt),
[inventory validation](reviewed-source.log), [exit status](miri.status).

The run used the same full locked workspace-library command as `54e3a8d`,
with strict provenance, symbolic alignment and isolation enabled. This is
supplemental Linux evidence. The canonical Apple Miri gate remains separate.
[Previous Linux receipt](../54e3a8d-linux-miri/README.md).

The Linux x86-64 AMD Ryzen 9 7950X3D host ran with `nightly-2026-11-15`,
Miri 0.1.0 (`5f4effd579`) and LLVM 23.1.5. Cargo ran offline with two build
jobs on CPUs 2 and 3. GNU time measured 3m35.00s and 1,481,348 KiB peak
RSS. No application, desktop or physical device case ran.
[Exact command](command.txt), [toolchain and source hashes](input-hashes.txt),
[timings](miri.time), [archive checksums](SHA256SUMS).

Source Git tree `0ea17aeb65a346e3cb6b5593d8c8b4a02e827832`.
Original log SHA-256 `1eb6a057981b919f8bad2ec9deb776b84dc15165185cf621a96c9dff208a8423`.
