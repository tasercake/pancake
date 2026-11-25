# Registration thumbnail parity plan

The `287828a` R50 trace spent 17.206 s inside `frame_thumbnail` across
50 frames. That call includes RGB staging, sampling and readback. The
49 transform searches combined took 0.420 s.
[Trace summary](stages.json)

The exact `07816a2` registration upload candidate changes
`kernels::upload`, called by `make_luma`, `warp` and pair scoring.
For thumbnails it keeps the same `prepare_thumbnail` dimensions, RGB
tensor, 5×5 sampler, readback and optional resident RGB handoff. The
paired `287828a` and `07816a2` R50 diagnostic will measure the effect of
that upload change before either stage or product optimization is claimed.
[Shared kernel source](../../../src/registration/kernels.rs)

## Identical sampling and summation

For output width `W` and height `H`, cache a scalar luma sample at every
point of a checked `(W+4)×(H+4)` lattice. The cell used by thumbnail pixel
`(x,y)` and tap `(c,r)` is `(x+c,y+r)`. Its coordinates remain

```text
source_x = (f32(x+c) + 0.5 − 2.0) * current_scale_x − 0.5
source_y = (f32(y+r) + 0.5 − 2.0) * current_scale_y − 0.5
```

The existing kernel computes `((f32(x) + 0.5) + f32(c) − 2.0)`
before the scale operation. The output dimensions are bounded by u16
and taps by four, so integer and half-integer additions before scaling
are exact in f32 in both expressions, including negative halo points.
The current scale and multiply/subtract ordering remain. Each lattice
sample uses the existing bilinear coordinate clamp, co-sited RGB reads,
per-channel clamp and Rec. 601 coefficient order. Clamping before
interpolation or after combining RGB would alter the stage's numeric
contract.

The second kernel walks rows 0–4 then columns 0–4 for each output.
Preserve the current coefficient products, f32 additions in that
order and final division by 256. A separable sum changes rounding
order. The parity oracle compares decoded f32 bits on each target to
the original one-pass sampler. This catches backend compiler behavior
at the new f32 storage boundary, including any change in contraction.
Changes to parity need a new design before integration. Both kernels
must guard padded work items before indexing their tensors.

## Working-data bounds and cancellation

Use checked `usize` halo dimensions, sample counts, byte counts and
dispatch conversions. Halo dimensions can exceed u16 for a one-pixel
axis paired with the longest allowed canvas. The source/output
geometry keeps its existing extent bounds. Do not clip the halo and
change the clamp. The exact additional scratch requirement is
`4 * ((W+4)*(H+4))` bytes because the lattice has one f32 per cell.
The R50 768×512 proxy needs 1,593,408 B. The maximum source sampling
points per frame are reduced from 9,830,400 to 398,352 for this
geometry. Those are operation counts.

Use the same registration working budget and verify both allocation
size and device addressability before launch. Preserve the current
one-pass route for a budget that fits the current operation but lacks
room for the additional lattice, unless the existing readback reserve
can cover the lattice by proven nonoverlapping lifetimes. This selector
preserves an existing public resource boundary. Add no image-size,
backend or file-format selector without a measured common-case need.
Keep input and lattice alive through their consuming kernel, release
the lattice after thumbnail materialization, and keep the existing
completed readback and optional resident RGB handoff. The stage remains
on the owner thread. Existing registration checkpoints govern pause
and cancellation, including normal drop of resources on error.

## Independent parity and resource tests

1. Before production edits, capture the current one-pass device
   sampler as a test-only oracle. Compare the new and original
   thumbnail f32 bits for every pixel on each backend. Use a second
   scalar oracle to confirm tap, clamp and color geometry on CPU.
2. Cover output extents 1×1, 1×N, N×1 and 2×2, odd 17×13 and
   non-square custom thumbnails, fractional downsample/upsample,
   extreme center-crop/halo positions and low-luma RGB detail.
   Include legal finite RGB values outside [0,1] to detect clamp
   reordering and a pan with a long axis to exercise checked strides.
   Include constant field, gradients, checker texture, colored impulse
   and reproducible pseudo-random arrays.
3. Exercise the smallest existing legal budget and a boundary one
   byte short of the lattice. A formerly legal job must still
   complete through the exact one-pass route if needed. Verify new
   exact budget passes and invalid size arithmetic fails before launch.
   Exercise forced CPU and physical NVIDIA parity and the existing
   P5/P6 registration, pause/cancel and working-memory regressions.
4. Verify source files, matrices, geometry and native RGB16 pixels for
   a complete frozen R50 replay. Check per-stage improvement separately
   from a whole-job result and record actual temporary allocation or
   conservative memory bounds. Reuse the other native workload to
   verify throughput and stable output on a different image scale.
