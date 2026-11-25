# Near-equal distant registration peak mutation

The original `0a9ae85` mutation shard recorded `MissedMutant` for replacing
`score - other` with `score + other` at `src/registration/estimate.rs:362:85`.
The typed roster diff and the saved diff matched exactly, with SHA-256
`c8b2076ebebf79d3ab7bb30d14644db583ba40a2797d1e719365d8e52476ed5b`.
[Original survivor record](original-survivor.json), [exact diff](native.diff.gz).

The exact integrated `870f7f923d70b1a7b0e7fe73d5f9a4987770fdc4` fixture is
byte-identical to the test and estimator at `d9ea867`. The saved mutant
compiled, then `repeated_registration_keeps_unwarped_cpu_export` failed at
its real source-map assertion with `repeated texture produced an unsupported
transform` and exit 101. The same test passed after restoring the exact stock
source bytes. The ordinary exporter produced both focused maps.
[Mutated test](mutant.log.gz), [restored test](original.log.gz),
[restored hashes](restored.log).

| Focused LLVM evidence | Restored stock | Mutated |
| --- | ---: | ---: |
| Distant-peak score expression at 362:79, evaluations | 20 | 56 |
| Ambiguity guard at 220, true / false | 1 / 0 | 0 / 1 |
| Identity abstention at 221, hits | 1 | 0 |
| Later reference-self-correlation guard at 254, true / false | 0 / 0 | 0 / 1 |

The single real fixture reached the intended difference comparison and
abstained on the stock path. With the saved mutation, it passed both
abstention guards and supplied an unsupported transform. The test and
production hashes were preserved after restoration. This replay closes the
single previously missed variant. [Counts and map hashes](summary.json),
[stock map](original-map.json.gz), [mutated map](mutant-map.json.gz).

The Linux x86-64 AMD Ryzen 9 7950X3D host ran offline at CPUs 10 and 11
with two build jobs, `nightly-2026-11-15` and cargo-llvm-cov 0.9.1. Compiler
wrappers and desktop endpoints were unset. No desktop or GPU ran.
[Commands](command.txt), [checksums](SHA256SUMS).
