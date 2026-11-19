# Canonical Linux CPU mutation shard 9/16 at 0a9ae85

The native producer executed all **167/167** operations in the verified
`9/16` partition at `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`.
The baseline passed **148 tests across 19 targets**. The shard
**failed** the mutation gate with 45 survivors and one timeout.
The other 15 canonical partitions at this revision have no native
result in this receipt.
[Exact list preflight](../0a9ae85-canonical-preflight/README.md) ·
[Terminal outcome and findings](terminal-summary.json).

| Workspace package | Caught | Missed | Timeout | Compiler-unviable |
| --- | ---: | ---: | ---: | ---: |
| Core | 106 | 33 | 1 | 7 |
| GUI CPU/headless | 2 | 12 | 0 | 5 |
| CLI | 0 | 0 | 0 | 1 |
| Total | 108 | 45 | 1 | 13 |

The run started November 19, 2026 at 01:30:24 SGT and ended at
02:34:05 SGT after **1h 3m 41s**. Cargo-mutants exited 3, the log
writer exited 0, and the strict evidence verifier exited 1. The source
was clean before and after native execution. The separate terminal
reconciliation matched every exact operation once, with no missing,
duplicate or unexpected outcomes, a successful baseline, the required
native phase commands and the associated logs and diffs. The timeout
was `registration/spectral.rs:259:32`, where the changed FFT butterfly
index built successfully and the test phase reached its actual 60s
deadline. Unviable rows each retained their failed native Build phase.
[Run provenance](provenance.txt) · [Replay command](command.txt).

Linux 6.8.0-147-generic x86_64 ran two Cargo jobs on distinct Ryzen
7950X3D cores, CPUs 6 and 7. The pinned Rust 1.100.0 and cargo-mutants
27.1.0 used a dedicated target, no shared wrapper, no GUI session and
no GPU target. The all-features locked CPU Test phases used the crate
libraries, binaries, all 16 classified CPU integration targets and
`--include-ignored`. Build and test settings stayed at 180s and 60s.
The largest sampled target was 7,339,266,048 bytes (6.84 GiB). It
finished at 7,331,684,352 bytes. The 168 native logs held 170,442,947
bytes alongside 167 exact replacement diffs.
[Resource observations](observed-resources.json) · [Samples](progress.tsv).

The local branch `codex/pancake-native-mutation-preflight-0a9ae85`
retains the raw archive as `raw-native.tar.gz`, SHA-256
`4c0707b77673d444363b07acf99df632cae44fa0877106fa8115e4eb93744462`.
It preserves the independent roster, authoritative native execution
list and outcomes, stdout, exit statuses, every phase log and patch,
and the replay and reconciliation scripts. Artifact-only controls
verified the separately executed historical success and failure reports
and rejected both a missing operation and a missing native end marker.
[Classifier controls](classifier-controls.txt).
