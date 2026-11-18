# Hybrid detail through overlapping slabs

The sealed D150 CPU result retained 0.686–0.706× the reference's native luma Laplacian in four matched regions. The adjacent-fit prototype eliminated the false 129.8 px jump and retained 0.685–0.704×. Its 150 identity maps short-circuited Pancake's bilinear warp. Slow unresolved geometry and fusion were still mixed in that photographic comparison. The sealed and adjacent-fit reports preserve the same metric and region coordinates.

## Confirmed independent failure

Test revision `ce038ac14dea89bafe3fc1c7adac9ab2b31d91ec` used the real application CPU job with alignment and crop disabled. There were five generated RGB16 PNGs per period, all 257×9. An unambiguous sharp frame on each side lay outside a shared frame in overlapping slabs `0..3` and `2..5`. Defocus reduced sinusoidal contrast to a quarter, without a phase or tone change. The independent oracle projected each native output onto its known phase and compared with the independently decoded sharp source. Two RGB16 codes projected onto the same sampled phases bound fixture quantization and numerical tolerance.

The period-two input isolates the first Laplacian band away from borders. A period-eight sinusoid still has 18.21% of its amplitude after two binomial decimations and zero after the third. With the production bilinear expansion, 15.55% of the period-eight fundamental arises from reconstructing that third band. For one sharp and two identical defocused sources, linear focus weighting predicts 0.75× sharp contrast in the mean. Mixing 14% of that mean predicts 0.965× finest-band contrast. Mixing 60% predicts 0.850× in the third band and 0.94712× across the period-eight fundamental. The actual first-slab values follow those independent calculations at both fixed regions.

| Input period | Decoded sharp amplitude | Stock single slab | Stock real two-pass result | Allowed amplitude error |
| --- | ---: | ---: | ---: | ---: |
| 2 pixels | 0.249996 | 0.241247 | 0.236098 | 0.000031 |
| 8 pixels | 0.250003 | 0.236781 | 0.229104 | 0.000037 |

## Prototype scope

The shared kernel tracks the strongest competing focus support per band as it streams. The prior soft weight is multiplied by `2 × competitor / winner − 1` when the competitor has more than half the winner's support. Clear winners retain the band, equal support retains the original blend, and copies of a weaker defocused source cannot by themselves make it compete. The algorithm keeps the same runtime launches, focus measure, pyramid, export and slab settings. It introduces no sharpening gain. The additional runner-up scalar is included in the checked active-accumulator budget, which changes from 32 to 36 bytes per pyramid pixel for Hybrid. The largest single allocation remains RGB. At level eight the estimated T50 Hybrid accumulator is 3,072,002,160 B within its 3,221,225,472 B allowance. D150 uses 509,607,612 B for each of its two concurrent accumulators within 1,610,612,736 B apiece. The independent budget test accepts the exact tiny requirement and rejects one byte less.

## Focused commands

On Linux x86-64, Ryzen 9 7950X3D, Rust 1.100.0, crate graph lock SHA256 `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`. Both commands ran in `pancake/` with two Cargo build jobs and a scoped compute cache. The host was shared and no elapsed measurements count toward performance gates.

```sh
cargo +1.100.0 test --locked --offline --release --jobs 2 -p pancake \
  --test cpu_job hybrid_preserves_clear_focus_detail_through_overlapping_slabs -- --test-threads=1
cargo +1.100.0 test --locked --offline --release --jobs 2 -p pancake \
  --lib --test cpu_job hybrid_ -- --test-threads=1
```

Prototype source `b7a42b6344340be587fadb0aab73342e7adbc44a` passed all four focused CPU tests. The original period-two and period-eight assertions now pass at both slab stages and both patches. The repeated-defocus test covers eight copies. The faint opposite-phase control retains the tied blend. The exact command output is preserved in the two receipts.

Physical Vulkan parity, untouched feature regressions, optical D150 detail and the frozen performance gates still require execution. A bounded real diagnostic can replay the identical first-frame-coordinate pixels from original D150 slab `108..148` into each CPU fuser with alignment disabled. Fixing those pixels separates fusion from estimated maps or resampling. The subsequent `144..150` tail permits the exact original overlap and second pass with only 42 distinct input frames.
