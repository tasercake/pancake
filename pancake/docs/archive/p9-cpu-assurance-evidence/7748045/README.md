# Integrated Linux fast and CPU dev checks

Clean revision `7748045173ec8b3d4d0bb0d3ca32bd0e7328c4c4` passed the unchanged
`bash scripts/check.sh fast` gate on 2026-11-18. The 54 Python policy tests,
Miri inventory validation, formatting, locked workspace all-target/all-feature
Clippy with warnings denied, private rustdoc, ShellCheck and actionlint all
passed. [Fast log](fast.log.gz), [exit status](fast.status).

At the same revision, pinned Nextest 0.9.152 passed all 147 selected CPU dev
tests across 19 executables, including the existing headless GTK setup-error
test. The 16 classified integration targets were run alongside the workspace
libraries and binaries. The JUnit record agrees with Nextest's 147 passes,
zero failures or errors, and zero skips. No desktop session or physical
GPU ran. [Nextest log](cpu-dev.log.gz), [JUnit](junit.xml.gz),
[target selection](selected-targets.txt), [classifier input](metadata.json.gz),
[exit status](cpu-dev.status).

The AMD Ryzen 9 7950X3D host used Linux x86-64 `6.8.0-147-generic`, Rust
1.100.0 (`0ef1bc386`) and two jobs pinned to CPUs 0 and 1. Compiler wrappers
and desktop endpoints were unset. This is the dev tranche of the normal
CPU gate. [Exact commands](run-assurance.sh.txt), [checksums](SHA256SUMS).

Cargo lock SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Canonical gate SHA-256 `63ce2ccca53e1e4e73666b5bc72374d2ea5f8a25fe474e0c87ee41284d27eec7`.
CPU selector SHA-256 `044dcaefaf20b04fda64f974d464c79298c54ee4bbac234661bb204809e27a3a`.
