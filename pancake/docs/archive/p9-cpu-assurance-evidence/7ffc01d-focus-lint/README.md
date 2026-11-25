# CPU focus fixture lint repair

The original `8694b555666a1ddf4051e2839bbced4bcc625859` run of
`bash scripts/check.sh fast` stopped at strict Clippy with 19 numeric fallback
diagnostics and one constant `chunks_exact` diagnostic. All 20 pointed to
`tests/cpu_fusion_contract.rs`. [Original gate log](check-fast-before.log.gz).

Clean revision `7ffc01d720339f3472c74dbcaa99115b91a728ad` passed the same
gate on Linux x86-64 with pinned Rust 1.100.0. It passed 54 Python checks,
the Miri source-inventory validation, formatting, locked workspace Clippy
with all targets, all features and warnings denied, private rustdoc,
ShellCheck and actionlint. [Passing gate log](check-fast-after.log.gz).

The same clean `7ffc01d` passed all three `cpu_fusion_contract` tests,
including the repaired numerical regression. Zero tests failed, were
ignored or were filtered out. The test executable ran in 0.20 s after
the 3m25s build. [Focused test log](focused-test.log.gz).

Only the integration test changed. Explicit `f64` array types replace
literal suffix inference, the existing tolerance is explicit `f64`, and
`as_chunks::<3>` yields the same three channels per pixel after the unchanged
output-length check. A standalone probe extracted the original fixture
declarations from both Git revisions, compiled them with `rustc +1.100.0
--edition 2024`, and compared each value's Rust `to_bits` result. All 14
`f32` inputs, 21 `f64` reference values, and the `f64` tolerance matched.
[Bit comparison](fixture-bits.log).

Run the gate from `pancake/` in the fixed revision with the pinned tools.
This replay used logical CPUs 14 and 15 of an AMD Ryzen 9 7950X3D and a
shared, already-built Cargo development target. The lockfile SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.

```sh
taskset -c 14,15 nice -n 5 env PATH="/tmp/pancake-tools/bin:$PATH" \
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache \
  bash scripts/check.sh fast

taskset -c 14,15 nice -n 5 env PATH="/tmp/pancake-tools/bin:$PATH" \
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache \
  cargo +1.100.0 test --locked --package pancake --test cpu_fusion_contract \
  -- --test-threads=1
```

[Artifact checksums](SHA256SUMS).
