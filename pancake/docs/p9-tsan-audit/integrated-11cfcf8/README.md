# Exact integrated Linux CPU ThreadSanitizer

The pinned workspace ThreadSanitizer gate passed on
`11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6`. All **164 tests** passed
across **20 test executables**. The 17 CPU integration targets
include the new resident-buffer policy target. The workspace library,
CLI and 18 offscreen GUI unit tests also passed. The output records
zero failed, ignored, measured or filtered tests and no sanitizer
diagnostics. [Machine-readable receipt](receipt.json)

The run used the same unmodified check script, CPU selector and Cargo
lock as the [previous integrated replay](../integrated-0a9ae85/README.md).
It executed 2026-11-19 10:57:43–11:04:59 SGT on Ubuntu 24.04.5 LTS,
Linux 6.8.0-147-generic and Ryzen 9 7950X3D. Logical CPUs 14 and 15
are distinct physical cores. The pinned `nightly-2026-11-15` workspace
gate ran with two direct offline build jobs, default libtest scheduling
and default sanitizer options. The stock offscreen GUI unit target ran
with display variables unset.

Replay from a clean repository root at the tested revision. Create an
owned `artifacts/tsan-replay` directory, extract the archived script
there, and run `bash artifacts/tsan-replay/replay.sh`. The script
prepares the private target and invokes the unchanged
`bash scripts/check.sh sanitizer thread` entry point.
