# E9 CPU with the corrected reference

| --- | ---: | ---: | ---: | ---: | --- |
| Warm median completed job | 0.377574641 s | 0.886587335 s | 0.425874 | 0.50 | Pass |
| Cold median completed job | 0.372393204 s | 6.516783979 s | 0.057144 | 1.00 | Pass |
| Peak process-tree RSS | 110,604,288 B | 346,869,760 B | 0.318864 | 0.50 | Pass |

Every export had the same `337×510` geometry and genuine RGB16 depth. The first-warm images reproduced the earlier functional receipt hashes. Full-image MAE was 510.171 unsigned 16-bit levels. Fixed upper-left, center and lower-right window MAEs were 302.447, 260.205 and 649.201 levels, with zero floor or ceiling fractions in both images within these windows. Independent native-scale and enlarged review found comparable sampled detail, no new registration or reconstruction artifacts, and residual exposure differences. The Pancake output reached the lower endpoint in 364 additional blue-channel samples in the right balloon artwork and the upper endpoint in 731 additional channel samples around the sun and clouds. The reviewer found no visible texture loss at native scale. [First-warm ROI metrics](fixed-roi-review.json) · [Independent review](independent-quality-review.md) · [Independent metrics](independent-quality-metrics.json)

## Reproduction

Use a clean benchmark checkout at `e9b20f47ffb69fe7366e8dfe7c4907086d1fc014` with the frozen reference environment, the exact originals and the verified P9 package. The bridge binds the original function code to an isolated candidate provenance check. It validates the P9 package, read-only extracted CLI, the build log, matching complete product tree, descriptor and `Cargo.lock`. The benchmark checkout and reference launcher retain separate commit and file hashes. The original parser, paired job schedule, subprocess calls, cache rules, clocks, validators and reducers execute unchanged. Two no-device tests covered wrong source, descriptor, routing and private function namespace rejection. The real no-job preflight verified both immutable identities and all 34 scheduled entries.

Place the exact `./usr/bin/pancake-cli` member from the verified `.deb` in the owned benchmark checkout at `pancake/target/release/pancake-cli` with mode `0555`. Configure the pinned Python environment at the reference descriptor's `.venv`. Run the archived bridge from an ignored path in that worktree. `QUAL_PY` points to the installed qualification Python, `RELEASE_LOG` is the existing P9 release build log, `PACKAGE` is the exact Debian archive alongside `package-verification.json`, and `PRIVATE` is the case-owned artifact directory.

After independent review, the nine originals, both first-warm TIFFs and the privately extracted executable were rehashed and removed. The old source archive and case-owned caches had already been removed. Reproduce the verdict offline with the original reducer, verified lane inventory and independent quality receipt. For this command, use the integrated repository as `ROOT` and this report folder as `EVIDENCE`.

```sh
"$QUAL_PY" "$EVIDENCE/reduce_verified_profile.py.txt" \
  --p9-dir "$ROOT/pancake/docs/p9-linux-package-evidence/c1e2cac" \
  --targets "$ROOT/pancake/docs/performance-targets.json"
```

[Cleanup receipt](cleanup.json)
