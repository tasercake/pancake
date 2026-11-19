# Integrated Linux fast gate after native mutation regressions

The clean `7551f73dbe5132f7e9723a9ac4b0c9144504a27a` checkout passed the
unchanged `bash scripts/check.sh fast` gate on 2026-11-18. All 54 Python
tests, Miri source-inventory and dependency validation, formatting, locked
all-target/all-feature workspace Clippy with warnings denied, private
rustdoc, ShellCheck, and actionlint passed. The gate exited 0 on Linux
`6.8.0-147-generic` x86-64 with Rust `1.100.0 (0ef1bc386 2026-11-09)`.
[Original log](fast.log.gz), [artifact checksum](SHA256SUMS).

Both prior native archives matched their recorded SHA-256 digests. Their
`mutants.out/outcomes.json` records contain the separate caught mutant
results for `prepare_thumbnail`, `panel::linux::build`, and
`desktop_menu::reveal`. That evidence belongs to the archived test
revisions. This replay ran the fast gate on the integrated checkout.
[Panorama receipt](../../p9-mutation-audit/2654813-panorama-budget/README.md),
[panel and reveal receipt](../../p9-mutation-audit/native-panel-and-reveal/README.md).

The host used logical CPUs 12 and 13 on the AMD Ryzen 9 7950X3D with two
Cargo jobs and the existing development target. From `pancake/`, run
with the pinned tools and an equivalent Cargo target.

```sh
taskset -c 12,13 nice -n 5 env PATH="/tmp/pancake-tools/bin:$PATH" \
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache \
  bash scripts/check.sh fast
```

Lockfile SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Gate script SHA-256 `63ce2ccca53e1e4e73666b5bc72374d2ea5f8a25fe474e0c87ee41284d27eec7`.
Original log SHA-256 `21def6ede58eb44419762c12898689a437c06340a59351e83bba5fd0efd980e1`.
