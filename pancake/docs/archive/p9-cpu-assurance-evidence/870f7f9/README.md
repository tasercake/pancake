# Integrated Linux fast and CPU dev checks for 870f7f9

The clean `870f7f923d70b1a7b0e7fe73d5f9a4987770fdc4` checkout passed
canonical `bash scripts/check.sh fast`. All 54 Python policy tests,
Miri source-inventory and dependency policy, formatting,
workspace all-target/all-feature Clippy with warnings denied,
private rustdoc, ShellCheck and actionlint completed with exit 0.
[Full fast log](fast.log.gz) · [Exit status](fast.status)

Pinned Nextest 0.9.152 ran the normal CPU dev tranche. All **151** tests
passed across **19** binaries, with zero failures, errors or skips.
The checked-in classifier selected 16 integration targets alongside
workspace libraries and binaries. The JUnit roster retains all 148
baseline tests and adds Hybrid fine detail, depth-map focus-scale
normalization, and repeated-registration CPU export checks.
[Full Nextest log](cpu-dev.log.gz) · [JUnit](junit.xml.gz) ·
[Selected targets](selected-targets.txt) · [Classifier input](metadata.json.gz) ·
[Exit status](cpu-dev.status)

The two scopes ran 02:04:46–02:09:27 SGT on 2026-11-19. Rust 1.100.0
used logical CPUs 0 and 1 of the Linux Ryzen 9 7950X3D host, with
two Cargo build jobs, Cargo offline, compiler wrappers unset and no
desktop display endpoints. The fast gate and normal CPU dev execution
are scoped separately from native GPU, GUI, coverage and mutation
campaigns. Relative to `0a9ae85`, the changes within the executable,
test, manifest, tool and automation scope are the five recorded test
paths. [Typed result and identities](receipt.json) · [Host and tool versions](host-tools.txt) ·
[Outer command](outer-command.txt) · [Exact runner](run-assurance.sh.txt) ·
[Checksums](SHA256SUMS)
