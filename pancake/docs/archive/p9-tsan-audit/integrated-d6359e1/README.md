# Integrated P9 Linux CPU sanitizer replay

The Linux CPU AddressSanitizer and ThreadSanitizer gates passed on the clean
`d6359e129737f1a9077fb4abc2a8f783fa1f7b38` release candidate. The Git tree was
`e7412552ba9be952fa8972bccce510af13680b3c` and the Cargo.lock SHA-256 was
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.

| Gate | Executable targets | Passed | Failed | Ignored | Filtered | Process exit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| AddressSanitizer | 19 | 127 | 0 | 0 | 0 | 0 |
| ThreadSanitizer | 19 | 127 | 0 | 0 | 0 | 0 |

The host was Ubuntu 24.04.5 LTS with Linux `6.8.0-147-generic` and a Ryzen 9
7950X3D. Logical CPUs 0–3 mapped to four distinct physical cores. The run
used `nightly-2026-11-15` with `rust-src` and LLVM 23.1.5, a registry-only
build cache, and two build jobs. The workspace’s unchanged sanitizer gate
used default diagnostic settings and the locked dependency graph. Both
generated sanitizer targets were removed after their binaries were recorded.

Run from the `pancake` workspace on Linux `x86_64-unknown-linux-gnu` with the
pinned toolchain. Adjust the CPU IDs to match four distinct physical cores on
the test host.

```sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS
unset PANCAKE_RUNTIME PANCAKE_TEST_GPU_RUNTIME CONFORMANCE_EVENTS_PATH
for sanitizer in address thread; do
  export CARGO_TARGET_DIR="$PWD/target/host-sanitizer-$sanitizer"
  export PANCAKE_COMPUTE_CACHE_DIR="$CARGO_TARGET_DIR/compute-cache"
  env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS -u UBSAN_OPTIONS \
    CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 \
    timeout --signal=TERM --kill-after=10s 1200s \
    taskset --cpu-list 0,1,2,3 bash scripts/check.sh sanitizer "$sanitizer"
done
```

The [previous integrated replay](../integrated-12f2151/README.md) and
[startup-channel audit](../3dc9bcf/README.md) retain the earlier sanitizer
history.
