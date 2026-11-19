# Integrated Linux fast and selected CPU dev tests

Clean revision `25cc498b08a9137af25a5aef615ed5734f27704e` passed canonical
`bash scripts/check.sh fast` on 2026-11-18. The unchanged gate passed all 54
Python tests, Miri source-inventory and dependency validation, formatting,
locked all-target/all-feature workspace Clippy with warnings denied,
private rustdoc, ShellCheck and actionlint. The command exited 0.
[Original fast log](fast.log.gz), [exit status](fast.status).

The same checkout passed all 147 tests across 19 binaries in the normal
CPU dev tranche. It selected 16 integration targets with the repository's
CPU classifier and included library and binary tests. Nextest 0.9.152
reported zero skips, and the command exited 0. It ran the existing headless
GUI-crate tests without a native desktop session or a physical GPU lane.
[Target selection](selected-targets.txt), [original Nextest log](cpu-dev.log.gz),
[JUnit](junit.xml.gz), [exit status](cpu-dev.status).

The Linux x86-64 `6.8.0-147-generic` host used an AMD Ryzen 9 7950X3D and
Rust 1.100.0 (`0ef1bc386`), with two build workers pinned to CPUs 6 and 7.
Display and desktop bus endpoints were removed from the environment.
[Exact commands](run-assurance.sh.txt), [artifact checksums](SHA256SUMS).

The Cargo lockfile SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
