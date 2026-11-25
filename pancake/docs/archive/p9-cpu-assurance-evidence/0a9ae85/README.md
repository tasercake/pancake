# Integrated Linux fast and CPU dev checks after the RAW change

Clean revision `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` passed the unchanged
`bash scripts/check.sh fast` gate on 2026-11-19. The 54 Python policy tests,
Miri source-inventory validation, formatting, locked workspace all-target
and all-feature Clippy with warnings denied, private rustdoc, ShellCheck and
actionlint all passed. [Fast log](fast.log.gz), [exit status](fast.status).

At the same revision, pinned Nextest 0.9.152 passed all 148 selected CPU dev
tests across 19 executables. The JUnit record has 148 test cases with zero
failures, errors or skips. The 16 classified integration targets ran alongside
the workspace libraries and binaries. The new typed RAW allocation and sample
regression, Bayer decode fixtures and existing headless GTK probe ran. No
physical GPU or desktop session ran. [Nextest log](cpu-dev.log.gz),
[JUnit](junit.xml.gz), [selected targets](selected-targets.txt),
[classifier input](metadata.json.gz), [exit status](cpu-dev.status).

The AMD Ryzen 9 7950X3D host used Linux x86-64 `6.8.0-147-generic`, Rust
1.100.0 (`0ef1bc386`) and two jobs pinned to CPUs 0 and 1. Cargo ran offline
with compiler wrappers and desktop endpoints unset. The CPU result is
limited to the normal dev tranche. [Exact commands](run-assurance.sh.txt),
[checksums](SHA256SUMS).

Cargo lock SHA-256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Canonical gate SHA-256 `63ce2ccca53e1e4e73666b5bc72374d2ea5f8a25fe474e0c87ee41284d27eec7`.
CPU selector SHA-256 `044dcaefaf20b04fda64f974d464c79298c54ee4bbac234661bb204809e27a3a`.
