# Integrated Linux CPU AddressSanitizer

The clean product revision `287828aac07ffad2252785b73a52a59fc07b3409`
passed the unchanged `bash scripts/check.sh sanitizer address` gate.
All 137 tests passed across 19 result groups. The exit status was 0 with
zero failed, ignored or filtered tests and no address or leak sanitizer
diagnostics. The software-rendered GUI unit group passed 16 tests,
including comparison rendering, preference error recovery and viewer fit.
[Raw gate log](sanitizer-address.log.gz) [Accounting and executable hashes](receipt.json)

The gate used pinned nightly `nightly-2026-11-15`, the locked all-feature
workspace, Rust's rebuilt standard library and the normal sanitizer
flags supplied by the gate. It ran on Linux x86-64 with an AMD Ryzen 9
7950X3D, CPUs 12–15 and 28–31, two Cargo build jobs, ordinary Rust test
parallelism and no sanitizer or allocator overrides. The display and
session-bus variables were unset. The CPU affinity was shared with the
separate package build, and the thread sanitizer lane used a separate
CPU mask. GNU time recorded 7m17.90s including the fresh build and tests.
[Command](command.txt) [Environment](environment.txt) [Process exit and counters](time.txt)

The receipt records SHA-256, size and ELF build ID for all 19 executed
test binaries and the instrumented application CLI. The CLI and GUI
unit binary define the sanitizer runtime entry point.
[Selected integration targets](selected-targets.txt)
[Runtime symbols](sanitizer-runtime-symbols.txt)
