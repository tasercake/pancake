# D150 local motion and sampling controls

The follow-up starts from `2143eda2b308497caee96334357dd23d324e1e8e` on Linux
6.8.0-147-generic, AMD Ryzen 9 7950X3D. The sealed acquisition script recovered
the exact 53,459,291-byte Figshare ZIP, SHA-256
`311ca6aab7d3bff03c714fd2d81c741f3847de1d4d3f2f9cfa767b6eb62cf56a`, and all
150 original 2304×1728 JPEGs. The extracted 52,911,148 bytes matched all
individual sizes and hashes in the frozen manifest. The fusion study shared
this same read-only materialization and owned its outputs separately.
[Acquisition receipt](acquisition.json.gz).

## Independent native track

All 149 D150 adjacent steps were fitted. The separate grid qualified 148
steps with at least four usable witnesses. Median per-step error was
0.736 px versus 0.906 px for identity. The fit improved on identity in 107 of
those 148 steps. The largest fitted consecutive change on the native corners
and center was 8.678 px at the same source pair where the pinned reference
recorded its largest 8.492 px step. The diagnostic end map applies +1.558%
isotropic scale and `[-14.07,+66.56]` px translation. It would require
conservative symmetric margins of 40 px x and 95 px y. These estimates
describe an experimentally supported local hypothesis. The absolute
trajectory and expected export shape remain open. [All local maps and
witness statistics](native-local-track.json.gz).

Separate reciprocal normalized patch matching used the original 72×72
oracle and fixed coordinates with no fitted map used to choose searches.
On preregistered gaps of 24–149 sources no patches met the old acceptance
criteria. Across 16 supplementary fixed four- or eight-step gaps eight met
the three-patch minimum. The local chain improved on identity in six of
those eight. These checks establish some continuity over shorter gaps.
They leave long-range drift uncertain as focused texture changes across the
sequence. [Cross-gap source](../long-track-check.py.txt) ·
[Long gaps](long-track-check.json.gz) · [Short gaps](short-track-check.json.gz).

## Warp sampling control

## Test-first application fit

At test-only `1ce708b`, the application returned identity on an independent
769×577 camera-motion fixture. The known move has 0.62/−0.59 px translation
and 0.038% isotropic scale with local focal detail changing. The resulting
origin errors were −0.62/+0.59 px. The zero-motion companion passed.
[Red test output](subpixel-red-1ce708b.log.gz).

The `841c863` prototype admits a bounded small transform for downsampled
Breathing only after the same CubeCL correlation kernel confirms improvement
in both directions on at least three of four separated regions. The broad
motion confidence limits and the other registration models keep their
existing scoring behavior. Both focused fixtures and the existing CPU
registration test passed on the isolated prototype. Commit `99a23e9`
reuses the adjacent-frame orchestration and retains the selected output
anchor. [Focused test output](subpixel-candidate-focused.log.gz).

## Full-stack CPU diagnostic

The combined source `0cc91592e77a1f6781369de5202a54dfbad4cda7` passed all ten
focused CPU checks across breathing, registration and streaming job targets.
It built with `CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 cargo build --release
--locked -p pancake-cli`, Rust 1.100.0, unchanged Cargo lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The CLI SHA-256 was
`e0c3544dce5b8cf53396a0f5a1cc6c44e8d9f34a2da2a098e5f5db619a31f7ea`.
[Combined native tests](release-0cc9159/subpixel-combined-native.log.gz) ·
[Build output](release-0cc9159/subpixel-combined-0cc9159-release.log.gz).

| --- | ---: | ---: | ---: |
| Published RGB16, width × height | 2130×1530 | 2266×1660 | 2288×1696 |
| Native symmetric crop x/y per edge | 87/99 px | 19/34 px | 8/16 px |
| Largest map change on nearby frames | 129.796 px | 2.607 px | 8.492 px |
| 654→655 median held-out error | 102.893 px | 1.000 px | 1.000 px |
| 577→578 median held-out error | 1.000 px | 0.296 px | 1.000 px |
| Detail at the original four 192×192 specimen ROIs, fraction of pinned view | 0.686–0.706 | 0.619–0.635 | 1.000 |

The prototype admits 54 nonidentity consecutive steps and eliminates the
previous false jump. It missed the supported larger step around 580→581,
and its accumulated map remains substantially shorter than the independent
native hypothesis. The four focus regions all became softer than the sealed
result. A separate Lanczos-versus-bilinear control established sampling
attenuation in those local-source conditions. The further fixed-map fusion
control still had a detail gap on identical inputs. Both registration
and fused quality remain open. Production experiments stay on the isolated
branch. Frozen case replay and performance gates still need evidence for
any future merge. [Complete geometry and focus review](release-0cc9159/geometry-review.json).

After both tasks released the shared originals, the ZIP, all 150 JPEGs and
the prototype TIFF were rehashed and removed. The prototype binary and
completion trace retain their verified hashes. [Cleanup receipt](cleanup.json).
