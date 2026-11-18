# Native-scale registration and crop checks

The bad translation largely followed a repeat in the reference itself. An
independent proxy calculation gave approximately 0.916 correlation for the
chosen cross-image fit and 0.921 for the same transform on the reference
alone. The acceptance check now requires a translation separated by at least
one scoring sample to beat that repeated-reference alternative by the same
minimum correlation advantage used against identity. The CubeCL moment
kernel and sampling grid are shared for both evaluations. Fine adjustments
below the sample spacing keep their existing confidence checks.

## Locked functional and regression evidence

The checked release used clean source `5e102447ad95c641eb773d9fdf94f2bc3757b81d`,
binary SHA-256 `3ac6abaecc1bbd5743efd2abc6879a43d4b17294514b7bc209a05227d613ccbb`
and Cargo lock SHA-256 `012cda6d161124eef8ecc3c5c6a4fed8db881e5f7606f4b9d7aee5f766f1a879`.
Rust 1.100.0 built it with `cargo build --locked --release -p pancake-cli`.
The Linux x86-64 host ran kernel 6.8.0-147-generic with AMD Ryzen 9 7950X3D CPU
and NVIDIA RTX 3090, driver 580.178.04 and PCI 0000:01:00.0. Display variables
were unset for native CLI execution.

| Check | Result | Evidence |
|---|---|---|
| Exact 2-frame T50 reproduction, Vulkan | Two exact identities, 3998×5998 RGB16 after the one-pixel margin | [Receipt](p10-registration-evidence/t50.receipt.json), [device event](p10-registration-evidence/t50.events.jsonl) |
| Nine frozen real E9 exposures, CPU | Nine exact identities, reference geometry of 337×510 RGB16 | [Receipt](p10-registration-evidence/e9-cpu.receipt.json), [event](p10-registration-evidence/e9-cpu.events.jsonl) |
| Nine frozen real E9 exposures, physical Vulkan | Nine exact identities, reference geometry of 337×510 RGB16 | [Receipt](p10-registration-evidence/e9-gpu.receipt.json), [device event](p10-registration-evidence/e9-gpu.events.jsonl) |

The separate focused `cargo test --locked --test cpu_job --test
cpu_registration_contract --test gpu_registration_contract` passed all six
tests. Locked Clippy for those three targets and the library passed using the
effective reviewed dependency policy. Runtime fingerprints and hashes refer to
the clean functional release. The functional receipts verify exact input
hashes before and after the jobs. Generated outputs were removed after decode
and hashing. Whole T50 and the frozen warm, cold, memory and equal-quality
protocol remain separate from these functional results.
[Evidence checksums](p10-registration-evidence/SHA256SUMS).

Replay the P5 selection from the repository root with the locked release
executable staged at `pancake/target/release/pancake-cli`:

The integration based on local `bc3458a` retained its IO, runtime and worker
changes. The four revised production files were re-reviewed as OS-neutral
and the unchanged Cargo lock stayed in the source inventory. The platform
inventory guard, eight policy/classifier tests and workspace format check
passed. The integrated `cargo test --locked --lib --test cpu_job --test
cpu_registration_contract` passed 24 tests. The classifier retained
`cpu_job` and `cpu_registration_contract` for CPU assurance and
`gpu_registration_contract` for physical GPU assurance. The release
receipts above retain their original tested source revision.
