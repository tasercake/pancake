# Exact Linux nightly CPU branch coverage

Clean integrated application revision
`d6359e129737f1a9077fb4abc2a8f783fa1f7b38` ran the unchanged
`branches-cpu` check on Linux x86-64, AMD Ryzen 9 7950X3D,
with nightly `nightly-2026-11-15` and cargo-llvm-cov 0.9.1.
The gate exercised 127 passing CPU tests across 19 nonempty test
targets. A separate doc-test harness reported 0 tests. No lane
reported a failed or ignored test. This run launches no GUI desktop
and makes no physical GPU execution claim.
[Toolchain, source and lock hashes](provenance.txt) and [CPU target selection](selected-targets.txt).

The strict check exited 1 and printed `CPU assurance: uncovered CPU lines`.
LLVM emitted a real branch denominator. The report records these counts.

| Metric | Covered | Total | Uncovered |
| --- | ---: | ---: | ---: |
| Lines | 4,202 | 5,932 | 1,730 |
| Functions | 426 | 684 | 258 |
| Regions | 4,303 | 6,713 | 2,410 |
| Branch outcomes | 520 | 640 | 120 |

[Original test and gate log](branches-cpu.log.gz),
[exit status](branches-cpu.status), [LLVM map](branches-cpu.json.gz),
[merged nightly profile](branches-cpu.profdata.gz) and
[totals and per-file branch gaps](summary.json).

Replay from `pancake/` in an isolated clean checkout with the pinned nightly
and `llvm-tools-preview` installed. The optional registry compiler cache
reflects the command used for this capture. Test targets, source filters and
strict thresholds come from the unchanged script.

```sh
export PATH=/tmp/pancake-tools/bin:$PATH
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS AT_SPI_BUS_ADDRESS
nice -n 5 bash scripts/check.sh branches-cpu
```

The indexed profile and LLVM report use the matching nightly instrumentation.
They belong to this exact CPU run. [Artifact checksums](SHA256SUMS).
