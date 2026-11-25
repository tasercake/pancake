# Integrated Linux CPU ThreadSanitizer refresh

The pinned workspace CPU ThreadSanitizer gate passed at
`0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` with 148 passing tests and exit
status 0. There were no sanitizer diagnostics or failed, ignored, measured,
or filtered tests. The run exercised 19 executables, including the workspace
library, CLI and offscreen GUI and the same 16 CPU integration targets as the
[previous integrated replay](../integrated-c1e2cac/README.md).
[Machine-readable receipt](receipt.json).

The selected tests exercised typed RAW working values, packed RGB16
quantization and orientation, tiled TIFF decode, retained crop,
and worker and observer behavior. All 18 GUI tests passed, including
comparison, preference recovery and viewer fit under the shared offscreen
ownership helper, the headless GTK panel error, and missing-source reveal.

The test window was 2026-11-19 01:00:11–01:06:29 SGT on an AMD Ryzen 9
7950X3D host running Ubuntu 24.04.5 LTS. The toolchain was
`nightly-2026-11-15` for `x86_64-unknown-linux-gnu`. Cargo used two direct
jobs on logical CPUs 2 and 3, which map to distinct physical cores.
The gate used the checked-in flags, default libtest scheduling, and
default sanitizer options under an owned Cargo target.

Run from the `pancake` workspace of the sealed source with a disposable
target. The archived replay script records the complete setup.

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS
unset PANCAKE_RUNTIME PANCAKE_TEST_GPU_RUNTIME CONFORMANCE_EVENTS_PATH
unset RUST_TEST_THREADS RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
unset RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$PWD/target/isolated-cpu-tsan"
export PANCAKE_COMPUTE_CACHE_DIR="$CARGO_TARGET_DIR/compute-cache"
env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS -u UBSAN_OPTIONS \
    timeout --signal=TERM --kill-after=10s 1200s taskset --cpu-list 2,3 \
    bash scripts/check.sh sanitizer thread
```
