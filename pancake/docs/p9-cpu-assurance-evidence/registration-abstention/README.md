# Native CPU registration abstention

The exact `0a9ae85` nightly map had zero true executions and 14 false
executions at `src/registration/estimate.rs:220`, the repeated-structure
ambiguity guard. The unchanged gate exited 1, with 148 passing tests.
[Test coverage before the addition](../0a9ae85-nightly-branches/README.md).

Clean `d9ea867b4d1b126e576354efccbff3656d6150b0` adds one test in
`tests/cpu_job.rs`. It writes two 112×96 RGB16 PNG files with a 12-pixel
horizontal motif and a three-pixel relative phase shift. An independent
float64 check of the smoothed interior obtains Pearson scores of 1 at
+3 and −9 pixels and −0.02116 at identity. The actual test uses the normal
`job::run` CPU path. The translation run must preserve an identity source
map, complete with two identity sequential maps, and produce the same
full-size decoded output samples as a matched alignment-off control.
The original source files keep their exact bytes. The existing positive
subpixel and wide translation tests also ran.

The unchanged pinned nightly gate passed 149 of 149 tests. The new fixture
made the true edge at line 220 execute once and retained 14 false hits.
LLVM credited one more line, region, and branch outcome. The gate still
exited 1 on uncovered CPU lines. Scoped stable Clippy with the generated
reviewed dependency configuration exited 0. No desktop or GPU was used.
[Run log](branches-cpu.log.gz), [raw map](branches-cpu.json.gz),
[indexed profile](branches-cpu.profdata.gz), [summary](summary.json),
[Clippy log](clippy.log.gz), [checksums](SHA256SUMS).

| Metric | `0a9ae85` | `d9ea867` |
| --- | ---: | ---: |
| Lines | 4,730 / 6,516 | 4,731 / 6,516 |
| Functions | 485 / 760 | 485 / 760 |
| Regions | 4,849 / 7,328 | 4,850 / 7,328 |
| Branch outcomes | 577 / 764 | 578 / 764 |
| Estimator branch outcomes | 44 / 64 | 45 / 64 |

The two initial test builds stopped before execution. They exposed an
unsupported array-repeat expression in `serde_json::json!` and an unused
control result under strict lints. Both were corrected in the test while
retaining the same fixture. [First build](first-compile.log.gz),
[second build](second-compile.log.gz).

Replay from `pancake/` with Rust 1.100.0, pinned `nightly-2026-11-15` and
cargo-llvm-cov 0.9.1. The Linux x86-64 Ryzen 9 7950X3D ran on CPUs 0 and 1,
with two build jobs, Cargo offline and compiler wrappers unset.
[Commands and input pins](command.txt), [timing](branches-cpu.time).
