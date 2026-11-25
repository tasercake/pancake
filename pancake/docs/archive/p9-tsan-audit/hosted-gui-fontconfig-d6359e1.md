# Hosted GUI fontconfig sanitizer failure

The hosted `ubuntu-24.04` ThreadSanitizer job for
`d6359e129737f1a9077fb4abc2a8f783fa1f7b38` reported two data races and exited
66. The Rust harness reported all 127 tests passing, including 15 GUI tests.
Both reports involve concurrent offscreen Slint platform setup from
`comparison_clips_source_to_left_of_split` and
`preference_callback_reports_storage_errors_and_recovers`. The reported
interceptors are `strcmp` and `pthread_mutex_lock`, with native accesses
through `libfontconfig.so.1`.
[Hosted job](https://github.com/tasercake/focus-stacking/actions/runs/44277150143/job/134708073132).

The source chain reaches Fontique 0.12's Linux font configuration through
Slint 1.19's per-thread `SlintContext::new`. Fontique's configuration also
calls `FcConfigDestroy` when dropped. Slint retains that context in thread-local
storage after `set_platform` and drops it at thread termination. The normal
application creates its graphical platform on its main thread. The two unit
tests had each initialized a separate graphical platform on parallel Rust test
threads. The compile-only smoke test uses the interpreter's default resource
mode and does not create a Slint platform.

| Environment | Operating system | Fontconfig ELF build ID | GUI test time | Gate |
| --- | --- | --- | ---: | --- |
| Hosted image `20261111.370.1` | Ubuntu 24.04.5 | `9ea8985291d6ca9e904a796a5767d191039d813d` | 167.00 s | Two warnings, exit 66 |
| Local `d6359e1` replay, four physical cores | Ubuntu 24.04.5 | `9ea8985291d6ca9e904a796a5767d191039d813d` on the present host | 6.75 s | Exit 0 |

The shared Fontconfig build identity places the investigation on overlapping
sessions and execution conditions. The hosted log identifies its runner image.
The local receipt identifies the Ryzen 9 7950X3D and the explicit four-core
affinity. A difference in font inventory or initialization timing remains a
hypothesis. [Previous local replay](integrated-d6359e1/README.md).

## Scoped ownership and replay

Each offscreen test runs in a fresh owner thread. Only these GUI fixtures
acquire the test mutex. The caller holds it through `JoinHandle::join`, which
waits for thread-local destructors after the test body returns. This orders
native discovery, rendering, and Fontique teardown before the next fixture
begins. Each test still runs the existing framebuffer, input, callback, and
weak-reference assertions. All other tests retain ordinary libtest scheduling.
The change touches test ownership and its reviewed source digest.

| Targeted pinned ThreadSanitizer replay | GUI tests passed | Test time | Diagnostics | Process exit |
| --- | ---: | ---: | ---: | ---: |
| Two fixtures, `d8dea27` | 15 | 12.33 s | 0 | 0 |
| Three fixtures with merged viewer, `744c986` | 16 | 19.81 s | 0 | 0 |

The first run built with two jobs in a private target directory. The second
reused that target and verified the added `viewer_fit_updates_without_decorative_animation`
fixture under the same ownership helper. It covers the viewer already merged
at `b192f7d`, including its real rendered image and keyboard actions.
Formatting, reviewed-source validation, and all six policy tests passed on
`744c986`. The archived logs retain the exact command, source and lock hashes,
pinned compiler, Fontconfig build ID, binary fingerprints, and timings.
[Receipt](hosted-gui-fontconfig-d6359e1/receipt.json),
[raw records](hosted-gui-fontconfig-d6359e1/raw-records.tar.gz), and
[checksums](hosted-gui-fontconfig-d6359e1/SHA256SUMS).

## Replay the scoped check

Run from `pancake` with the pinned nightly and normal libtest concurrency.
Choose a disposable build target and retain the process exit as well as the
Rust assertion count.

```sh
source scripts/tool-versions.sh
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS
unset TSAN_OPTIONS ASAN_OPTIONS LSAN_OPTIONS UBSAN_OPTIONS RUST_TEST_THREADS
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS RUSTC_WRAPPER RUSTC_WORKSPACE_WRAPPER
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_TARGET_DIR="$PWD/target/gui-tsan"
export RUSTFLAGS='-Dwarnings -Zsanitizer=thread'
cargo +"$PANCAKE_NIGHTLY" test -Zbuild-std --locked \
  --target x86_64-unknown-linux-gnu -p pancake-gui --all-features --bin pancake-gui \
  -- --include-ignored
```

The full workspace gate uses `bash scripts/check.sh sanitizer thread`.
The hosted job and workspace-wide sanitizer status on the integrated source
remain pending.
