# P6 job observer and worker boundary tests

The focused native comparison ran on the Linux x86-64 host documented in the
[parent receipt](../README.md), with Rust 1.100.0 and Cargo LLVM Coverage 0.9.1.
The only source delta from `4aa4f81f1a476bac7054a8badc17954345e49354` to
`f3725dc63d907589712398a38694ad2bed785c94` is the test target. Both captures
used test-owned 9×8 RGB16 PNGs and forced the actual CPU processing runtime.
The new cases exercise real invalid observer paths, immutable input and output
files, in-memory completion with geometry and quality, a runtime budget
rejection, and failed worker generation recovery and shutdown.

The focused coverage command ran from `pancake/` on both revisions.

```bash
cargo llvm-cov --locked --workspace --all-features --test cpu_job --json \
  --output-path "$report" -- --test-threads=1
```

The baseline ran three passing tests and the updated target ran seven.
Both exited zero with no skips. [Baseline log](before.log.gz)
[Baseline coverage](before.json.gz)
[Updated log](final.log.gz) [Updated coverage](final.json.gz)

| Production module | Executed lines before | Executed lines after |
|---|---:|---:|
| Job events | 46/73 | 63/73 |
| Job entry and result API | 106/169 | 131/169 |
| Frame stream | 138/224 | 147/224 |
| Worker | 97/115 | 101/115 |
| Runtime | 111/228 | 114/228 |

The focused comparison exercises 58 additional production lines, ten functions
and 57 regions. `bash scripts/check.sh fast` also exited zero at the updated test
revision. Full source and branch assurance require their separate gates.
[Fast log](fast.log.gz)
