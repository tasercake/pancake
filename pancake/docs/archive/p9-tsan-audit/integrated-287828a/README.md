# Integrated Linux workspace CPU ThreadSanitizer replay

The unchanged workspace CPU ThreadSanitizer gate passed on exact revision
`287828aac07ffad2252785b73a52a59fc07b3409`. All 137 tests passed. The process
exited 0 with zero sanitizer diagnostics and zero failed, ignored, or filtered
tests. It ran 16 selected CPU integration executables and the workspace
library, CLI, and GUI unit binaries. [Machine-readable receipt](receipt.json).

| Scope | Executables | Result |
| --- | ---: | --- |
| Full selected CPU workspace | 19 | 137 passed, exit 0 |
| GUI unit binary within the workspace run | 1 | 16 passed, 20.69 s |

The local test window was 2026-11-18 18:27:56–18:34:58 SGT. The host was an
AMD Ryzen 9 7950X3D on Ubuntu 24.04.5 LTS. It ran the pinned
`nightly-2026-11-15` toolchain for `x86_64-unknown-linux-gnu`, with two Cargo
jobs and logical CPUs 0–3 on separate physical cores. The Fontconfig ELF build
ID was `9ea8985291d6ca9e904a796a5767d191039d813d`. P9 built concurrently on
separate cores. The optional shared compiler-cache wrapper from the earlier
local replay was omitted because its running servers had 0–31 affinity. The
source gate used its pinned instrumentation, default libtest scheduling,
and default sanitizer options. The locked dependency file SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
[Earlier local replay](../integrated-d6359e1/README.md) and
[hosted GUI failure and targeted replays](../hosted-gui-fontconfig-d6359e1.md).

The archive includes the executed shell command, per-target outcomes, selected
CPU test names, source and toolchain details, and SHA-256 and ELF build IDs for
all launched test binaries plus the ordinary CLI used by integration tests.
After fingerprinting the binaries, `cargo clean` removed 7,491 files and
20.5 GiB from the owned sanitizer target. The worktree was clean at the end of
the replay. This receipt qualifies the local CPU TSan gate at the exact source
above. Hosted verification is tracked separately. [Checksums](SHA256SUMS).

Run from the `pancake` workspace of the sealed source, with a disposable target
and the same available CPU topology.

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS
unset PANCAKE_RUNTIME PANCAKE_TEST_GPU_RUNTIME CONFORMANCE_EVENTS_PATH
unset RUST_TEST_THREADS RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$PWD/target/isolated-cpu-tsan"
export PANCAKE_COMPUTE_CACHE_DIR="$CARGO_TARGET_DIR/compute-cache"
env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS -u UBSAN_OPTIONS \
    timeout --signal=TERM --kill-after=10s 1200s taskset --cpu-list 0,1,2,3 \
    bash scripts/check.sh sanitizer thread
```
