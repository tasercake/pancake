# Sharpness row and reflected neighbor mutation replay

Exact `0a9ae85` shard 9/16 recorded two missed operators in
`pancake/src/fusion/focus.rs`. The test-only `6358a90` revision extends
the shared sharpness contract with a top-left unit grayscale impulse
on a 2×2 grid. Reflect-101 in both axes gives Laplacians
`[-4, 2, 2, 0]` and energies `[16, 4, 4, 0]`. The mean is 6.
The earlier single-row and flat-image cases remain.

Each exact saved mutation ran in isolation through the existing
`cpu_fusion_and_resize` test, with the real default CubeCL CPU runtime.
Replacing row-plus-column by row-times-column at 313:26 produces row
sums 32 and 20 and mean 13. Replacing the below-neighbor row-stride
multiply by divide at 387:43 changes the first energy to 9 and the
mean to 4.25. Both observed values matched these predictions.

| Leg | Exit | Observed result |
| --- | ---: | --- |
| Original `313:26` patch | 101 | The new assertion reported 13. |
| Restored stock | 0 | All three target tests passed. |
| Original `387:43` patch | 101 | The same assertion reported 4.25. |
| Restored stock | 0 | All three target tests passed. |

Replay each archived patch separately in a clean `6358a90` checkout
from the `pancake` directory with `patch -p0 -i "$diff"`. Exact direct,
offline, locked Cargo commands and all outputs are in the archive.
Rust 1.100.0 used logical CPUs 8 and 9 with two jobs, both wrappers
disabled and incremental build off. Production source was restored
after each mutant and byte-verified against `0a9ae85`. Formatting
passed and the owned Cargo target was removed. These are scoped
mutation results. Full-shard and device assurance remain separate.
[Run receipt](receipt.json) [Exact original diffs and run logs](native-capture.tar.gz)
