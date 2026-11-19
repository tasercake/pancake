# Integrated supplemental Linux Miri after the host regressions

At clean `5b98890d5e976d7287c354091671fff6f87d266c`, pinned Miri passed all
26 workspace library tests with zero failures, ignored tests or filters.
The full 32,768-pixel typed RAW allocation and working-sample comparison,
TIFF cursor and lane tests, stale worker outcome channel test, and both
new estimator geometry tests all ran. The reviewed source inventory
validated and the Miri process exited 0.
[Original log](miri.log.gz), [actual test selection](selected-tests.txt),
[inventory validation](reviewed-source.log), [exit status](miri.status).

This supplemental Linux run reused the full locked workspace-library
command, with strict provenance, symbolic alignment checks, and isolation
enabled. The canonical Apple Miri gate remains separate.
[Prior Linux procedure](../0a9ae85-linux-miri/README.md).

On the Linux x86-64 AMD Ryzen 9 7950X3D host, pinned
`nightly-2026-11-15` reported Miri 0.1.0 (`5f4effd579`) and LLVM 23.1.5.
The command ran offline with two jobs on CPUs 10 and 11. GNU time measured
3m35.73s and 1,481,492 KiB peak RSS. No application, desktop or device case
ran. [Exact command](command.txt), [toolchain and source hashes](input-hashes.txt),
[timings](miri.time), [artifact checksums](SHA256SUMS).

Source Git tree `1fcdfa60274a95c62b8af5b1b9f56415ccc283fa`.
Original test log SHA-256 `87115bf581bf7651c35d26984c87727776b0e0ff69e5af825093034df862db30`.
