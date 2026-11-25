# Hybrid fine-band mutation regression

The saved one-expression mutant changes `index < 2` to `index == 2` at
`fusion/advanced/mod.rs:280:70`. The mutated native test binary built,
then the targeted test failed with **0.160** retained power at each
period. After restoring the exact production source, all **5** tests in
`cpu_advanced_fusion` passed. Focused Clippy with the checked-in dependency
policy and formatting checks passed as well. The actual native runs
used pinned Rust 1.100.0, the locked offline graph and two jobs on logical
CPUs 4 and 5 of the Linux Ryzen 9 7950X3D host.

The archive contains the exact mutation patch, original native failure and
stock output, command provenance, lint result and exit codes. The receipt
binds every archived file to the original local checksum ledger and
retains the exact input policy file hashes. This evidence covers the
single specified mutation using the focused native test. The canonical
shard has its own independent receipt.
[Typed receipt](receipt.json) · [Exact raw witnesses](raw-native.tar.gz) ·
[Checksums](SHA256SUMS)
