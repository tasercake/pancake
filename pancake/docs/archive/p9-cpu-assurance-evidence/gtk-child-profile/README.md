# Headless GTK child coverage profile

The existing headless GTK regression passed at both clean source revisions
on 2026-11-18. The test re-executes its own Rust binary and checks the actual
GTK initialization error without opening a desktop. The original
`e81079dee4078d7a1149da0f5950aab8c89f8d44` child exited 0 but dropped the
standard `LLVM_PROFILE_FILE` when clearing its environment. Only the parent
wrote a raw profile. The ordinary exporter recorded zero hits in both
the child assertion and the native error path. [Original test](red-test.log.gz),
[process trace](red-exec.log), [original panel counts](red-counts.json).

Test-only revision `4f50250df9085e430edb61e2b4e100a9378fced0` forwards that
one compiler profile variable while retaining the original display and bus
isolation. The only other change is the reviewed-source digest for the test
file. The parent and child each wrote a raw profile at their own observed
PIDs. The pinned `llvm-profdata show --binary-ids` matched both files to
the same traced test ELF. The existing exporter hit lines 27, 64, 65 and 66
once each, including the error closure in native `gtk::init` setup.
Focused formatting and Miri source-inventory validation also passed.
[Fixed test](green-test.log.gz), [process trace](green-exec.log),
[ELF, PID, and profile receipt](green-counts.json).

| Scoped stable result for `panel/linux.rs` | Original | Fixed |
| --- | ---: | ---: |
| Lines | 24 / 174 | 59 / 176 |
| Functions | 2 / 14 | 8 / 15 |
| Regions | 29 / 194 | 64 / 197 |

The file counts include the existing test child and the new test-launch
line. The workload was one filtered unit test. No strict full-matrix or
branch verdict is claimed. [Original map](red-map.json.gz),
[fixed map](green-map.json.gz), [artifact checksums](SHA256SUMS).

The Linux x86-64 Ryzen 9 7950X3D used Rust 1.100.0 and cargo-llvm-cov 0.9.1
with two build jobs. Both test launchers used CPUs 0 and 1. Some original
dependency cache misses ran on CPUs 12 and 13 through the shared compiler
cache daemon. The fixed invocation disabled that wrapper. It stayed on
CPUs 0 and 1. No physical GPU or real desktop session ran.

From `pancake/`, for each clean revision set `CARGO_BUILD_JOBS=2`,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, and the pinned tools on
`PATH`. Use an owned target and unset display, bus, and profile environment
variables before Cargo selects its standard profile pattern. The test
runner was `/usr/bin/strace -f -qq -e trace=execve -o <trace>` via
`CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER`. The original used
`--no-report`; the fixed run used `--no-clean --json --output-path <initial>`
after `cargo llvm-cov clean --profraw-only` removed all original profiles
from the owned target. The same separate exporter produced both maps.

```sh
cargo llvm-cov --locked --all-features -p pancake-gui --bin pancake-gui \
  --no-report -- --exact panel::linux::tests::unavailable_display_rejects_native_panel
cargo llvm-cov report --workspace --include-build-script --json --output-path "$map"
```
