# Weak spectral peak regression

The exact `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` canonical native `9/16` mutation shard recorded `src/registration/spectral.rs:146:17` as a survivor. The saved patch replaces the weak-peak comparison `<` with `==` in `dominant_peak`. The canonical build and test phases both succeeded. The original shard record is retained as historical evidence.

The new pure library test supplies a unique real bin in a 64×64 complex correlation surface for a 192×80 thumbnail extent. The bin at column 61 and row 8 encodes the signed translation `[-9, 10]` in thumbnail coordinates. The test requires 204.75 to abstain and 204.875 to return the exact signed displacement. Both values flank the predetermined 5% cutoff of 204.8. The initial replay used 192×128 and independently detected the same failure. The final extent uses the 64×64 proxy size that production selects for a thumbnail of those dimensions.

The exact saved mutant compiled and failed the focused test by emitting `Some([-9.0, 10.0])` for the weak input. The byte-restored source passed the focused test and all 24 native library tests. Rustfmt and the reviewed Miri source and dependency policy passed. The test uses only the pure reducer and host values. The Darwin Miri runtime and the full mutation shard were outside this replay.

Three nearby survivors remain unresolved. The right-neighbor substitution at `194:39` changes the interior Laplacian of a horizontal ramp, but the public estimator can bypass the spectral stage on that input. The window arithmetic at `199:45` changes flat-image intermediate values, but ambiguity rejection can mask those values in the public result. The column address at `223:23` can select the wrong sample or underflow during the vertical pass, with no independently measured public downstream effect in this audit. These are source observations. No survivor classification changed.

[`receipt.json`](receipt.json) records the revisions, source and lockfile hashes, CPU and exact commands. [`native-raw.tar.gz`](native-raw.tar.gz) retains the canonical mutant record and log, exact patch, both red witnesses, restored native logs, and policy and scope checks.
