# P9 ThreadSanitizer startup-channel audit

The original Linux CPU gate at `3dc9bcf3c30605badea2f7b2ee2840075507b6c3`
failed in the real CLI subprocess spawned by
`relative_glob_processes_the_selected_sources`. With four physical cores
selected, 74 tests passed, one failed, and 22 remained unrun. The raw report
identifies the executable by ELF build ID and reports a free on the main
thread at `oneshot::Receiver::recv` line 278 against an atomic write at
`oneshot::Sender::send` line 104 on the CubeCL device runner. The earlier
unconstrained run stalled and is preserved separately.

The sender's acquire-release RMW publishes `MESSAGE` from `EMPTY` and
finishes its channel accesses. The reported receiver branch observes that
publication with a relaxed atomic load and acquires it through a standalone
fence before taking the message and freeing the allocation. Rust's documented
atomic-to-fence rule supplies the happens-before edge. ThreadSanitizer does
not recognize that standalone fence, so the reported free and atomic RMW
appear unordered to the tool. The pinned Rust allocator uses an acquire
load for the corresponding instrumentation gap in `Arc`.
[Rust fence semantics](https://doc.rust-lang.org/std/sync/atomic/fn.fence.html#atomic---fence)
and [Rust ThreadSanitizer support](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html#threadsanitizer).

| Experiment | Result |
|---|---|
| Unmodified `oneshot 0.2.1`, standalone probe | Exit 66. The same receiver line 278 and sender line 104 appeared without Pancake, CubeCL, or LLVM JIT in the executable. |
| Same probe and same package version with strengthened receive ordering | Exit 0 with an empty TSan log. |
| Exact formerly failing CLI test on the experimental graph | Passed, 1 of 1. |
| Full CPU sanitizer runner on the experimental graph | Passed, 97 of 97, with zero warnings, ignored cases, or filtered tests. |

The experimental dependency changes only blocking `Receiver::recv`. It
acquires the initial state load and the receive-registration exchange,
replacing four standalone acquire fences with instrumentable operations.
The exchange still releases the waker, and the parked receiver still
acquires the terminal state. The package version remains 0.2.1. The complete
archive contains the source diff, probe source, both lockfiles, exact
application graph diff, commands, and original and experimental raw reports.
[Receipt](receipt.json) and [raw archive](raw-records.tar.gz).

## Reproduce the comparison

Use a disposable checkout of `3dc9bcf` with the pinned nightly and its
`rust-src` component. Extract the raw archive into the checkout's
`artifacts/oneshot-tsan/`. Run the following from that directory for the
unmodified dependency and then for the candidate.

```sh
cp original.Cargo.toml Cargo.toml
cp original.Cargo.lock Cargo.lock

env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS \
  RUSTFLAGS='-Dwarnings -Zsanitizer=thread' \
  taskset --cpu-list 0,1,2,3 cargo +nightly-2026-11-15 build -Zbuild-std \
  --locked --offline --target x86_64-unknown-linux-gnu -j 1
env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS \
  timeout --signal=TERM --kill-after=5s 25s taskset --cpu-list 0,1,2,3 \
  target/x86_64-unknown-linux-gnu/debug/pancake-oneshot-tsan-probe
```

Verify the cached `oneshot-0.2.1.crate` against the receipt. Extract it to
`vendor/oneshot` with the original archive's top-level directory stripped,
apply `blocking-receive-acquire.patch` there with `patch -p1`, and copy
`candidate.Cargo.toml` and `candidate.Cargo.lock` to the active filenames.
Repeat the two commands above. The probe's separate relaxed readiness marker
selects the observed x86 path without supplying cross-thread synchronization.
The join occurs after the receive.

To replay the full experimental gate, apply `experimental-app-graph.patch`
from the application repository root after preparing that same dependency
source. The scratch graph's lock changes only the origin of `oneshot 0.2.1`.
Run `artifacts/oneshot-tsan/replay-app.sh` with a dedicated `CARGO_TARGET_DIR`
from the repository root. The script verifies the experimental manifest and
lock hashes before running the focused CLI test and the full CPU sanitizer
gate. The recorded affinity uses CPU IDs 0–3 on the qualified host.

## Integration boundary
