# Integrated P9 Linux CPU sanitizer replay

Linux CPU AddressSanitizer and ThreadSanitizer both passed on the clean
`12f2151c8a198b5209c8f4d506dbd17fd9aaa74f` integration candidate. The
Git tree was `002a96e90109c4517e29222c85629cd0d81b3711`. Cargo.lock
SHA-256 was `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The [receipt](receipt.json) records test counts by executable, binary
hashes and build IDs, toolchain, run window, and the exact set of CPU
integration targets. The [raw archive](raw-records.tar.gz) preserves the
sanitizer output, run script, scope selectors, and source hashes.

| Gate | Test executables | Passed | Failed | Ignored | Filtered | Process exit |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| AddressSanitizer | 19 | 119 | 0 | 0 | 0 | 0 |
| ThreadSanitizer | 19 | 119 | 0 | 0 | 0 | 0 |

The tests exercise the CPU library, CLI subprocesses, and the full GUI
Rust unit suite, including the rendered Slint comparison workspace.
Inputs come from the repository test fixtures. The classified CPU
sanitizer lane leaves physical-GPU execution and timed performance to
their existing lanes. The [original startup-channel audit](../3dc9bcf/README.md)
and [production repair replay](../production-47cc2a3/README.md) preserve
the earlier sanitizer failures and intermediate fixture results.

Reproduce from the `pancake` workspace of the exact revision on Linux
`x86_64-unknown-linux-gnu` with `rust-src` for
`nightly-2026-11-15`. The observed host was Ubuntu 24.04.5 LTS with
Linux `6.8.0-147-generic` and a Ryzen 9 7950X3D. Logical CPUs 0, 1, 2,
and 3 referred to distinct physical cores. Keep four physical cores
for comparable execution. The pinned check script generates the CPU
scope and uses the locked workspace with both sanitizers' default
diagnostics. The capture used a registry-only build cache and cleaned
each generated target after recording its provenance.

```sh
for sanitizer in address thread; do
  export CARGO_TARGET_DIR="$PWD/target/host-sanitizer-$sanitizer"
  env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS \
    CARGO_BUILD_JOBS=3 CARGO_INCREMENTAL=0 \
    timeout --signal=TERM --kill-after=10s 1200s \
    taskset --cpu-list 0,1,2,3 bash scripts/check.sh sanitizer "$sanitizer" || exit
done
```
