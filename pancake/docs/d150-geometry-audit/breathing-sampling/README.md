# Bounded sampling for diagonal Breathing maps

The D150 sampling control used identical source coordinates and found that
bilinear interpolation retained a median of 84.2% of Lanczos4 detail on
eight selected source ROIs. The fusion control separately found a quality
gap on identically prewarped inputs. This branch tests a sampling improvement
within the product. [D150 controls](../subpixel-diagnostic/README.md).

## Independent signed-detail proof

The test renders R, G and B from independent continuous functions evaluated
in f64. The signed input has spatial frequencies of 0.161 and 0.173 cycles
per pixel on separate axes. The test calls the same sampler the Breathing
job uses and compares the cropped output with the independent continuous
image at the exact `1.002` diagonal scale and `(+0.42, −0.52)` px offset.
The required signed amplitude projection is at least 0.92. Signed normalized
agreement must reach 0.995.

On test-first commit `9028c62` the bilinear amplitude was **0.866763** with
agreement **0.999927**, failing the gain criterion. The same test passed
with bounded cubic sampling. This establishes at least 0.053237 improvement
in signed gain on the independent fixture. Signed cropped-edge bounds,
nearest border extension, exact generic fallback, extreme finite HDR and
the original allocation limit were also exercised. The final checks add
nonfinite rejection and an independent packed RGB coordinate ramp whose
264,000 samples cross a device tile between color channels.
[Red output](red-9028c62.log.gz) · [CPU run](cpu-00151fe.log.gz).

## Eligible geometry and memory

The kernel uses separable four-tap Catmull–Rom interpolation on transformed
Breathing maps with zero cross-axis terms and both scales in `[0.95, 1.05]`.
Each axis clips to the two enclosing samples to limit ringing. Edge taps
extend the nearest raster value. The weighted sums use half-scale arithmetic
to keep extreme signed f32 radiance finite. Stage admission, tile capacity,
buffer allocation accounting and the same finite-result readback apply to
both samplers. Identity on a matching canvas keeps the direct frame path.
Generic affine, other models and scale outside the near-unit range keep the
established bilinear filter. The crop calculation uses the same affine maps.

At `00151fe`, all 25 library checks, the existing CPU geometry target and all
six CPU streaming tests passed on Linux x86-64 with AMD Ryzen 9 7950X3D and
Rust 1.100.0. The locked command used `CARGO_BUILD_JOBS=2`,
`CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0`.

```bash
cd pancake
cargo test --locked --lib registration::diagonal_tests -- --nocapture
cargo test --locked --lib --test cpu_registration_contract --test cpu_stack_plan -- --nocapture
```

The resident-RGB integration `59d0bcb579b8cbd7718df7867ef1fd0a87cb86f5`
passed the same 32 CPU checks and strict scoped Clippy. It preserves the
resident identity return and drops the resident handle before either warp.
[Integrated CPU tests](release-59d0bcb/cpu-59d0bcb.log.gz) ·
[Strict Clippy](release-59d0bcb/clippy-59d0bcb.log.gz).

## Matched actual D150 CPU pair

The baseline is the independently verified CLI payload from the exact local
Linux `287828a` Debian package. Its executable SHA-256 is
`874c8683623131b5619858a9cba61cb6219bc2eae6d6bdb30ed1bc5d8634ce8e`.
The independent locked release at `59d0bcb` uses SHA-256
`06cc7cd775541281093ad4da427e590276fb743cecfea86b835d604171e172c6`.
The two sources differ only in the bounded-sampler experiment and evidence.
[Baseline package receipt](release-59d0bcb/baseline-package.json) ·
[Candidate build log](release-59d0bcb/release-59d0bcb.log.gz).

The four previously chosen raw-focus regions retained their input-derived
witnesses. For the second group, each of the 12 existing raw grid cells used
the highest-detail witness from the eligible late sources, rendered at the
same map with independent Lanczos4. Selection used originals and maps
before decoding either product export. Both outputs were sampled in matching
native first-canvas coordinates. Signed projection measures agreement with
the source's actual luma Laplacian instead of rewarding additional unsigned
contrast. [Fixed raw selection](release-59d0bcb/raw-selection.json) ·
[Comparison source](release-59d0bcb/run-paired-d150.py.txt).

| Witness set | Signed projection gains | Median change | Median signed phase change |
| --- | ---: | ---: | ---: |
| Prior raw focus holdouts | 0/4 | −0.0001735 | −0.0000179 |
| Eligible late-source grid | 10/12 | +0.0004339 | +0.0000831 |

The archive, 150 originals and both generated TIFFs were authenticated and
removed after both consumers released the shared stage. The exact candidate
CLI and completion receipts were retained. [Cleanup](release-59d0bcb/cleanup.json).
