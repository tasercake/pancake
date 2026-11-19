# Supplemental Linux Miri after the spectral threshold test

At clean `54e3a8d60025cf77daa655e5e66781a82f515f7c`, pinned Miri passed all
27 workspace library tests with zero failed, ignored or filtered tests.
The new weak spectral peak test, the full 32,768-pixel typed RAW
allocation and sample test, and both TIFF cursor and lane tests ran.
The reviewed source inventory validated and Miri exited 0.
[Original log](miri.log.gz), [actual test selection](selected-tests.txt),
[inventory validation](reviewed-source.log), [exit status](miri.status).

The supplemental Linux run used the same full locked workspace library
command as `5b98890`, with strict provenance, symbolic alignment checks and
isolation enabled. The canonical Apple Miri gate remains separate.
[Previous Linux receipt](../5b98890-linux-miri/README.md).

The Linux x86-64 AMD Ryzen 9 7950X3D host ran with `nightly-2026-11-15`,
Miri 0.1.0 (`5f4effd579`) and LLVM 23.1.5. Cargo ran offline with two build
jobs on CPUs 10 and 11. GNU time measured 3m58.46s and 1,481,688 KiB peak
RSS. No application, desktop or device case ran.
[Exact command](command.txt), [toolchain and source hashes](input-hashes.txt),
[timings](miri.time), [archive checksums](SHA256SUMS).

Source Git tree `9fa7e0d1efe91c825d65c0460fb114075571c845`.
Original log SHA-256 `399ffbbe7e688c15cf249053b0e94f138b68c61b52c0873881e3cc62ec73e92b`.
