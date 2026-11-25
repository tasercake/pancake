# Integrated Linux CPU AddressSanitizer

The exact clean revision `c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb`
passed the unchanged `bash scripts/check.sh sanitizer address` gate.
All 146 tests passed across 19 result groups, with zero failed,
ignored or filtered tests. The process exited 0 and emitted no address
or leak sanitizer diagnosis. The native image I/O group passed 28 tests
and the software and offscreen GUI group passed 18.
[Raw test output](sanitizer-address.log.gz) [Accounting and ELF hashes](receipt.json)

The gate used pinned nightly `nightly-2026-11-15`, the locked all-feature
workspace, the rebuilt standard library and the original sanitizer flags.
The Ryzen 9 7950X3D Linux x86-64 run used CPUs 0 and 1 with two Cargo jobs.
The sanitizer settings, compiler wrappers, display and session-bus
variables were unset. The package and TSan builds had separate CPU
affinity. GNU time recorded 6m01.67s including the fresh build and tests.
[Exact command](command.txt) [Host environment](environment.txt)
[Process time and exit status](time.txt)

The receipt records SHA-256, size and ELF build ID for all 19 executed
test binaries and the instrumented application CLI. Both the CLI and the
GUI unit executable define the sanitizer runtime entry point. The isolated
build target occupied about 21 GiB at the end of the run.
[Runtime symbols](sanitizer-runtime-symbols.txt) [Disk](disk-before-cleanup.txt)
[Previous exact CPU AddressSanitizer result](../../287828a/address/README.md)
