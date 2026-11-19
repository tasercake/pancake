# Integrated Linux fast and CPU dev checks for 54e3a8d

The exact clean `54e3a8d60025cf77daa655e5e66781a82f515f7c` checkout passed
canonical `bash scripts/check.sh fast`. All 54 Python policy tests,
reviewed-source and dependency validation, formatting, workspace
all-target/all-feature Clippy with warnings denied, private rustdoc,
ShellCheck and actionlint completed with exit 0. [Fast log](fast.log.gz) · [Exit status](fast.status)

Pinned Nextest 0.9.152 passed all 162 tests across 20 binaries in the CPU
dev tranche. JUnit recorded zero failures, errors or skips. A separate
Nextest list call using identical selection and profile found the same
162 suite/name pairs. The 17 classified CPU integration targets match the
prior `5b98890` list. All 160 previous JUnit tests remain, with two additions
confirmed by PASS lines in the native log.

- `cpu_registration_requires_detail_in_the_reference` completed in 0.241 s.
- `registration::spectral::tests::weak_spectral_peak_abstains_before_displacement` completed in 0.006 s.

[CPU dev log](cpu-dev.log.gz) · [JUnit](junit.xml.gz) · [Nextest inventory](nextest-inventory.json.gz) · [Selected targets](selected-targets.txt) · [Typed result](receipt.json)

The gates ran 02:28:06–02:32:42 SGT on 2026-11-19. Rust 1.100.0 used logical
CPUs 0 and 1 of the Linux Ryzen 9 7950X3D host, with two direct offline Cargo
jobs. The ordinary dev tranche includes headless GUI-crate tests with desktop
endpoints unset. This receipt covers canonical fast and the selected CPU
dev profile. [Host and tools](host-tools.txt) · [Exact command](outer-command.txt) · [Runner](run-assurance.sh.txt) · [Checksums](SHA256SUMS)
