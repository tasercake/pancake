# Integrated Linux fast and CPU dev checks for 5b98890

The clean `5b98890d5e976d7287c354091671fff6f87d266c` checkout passed
canonical `bash scripts/check.sh fast`. All 54 Python policy tests,
Miri source-inventory and dependency validation, formatting,
workspace all-target/all-feature Clippy with warnings denied,
private rustdoc, ShellCheck and actionlint completed with exit 0.
[Full fast log](fast.log.gz) · [Exit status](fast.status)

Pinned Nextest 0.9.152 ran the normal CPU dev tranche. All **160** tests
passed across **20** binaries with zero failures, errors or skips.
The checked-in classifier selected 17 integration targets alongside
workspace libraries and binaries. The JUnit roster retains all 151
prior tests and adds nine. The new `cpu_resident_policy` target ran its
one configured-slab residency test. The sparse RAW at-limit fixture
also ran and passed. Both were verified in the actual test log and JUnit.
[Full Nextest log](cpu-dev.log.gz) · [JUnit](junit.xml.gz) ·
[Selected targets](selected-targets.txt) · [Classifier input](metadata.json.gz) ·
[Exit status](cpu-dev.status)

The two scopes ran 02:15:23–02:19:55 SGT on 2026-11-19. Rust 1.100.0
used logical CPUs 0 and 1 of the Linux Ryzen 9 7950X3D host, with
two Cargo build jobs, Cargo offline and compiler wrappers unset.
The CPU selector included existing headless GUI-crate tests under
disabled desktop display endpoints. Native desktop and device execution
have separate receipts. Relative to `870f7f9`, the changes are test
modules, integration tests, and the corresponding three source-inventory
entries. [Typed result and identities](receipt.json) ·
[Host and tool versions](host-tools.txt) · [Outer command](outer-command.txt) ·
[Exact runner](run-assurance.sh.txt) · [Checksums](SHA256SUMS)
