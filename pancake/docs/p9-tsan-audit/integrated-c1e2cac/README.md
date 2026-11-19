# Integrated Linux CPU ThreadSanitizer refresh

The pinned full workspace CPU ThreadSanitizer gate passed at
`c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb` with 146 passing tests and exit
status 0. There were zero sanitizer diagnostics and zero failed, ignored,
measured, or filtered tests. The run exercised the workspace library, CLI,
and offscreen GUI unit binaries and the same 16 CPU integration targets as
the [previous integrated replay](../integrated-287828a/README.md).
[Machine-readable receipt](receipt.json).

The source includes the bounded Hybrid fusion and RAW packing changes.
The run passed the Hybrid level and launch bound fixtures and the RAW and
tiled TIFF contracts. The GUI binary passed 18 tests, including comparison,
preference recovery, and viewer fit under the shared offscreen ownership helper.
It also passed the headless GTK panel error and missing-source reveal regressions.

The test window was 2026-11-18 20:48:25–20:54:43 SGT on the AMD Ryzen 9
7950X3D host running Ubuntu 24.04.5 LTS. The toolchain was
`nightly-2026-11-15` for `x86_64-unknown-linux-gnu`. Cargo used two jobs
on logical CPUs 20 and 21, which map to physical cores 4 and 5.
The gate used its checked-in flags, default libtest scheduling, and
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
    timeout --signal=TERM --kill-after=10s 1200s taskset --cpu-list 20,21 \
    bash scripts/check.sh sanitizer thread
```
