# Integrated Linux CPU AddressSanitizer

Exact clean revision `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`
passed the unchanged `bash scripts/check.sh sanitizer address` gate.
All 148 tests passed across 19 result groups, including 28 native image
I/O tests and 18 software and offscreen GUI tests. The gate exited 0
with zero failed, ignored or filtered tests and no AddressSanitizer or
LeakSanitizer diagnosis. [Raw log](sanitizer-address.log.gz) · [Receipt](receipt.json)

The gate used pinned nightly `nightly-2026-11-15`, the locked all-feature
workspace, the rebuilt standard library and the original address-sanitizer
flags. The Ryzen 9 7950X3D Linux x86-64 host used CPUs 14 and 15 and two
Cargo jobs. The 16 classified CPU integration targets are byte-identical
to the previous pinned `c1e2cac` replay. GNU time recorded 6m48.63s through
fresh compilation and completed tests. [Exact command](command.txt) · [Host environment](environment.txt) · [Process record](time.txt)

The receipt includes SHA-256, size and ELF build ID for all 19 executed
test binaries and the instrumented application CLI. The tested GUI unit
executable and application CLI both define `__asan_init`. The isolated
target occupied 21 GiB before removal. [Runtime symbols](sanitizer-runtime-symbols.txt) · [Disk](disk-before-cleanup.txt)

The run covers the Linux CPU AddressSanitizer gate. Physical GPU execution
and the other sanitizer gates have their own exact-source receipts.
[Target list](selected-targets.txt) · [Cargo metadata](assurance-metadata.json.gz) · [File checksums](SHA256SUMS)
