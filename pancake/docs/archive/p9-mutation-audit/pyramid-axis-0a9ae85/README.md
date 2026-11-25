# Hybrid pyramid axis-symmetry regression

Test-only commit `e570aba74bb0bcb676660d4ada05b016b068ce36` extends the exact
`0a9ae85` native CPU test. It fuses two phase-opposed 8×8 RGB stripe
frames with public Hybrid, then fuses both inputs transposed. The output
must be genuinely fused and transpose back within one RGB16 code. This
checks orientation symmetry through reduction and reconstruction using
real nonuniform focus and tone. A single input could cancel a reduction
error when its unchanged detail residual is reconstructed.

Four exact saved mutants built and failed on the same public witness.

| Saved mutation | Observed maximum normalized sample mismatch |
| --- | ---: |
| Row decoding `123:21` | 0.158125 |
| Top-right row offset `177:32` | 0.050313 |
| Bottom-right row offset `179:35` | 0.025156 |
| Bottom interpolation `183:30` | 0.028750 |

Restored stock passed all five focused native CPU tests and configured
strict Clippy. Rust 1.100.0 ran directly, locked and offline on logical
CPUs 14 and 15 with two build jobs. The archive retains the exact saved
diffs and native stdout, exit files, and reproducible commands. The
typed receipt records custody and policy hashes. This proof covers the
targeted saved mutations rather than a canonical shard rerun.
[Typed receipt](receipt.json) · [Raw evidence](raw-native.tar.gz) ·
[Checksums](SHA256SUMS)
