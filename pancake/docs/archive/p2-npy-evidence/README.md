# P2 NumPy metadata and sample decoding

On 2026-11-18, the production decode path was exercised with test-owned
`.npy` files on Ubuntu 24.04.5 x86-64 and the pinned Rust 1.100.0 toolchain.
The base revision was `df3171bca65c7b5c412d0c91fcac7158f98f2be0` and
`Cargo.lock` SHA-256 was
`3ddbc2bdae9eff3e8a5284e09d614684a1145fb384a46c59ad307acf17ea34bf`.
The fixed code and fixtures are in the same commit as this receipt.

The test independently constructs standard 64-byte-aligned extended headers.
NumPy 2.5.4 `numpy.lib.format.read_array(allow_pickle=False)` accepted the
reordered version 2 unsigned sixteen-bit fixture and the version 3 float
fixture without trailing commas. The latter preserves a nontrivial f32 bit
pattern and finite HDR values. Other fixtures exercise float code scaling,
malformed metadata, header-size limits, partial reads and nonfinite samples.
All temporary inputs were scoped to each test's temporary directory.

| Command, from `pancake/` | Result |
|---|---|
| `cargo test --locked -p pancake --test cpu_npy`, before the source change | Exit 101. Two tests rejected valid scalar syntax or reported the wrong invalid field. [Red log](red.log.gz) |
| `cargo test --locked -p pancake --test cpu_npy`, after the source change | Exit 0. Three tests passed. [Green log](green.log.gz) |
| `PATH=/tmp/pancake-tools/bin:$PATH bash scripts/check.sh fast` | Exit 0. Static, lint, rustdoc and workflow gates passed. [Gate log](fast.log.gz) |
