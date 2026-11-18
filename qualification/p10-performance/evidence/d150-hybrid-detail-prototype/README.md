# Hybrid detail through overlapping slabs

**Disposition** The focused fix improves real D150 texture, but the full diagnostic still retains only 0.741–0.767× the pinned reference's detail. The source remains on the experimental branch. The sealed package supplies the release evidence.

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

## Physical parity and scoped frozen cases

A locked offline CLI release build at `81e3c0089d3fd03899b59dba49c73f1ef38dfb7c` used Rust 1.100.0, two build jobs, and the same lock checksum as the sealed CPU package. Its SHA256 was `7d14496e76b0f6b678bc306f6b4cb80505e4823b98d249d5644bcc909da6694e`. All four known-phase scenarios completed with both actual CPU and physical Vulkan. Decoded RGB16 pixels were bit-identical over every sample. Each case met its independent projection bound. GPU completion named the RTX 3090 at `0000:01:00.0`, `wgpu-vulkan`, with no fallback.

## Real fixed-input 40-frame diagnostic

The ZIP and all 150 originals matched the frozen D150 manifest and hashes. Frames `108..148` were converted once into common first-frame coordinates using the 12f campaign's observed reference forward maps, Lanczos4 and replicated borders. Both products fused the exact same 40 full-canvas RGB16 TIFFs with alignment and crop disabled. The 40-frame comparison has one slab and therefore isolates the first blend from estimator and inter-slab effects. It took its four coordinates from the earlier frozen matched-region review.

The prototype raised unsigned luma Laplacian P90 by 1.94–2.67% across the four regions and differed from stock by 12–18 RGB16 codes on average. Signed phase correlation to the sharpest registered source per region was only 0.057–0.075 in stock and slightly lower in the prototype. The source review showed defocused scene patches across this group, with detail P90 0.0087–0.0107 in the selected source patches. A stronger texture claim needs a focused source. [Common inputs, selected maps and fixed-40 results](fixed-40-report.json) [Exact acquisition](acquisition.json.gz)

## Real full D150 diagnostic

One sealed and one prototype job used the authenticated native JPEGs directly, the sealed CPU launch's ordered settings, affinity 0–15 and a 32 GiB address-space limit. Both used the frozen Hybrid/breathing, first anchor, kernel 6, levels 8, scale 6, slab 40 overlap 4, auto-crop, and RGB16. Both completed on an actual CPU and produced byte-identical completion events including all 150 original and sequential transforms. Both exported a genuine 2130×1530 RGB16 image from first-canvas bounds `[87,99,2217,1629]`. The freshly run stock output reproduces the recorded first warm encoded and decoded hashes.

For each preregistered region, the independently sharpest registered original supplies the signed Laplacian witness. Only native source images were used to pick the witness. The prototype output increased signed projection onto that witness by 8.6–12.7% relative to stock and improved its signed phase correlation by 0.0008–0.0049. Image review showed slightly stronger texture in the same structures. The result has 23.3–25.9% less detail than the historical pinned reference, and both product outputs retain the sealed crop and geometry deficit.

| First-canvas region origin | Stock detail / reference | Prototype detail / reference |
| --- | ---: | ---: |
| (1271, 131) | 0.706 | 0.767 |
| (1079, 515) | 0.691 | 0.750 |
| (1271, 515) | 0.686 | 0.741 |
| (1079, 899) | 0.694 | 0.753 |

## Reproduce and clean up

Run from the repository root. Use the frozen reference Python environment for the NumPy, OpenCV and tifffile measurement scripts. Reacquire using the sealed D150 acquisition recipe. Extract `paired/diagnostic-01/candidate/launch.json` from the sealed D150 archive as the launch template. Replay `run_detail_parity.py.txt PROTOTYPE NEW_OUTPUT` on the named RTX. For D150, use `run_fixed_d150.py.txt STOCK PROTOTYPE INPUT_ROOT NEW_OUTPUT 148`, then the separately authorized full pair with `run_full_d150.py.txt STOCK PROTOTYPE INPUT_ROOT SEALED_LAUNCH NEW_OUTPUT` followed by `compare_full_d150.py.txt INPUT_ROOT FULL_OUTPUT`. The scripts authenticate the executables, originals, output shape and hashes. Source D150 material is the [Amber mite focal-stack release](https://doi.org/10.6084/m9.figshare.14707077.v1).

All acquired originals, the ZIP, common registered TIFFs, outputs, visual panels, and synthetic test workspaces were removed after checking receipts. The exact prototype binary remains in the local performance artifact directory with its verified SHA256. [Cleanup and retained executable](cleanup.json)
