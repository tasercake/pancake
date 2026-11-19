# Bilateral witnesses for fixed-anchor Breathing

This design starts from `8694b555666a1ddf4051e2839bbced4bcc625859`. It uses
the authenticated nine-source CPU observations at `806ab13` and the
independent forward and reverse native correspondences. No new input or
product build is needed for the coordinate replay. [Nine-source archive](../motion-admission/README.md).

## A useful candidate and a necessary limit

Let `F[i]` be the ordinary first-anchor-to-source estimate, and `R[i]` the
source-to-anchor estimate. Their implied adjacent maps are
`F[i] · inverse(F[i−1])` and `inverse(R[i]) · R[i−1]`. The latter agrees
with measured motion on both central transitions. This requires two measured
reverse fits. Replacing only the later absolute pose mixes two potentially
different anchor errors and creates another discontinuity.

| Witness group | Direct step error | Within-reverse step error | Mixed absolute switch error | Native witness coverage, forward / reverse |
| --- | ---: | ---: | ---: | --- |
| Genuine `580→581` | 1.323 / 1.347 | 0.205 / 0.196 | 7.030 / 6.972 | 9 / 8 of nine spatial cells |
| Wrong direct jump `654→655` | 17.844 / 17.262 | 0.137 / 0.134 | 12.927 / 12.994 | 3 / 3 cells, all in the left column |

Errors in the table are forward and reverse transfer medians in scale-6
proxy pixels. The native-to-proxy transform uses the same half-pixel
center convention as the product. The replay verifies each error equals
the corresponding native error divided by six. The genuine displacement
therefore remains geometrically distinguishable in proxy coordinates. The
archived matches contain no luma proxy samples and cannot establish that
the measured texture survives the product's downsampling.
[Coordinate replay](replay-proxy-geometry.py.txt) · [All six intervals](proxy-geometry.json.gz).

The false map is strongly contradicted on visible neighboring texture.
The narrow coverage and close identity and reverse residuals leave the
full-canvas replacement open. A supported genuine transition and an
untrusted direct proposal therefore need distinct witness outcomes. In
particular, `confidence == 0` from an existing pair means an unresolved fit.
It supplies no stationary measurement.

## One bounded matcher experiment

Start with equal-size normalized canvases and `Breathing` with a fixed
first anchor. The existing public `thumbnail_with_scale` and `estimate_pair`
produce candidate maps. The experiment compares their implied adjacent
steps on the previous and current thumbnail. Identity is an explicit
competing hypothesis. Reverse next-to-first maps are optional proposals
and must be finite, invertible and within the existing model's forward
limits after inversion. Each directional step requires proposals at both
endpoints or a positively supported anchor pose. The legacy apparent step
from an abstaining predecessor is useful as a negative control. A decision
never comes from the raw direction confidence, an NCC value, or the size
of the displacement.

1. Sample a fixed grid of spatially separated blocks. Use the same query
   sites for all maps in a comparison. Limit each block to the intersection
   where all hypotheses have complete source interpolation support. Do
   the same independently in the inverse direction. Curve fits cannot
   gain support by sampling a different crop or clamped edge. Candidate
   estimation used the two first-anchor relationships. This comparison
   uses the separate previous-to-current relationship, with shared imagery.
   The independent optical test remains the external witness.
2. Per block and direction, split sites into two deterministic lattices.
   Fit positive affine exposure gain and offset on one lattice and measure
   the normalized transfer error on the other. Cross-fit by exchanging the
   lattices. Variance, covariance and squared residuals use the five
   existing correlation moments. For a fit `a*y + b` to `x`, the validation
   squared residual is `Sxx + a²*Syy + n*b² − 2*a*Sxy − 2*b*Sx + 2*a*b*Sy`.
   Centered contrast normalization conditions the score. Unresolved
   exposure fit or texture variance leaves the block unsupported.
3. Require observable two-axis texture in both images. The same moments
   give sampled self-differences `Sxx + Syy − 2*Sxy` under unit grid steps
   in x, y and the two diagonals. Their directional differences estimate a
   local gradient normal matrix. Reject an unresolved spatial eigenvalue
   relative to independently calibrated numerical and texture limits.
   A unit grid step defines sampling for that diagnostic. The estimated
   camera motion can be larger or smaller. The normal matrix tests local
   observability. It cannot establish uniqueness on a repeating pattern.
4. Compare paired validation residuals for every proposed map versus its
   competitors. Inspect both directions and both lattice fits. Require the
   preference to survive the measured backend/repeatability envelope and
   removal of any single spatial block used for acceptance. Check coverage
   of the original supporting set and the preference after each omission. This is a
   geometric robustness check, and overlapping filtered samples preclude
   a statistical independence claim. A reproducible local contradiction can
   disqualify a global proposal. Admitting a replacement requires coverage
   across both spatial axes, rank-two support of block centers, and
   agreement between the supporting regions across the focus change. If
   the alternatives are indistinguishable, the support is confined to one
   part of the frame, or the low-pass appearance changed too far across
   focus, return unresolved. An independent bound for block comparison at
   product resolution remains to be measured. The earlier raw landmark
   error margins are not such a bound. Calibrate against changes in known
   stationary and moving synthetic pairs, on separate scenes with varying
   defocus and the same interpolation support, before seeing the D150
   candidate preference.

The existing `Pair` already uploads a common luma pair, evaluates affine
bilinear samples and reduces five moments in fixed order on CubeCL. Its
default launch has one rectangle for the entire batch. A small experiment
would add a bounded layout for multiple rectangle records and host
selection, reusing the same sampler and moment algebra. Two launches can
swap query and source tensors and each can batch several block/candidate
records. Product code needs no native OpenCV dependency, native RGB
rescoring or altered thumbnail scale for this experiment. The current
single whole-grid Pearson result loses the block and cross-fit evidence
and cannot serve as that witness directly.

## Connected support and public tests

Keep the first-anchor state, previous monochrome thumbnail, previous
directional proposals and an explicitly supported pose. Translate a
selected relative step into the current common-canvas coordinate system
by composing it with the supported predecessor pose. Never switch the
current absolute map alone. Update that state once per new source index.
Cached slab-overlap maps return their saved geometry and leave the previous
state intact. An unresolved transition leaves absolute support disconnected.
An independently supported anchor link is needed to reconnect. The initial
experiment reports that state and does not change publication behavior.
Computing the final crop from a set of admitted maps is a separate whole-job
check. Even perfect adjacent transfers can share an absolute gauge error.

Before integration, use the existing public CPU/GPU registration and
streaming entry points on a continuous, independently rendered textured
sequence with known transforms and changing optical blur/contrast.
The first pilot returns witness outcomes to an observer without changing
the normal job selection. A later end-to-end test would exercise that
selection through `job::run` and the completion observer.
Construct candidate matrices with the application's real pair estimates.
Choose the optical fixture and scale before evaluating the candidate. Keep
the scene coordinates and expected corner and center positions in the
oracle only. Record the baseline failure and require these outcomes:

- A supported real signed translation plus isotropic scale survives focus
  changes and meets the existing one-pixel geometric tolerance on the
  independently known synthetic map.
- A stationary focus change and featureless or repeated pattern give supported
  identity only when the data distinguish it, otherwise an explicit
  unresolved result. Similarity due to periodic texture earns no motion
  credit. A patch group that contains only one edge cannot establish global
  two-axis support.
- The measured mixed-history maps above are a negative control for the
  candidate construction. A separated focus plane or missing overlap
  disconnects support. Slab overlap cannot consume a second motion step or
  encode a different result. Exercise checked memory bounds and pause and
  cancel boundaries on the same shared runtime path.
- Preserve both old direction/step trace rows on a later authorized
  nine-original replay. Verify independent raw forward and inverse
  correspondences. Expect a useful movement on the genuine interval and
  explicit disagreement or unresolved status on the wrong direct fit.
  Narrow false-jump coverage earns rejection evidence only. A regenerated
  full D150 output still needs its own signed-detail and geometry review.

## Resource envelope and decision

One preceding 384×288 f32 thumbnail adds 442,368 host bytes on D150. A
short-lived witness pair adds at most two uploads of that size unless the
candidate uploader is shared. Retain the existing anchor thumbnail and
check the actual extent and live registration budget, including resident
input ownership. Preserve the exact identity fast path and cancellation
checkpoints. An initial nine-block, 16×16-per-block witness with two
stride-two lattices, three hypotheses, both directions, and four self
observability probes per direction uses at most 11,520 sample pairs and
3,600 moment output bytes per edge. That is about 1.72 million samples over
149 edges. These are layout bounds, not measured CPU/GPU times.

Candidate generation is the unpriced part. A full reverse spectral/refine
pass for every frame can nearly duplicate registration. Cache each
attempt by original source index and retain only the needed preceding
proxy and map. Measure the unresolved/reverse-demand rate before making a
product decision. An identity proposal without a stationary witness cannot
justify skipping reverse work. The two historical matched D150 CPU medians
were 43.357 s and 88.300 s, leaving only about 0.793 s to the 0.50 warm
limit, or 5.32 ms per edge if spread uniformly. That headroom belongs to
`12f2151`, and the current candidate needs a fresh matched measurement.
[Historical D150 profile](../../../../qualification/p10-performance/evidence/integrated-d150-12f2151/summary.json).

Recommendation stops at a bounded witness experiment on the existing
proxy representation. Require candidate preference to survive the signed
synthetic and independent native tests, and measure the standalone witness,
reverse demand and full-job cost before proposing a production matcher.
Insufficient coverage or subpixel information leaves the pose unresolved.
Increasing the proxy resolution, shifting the anchor policy, changing the
frozen thresholds or promising a full-stack speed gain is unsupported by
the current evidence.
