# Public P5 geometry mutation replay

Two operators reported as missed on exact `0a9ae85` shard 9/16 are
covered by the existing native CPU registration target. The test-only
revision is `66271ed`. The geometry contracts use public `Affine`
and `symmetric_crop` operations.

For the translated signed shear
`X=x+y/2+3, Y=−x/2+3y/4−2`, solving the two equations gives
`x=3X/4−Y/2−13/4, y=X/2+Y+1/2`. The determinant and all
coefficients are exactly representable. The test verifies the
independent inverse and both composition orders. The saved
`76:17` mutant changed the lower-left coefficient from 0.5 to −0.5
and produced two nonidentity compositions.

For 20 columns, the last center is x=19. An x scale of 1.052
displaces it by 0.988 pixels. Rounding outward plus the interpolation
margin gives an x inset of two and bounds `(2, 1, 16, 8)` on the
20×10 canvas. The saved `265:41` mutant sampled x=20, where the
displacement exceeds one pixel. It returned `(3, 1, 14, 8)`.

Each exact original patch built and ran independently on logical
CPUs 8 and 9 under offline locked Rust 1.100.0 with two jobs and
empty compiler wrappers. Both focused mutant runs failed at the new
numerical oracle with status 101. Each restored stock run passed all
five tests, with status 0. The production geometry file was
byte-verified against the baseline. Formatting passed and the owned
build target was removed. These receipts cover both operators.
[Run receipt](receipt.json) [Original diffs and native run logs](native-capture.tar.gz)

To replay, unpack the archive into a fresh task-owned directory and
run its `replay.sh` with that directory as the argument from a clean
`66271ed` checkout.
