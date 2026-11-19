# Integrated Linux nightly CPU branch coverage

The clean `287828aac07ffad2252785b73a52a59fc07b3409` source ran the unchanged
`branches-cpu` gate. The Linux x86-64 Ryzen 9 7950X3D run used
`nightly-2026-11-15` with LLVM tools and cargo-llvm-cov 0.9.1. It passed all
137 CPU tests across 19 nonempty targets, with zero failures or ignored
tests. A separate doctest harness reported zero tests. No desktop or
physical GPU run supplied coverage. [Provenance](provenance.txt),
[target selection](selected-targets.txt) and [original test log](branches-cpu.log.gz).

The comparator printed `CPU assurance: uncovered CPU lines` and the strict
command exited 1. The shell gate and comparator retain the zero-uncovered
requirement for lines, functions, regions and branches. The gate scripts,
lockfile, toolchain pin and selected targets match the exact `3644085`
run. The source and measured denominators changed.
[Previous exact run](../cli-job-guards/3644085/README.md) and
[intervening source scope](scope-from-3644085.txt).

| Metric | Covered / total at 3644085 | Covered / total at 287828a | Change in uncovered |
| --- | ---: | ---: | ---: |
| Lines | 4,206 / 5,932 | 4,466 / 6,250 | +58 |
| Functions | 429 / 684 | 454 / 728 | +19 |
| Regions | 4,310 / 6,713 | 4,551 / 7,025 | +71 |
| Branch outcomes | 521 / 640 | 552 / 686 | +15 |

Among files with mappings in both runs, covered counts fell in two changed
files. `src/image_io/raw.rs` fell from 8 to 7 covered branch outcomes out of
10. `src/registration/estimate.rs` fell from 270/283 to 265/295 lines,
21/22 to 20/22 functions and 318/359 to 306/353 regions. The newly mapped
tiled TIFF decoder has 37 uncovered lines and ten uncovered branch
outcomes. [Per-file comparison](summary.json).

Replay from `pancake/` in a clean checkout of the measured revision, with
the pinned Rust nightly, LLVM tools and installed cargo-llvm-cov.

```sh
export PATH=/tmp/pancake-tools/bin:$PATH
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS AT_SPI_BUS_ADDRESS
nice -n 5 bash scripts/check.sh branches-cpu
```

[LLVM source map](branches-cpu.json.gz), [indexed profile](branches-cpu.profdata.gz),
[gate exit status](branches-cpu.status) and [artifact checksums](SHA256SUMS).
