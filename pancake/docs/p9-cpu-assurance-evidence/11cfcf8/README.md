# Integrated Linux fast and CPU dev checks for 11cfcf8

The exact clean `11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6` checkout passed
canonical `bash scripts/check.sh fast`. All 54 Python policy tests,
reviewed-source and dependency validation, formatting, all-target and
all-feature Clippy, private rustdoc, ShellCheck and actionlint completed
with exit 0. [Fast log](fast.log.gz) · [Exit status](fast.status)

Pinned Nextest 0.9.152 passed all 164 tests across 20 binaries in the CPU
dev tranche. JUnit recorded zero failures, errors or skips. A Nextest list
call using the same targets and profile found the same suite/name pairs.
The 17 classified CPU integration targets and all 162 prior test names
match the `54e3a8d` report. Both new numerical checks passed.

- Hybrid's two-frame result preserved transposition in `cpu_hybrid_commutes_with_transposition`.
- Laplacian focus selected stronger in-phase blue detail in `cpu_laplacian_prefers_in_phase_blue_detail`.

Weighted Average passed its native window and shared fusion contracts.
The CPU job tests also passed the added Similarity-aligned cancellation
check and the repeated-structure registration check against the unwarped
Weighted Average export. [CPU dev log](cpu-dev.log.gz) · [JUnit](junit.xml.gz) · [Nextest inventory](nextest-inventory.json.gz) · [Selected targets](selected-targets.txt) · [Typed receipt](receipt.json)

The gates ran 02:50:02–02:54:31 SGT on 2026-11-19. Rust 1.100.0 used logical
CPUs 0 and 1 of the Linux Ryzen 9 7950X3D host, with two direct offline Cargo
jobs. The ordinary dev tranche includes headless GUI-crate tests with desktop
endpoints unset. This receipt covers canonical fast and the selected CPU dev
profile. [Host and tools](host-tools.txt) · [Exact command](outer-command.txt) · [Runner](run-assurance.sh.txt) · [Checksums](SHA256SUMS)
