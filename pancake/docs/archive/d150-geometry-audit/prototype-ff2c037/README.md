# D150 adjacent-pair prototype

The test-first candidate was run on the locked Linux x86-64 CLI at
`ff2c037ad86afb8c4dc1220ceeffbb557fa79bdf`. It fits consecutive breathing
pairs and composes the maps into the selected first or middle output anchor.
The same CubeCL estimator and model limits apply to each pair. The prototype
is retained separately from the sealed release.

Resolved environment was Linux 6.8.0-147-generic, AMD Ryzen 9 7950X3D, explicit
CubeCL CPU, and diagnostic process affinity 0–15. Builds used Rust and Cargo
1.100.0, with `CARGO_BUILD_JOBS=2` and `CARGO_INCREMENTAL=0`. The headless
binary SHA-256 was `83335816e3a644cef89d3ca7485f16cab2e6f7ceee8a1e0a65fd50cedb395dbd`
and Cargo lock SHA-256 was `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
[Locked release build](release-ff2c037.log.gz).

## Independent known-motion result

The test generates five RGB16 PNG captures with two spatially separate focal
planes and a known `(+3, −2)` px translation per capture. Their detail moves
between the planes. A direct first-to-last fit must miss more than 3 px.
The complete CPU job uses overlapping slabs. At `a82ed6c` the fixed-first job
failed at source index 2 with identity and 6.000 px error. On `ff2c037` the
known track passed within 1 px at every tested corner for first, middle and
previous, with exact identity at the chosen anchor. Eight native tests passed
across the new known-motion case and the existing registration/job plan tests.
[Base failure](native-red-a82ed6c.log.gz) · [Prototype passes](native-green.log.gz).

```bash
cd pancake
cargo test --locked --test cpu_breathing_reference
cargo test --locked --test cpu_breathing_reference --test cpu_registration_contract --test cpu_stack_plan
cargo build --release --locked -p pancake-cli
```

## Frozen D150 inputs and separate geometry diagnostic

| Observation | Sealed `12f2151` | Prototype `ff2c037` | Pinned reference |
| --- | ---: | ---: | ---: |
| Export, width × height | 2130×1530 | 2302×1726 | 2288×1696 |
| First-canvas trim on each x/y edge | 87/99 px | 1/1 px | 8/16 px |
| Identity maps | 139/150 | 150/150 | 64/150 |
| Largest adjacent map change at canvas corners and center | 129.796 px | 0 px | 8.492 px |
| Median held-out landmark residual at 654→655 | 102.893 px | 1.000 px | 1.000 px |
| MAE on the same 2130×1530 first-canvas overlap versus reference | 3220.343 u16 | 3121.487 u16 | — |
| Detail at the same four fixed 192×192 specimen regions, fraction of reference | 0.686–0.706 | 0.685–0.704 | 1.000 |

The prototype eliminates the measured false jump and the large crop it caused.
Its estimator suppresses all steps on this full stack. The widened crop does
not establish accurate slow motion over 150 frames. The same four independent
native regions retain the softness observed in the sealed output. This
prototype therefore leaves D150 registration and fusion quality open.
[Numeric overlap and landmark review](geometry-review.json) ·
[Sealed D150 review](../../../../qualification/p10-performance/evidence/integrated-d150-12f2151/README.md).

## Further regression scope

The frozen CLI replay was prepared for 95 unique IDs, including all 64 P5
IDs, 16 other slab cases, five CPU corrupt-breathing cases, and ten physical
GPU slab, breathing and corrupt-breathing cases. These have no passing
evidence on the prototype. Replay uses the exact locked release CLI and a
physical GPU for the device cases.
