# Integrated Linux CPU AddressSanitizer for 11cfcf8

The exact clean `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6` checkout passed
canonical `bash scripts/check.sh sanitizer address`. All 164 tests passed
across 20 result groups with zero failures, ignored tests, filtered tests
or AddressSanitizer and LeakSanitizer diagnostics. The output records
execution of the Weighted Average, Hybrid transposition, Laplacian blue
detail, low-variance registration and weak spectral peak checks.
[Raw gate output](sanitizer-address.log.gz) · [Typed receipt](receipt.json)

The 17 classified integration targets match the exact CPU dev Nextest check.
An independent reduction matched every executed sanitizer suite/name pair
with the same revision's 164-test JUnit roster and linked each name to its
sanitized executable. The test scope includes headless GUI-crate unit
checks with desktop endpoints unset. [Full named roster](test-roster.json.gz) ·
[Roster counts](roster-summary.txt) · [Exact CPU dev JUnit](nextest-junit.xml.gz) ·
[Selected targets](selected-targets.txt)

The gate ran 10:57:08–11:04:08 SGT on 2026-11-19. Pinned
`nightly-2026-11-15` used the locked all-feature workspace, rebuilt standard
library and original sanitizer flags. The Linux x86-64 Ryzen 9 7950X3D
host used CPUs 12 and 13, two direct offline Cargo jobs and default
sanitizer settings. GNU time recorded 7m00.15s through build and tests.
[Exact command](command.txt) · [Environment](environment.txt) · [Process result](time.txt)

The receipt records SHA-256, size and ELF build ID for all 20 executed test
binaries and the application CLI. The instrumented GUI unit binary and CLI
both define `__asan_init`. The owned target occupied 22 GiB before cleanup.
[Runtime symbols](sanitizer-runtime-symbols.txt) · [Disk](disk-before-cleanup.txt) · [Checksums](SHA256SUMS)
