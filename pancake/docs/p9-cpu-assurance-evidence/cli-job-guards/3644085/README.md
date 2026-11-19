# Headless I/O and job guard assurance

The clean `2143eda` baseline and tests-only `3644085` candidate ran the
same unchanged CPU nightly branch gate on Linux x86-64. The full report
retains the clean checkout, toolchain, lock and check script hashes. The
after lane passed 132 tests, up from 127. Both lanes had zero failing or
ignored tests. No physical GPU or desktop lane ran.
[Provenance](provenance.txt), [selected CPU targets](selected-targets.txt),
[before test log](before.log.gz), [after test log](after.log.gz).

The additions run the actual headless CLI with an anonymous OS pipe whose
reader has already been closed. Lost stdout progress exits 1 and preserves
the real CPU export and completion event. Lost stderr delivery of a JPEG
depth warning also exits 1, with the complete 8-bit JPEG and final progress
available. Neither test adds delays or changes behavior. The CPU job tests
reject a mismatched controller and prevent a duplicate stack, preserve a
completed export if a consumed source moves before observer append, and
retain a directory's sentinel bytes when an export destination is invalid.

Both strict gates exited 1 for uncovered CPU lines. The production source,
target selection, file coverage denominators and thresholds were unchanged.

| Metric | Covered before | Covered after | Total |
| --- | ---: | ---: | ---: |
| Lines | 4,202 | 4,206 | 5,932 |
| Functions | 426 | 429 | 684 |
| Regions | 4,303 | 4,310 | 6,713 |
| Branch outcomes | 520 | 521 | 640 |

The CLI main reached 16/16 branch outcomes and gained three lines, two
functions and five regions. The observer gained one line, one function and
two regions. The aggregate still has 1,726 uncovered lines and 119
uncovered branch outcomes. [Exact comparison](summary.json),
[before LLVM map](before.json.gz), [after LLVM map](after.json.gz),
[before indexed profile](before.profdata.gz), [after indexed profile](after.profdata.gz).

Run from `pancake/` in separate clean checkouts at each revision. Use the
pinned `nightly-2026-11-15` with LLVM tools and cargo-llvm-cov 0.9.1.

```sh
export PATH=/tmp/pancake-tools/bin:$PATH
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS AT_SPI_BUS_ADDRESS
nice -n 5 bash scripts/check.sh branches-cpu
```

The tests-only checkout also passed workspace formatting and focused
`cargo clippy --locked -p pancake -p pancake-cli --test cpu_job
--test cpu_runtime_options --all-features -- -D warnings`, using the
standard dependency-policy-generated Clippy configuration.
[Clippy log](clippy.log.gz) and [checksums](SHA256SUMS).
