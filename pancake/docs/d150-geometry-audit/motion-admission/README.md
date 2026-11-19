# D150 motion admission under changing focus

The archived `287828aac07ffad2252785b73a52a59fc07b3409` packaged CPU trace
matches all 150 absolute maps from the earlier `12f2151` run. The two
transitions below distinguish a physically supported movement from a new
direct fit to the distant first frame. Distances are native pixel-center
errors, and the corner statistic also includes the image center.

| Adjacent captures | Archived product step | Independent witness | Median error of the alternatives |
| --- | ---: | --- | --- |
| `P9160580` → `P9160581` | Identity | 47 native reciprocal optical witnesses, local step up to 8.67 px, pinned step up to 8.49 px | Identity 7.937 px, local motion 1.061 px |
| `P9160654` → `P9160655` | Up to 129.796 px | Nine reciprocal raw patches, observed moves at most 2 px | Packaged step 102.893 px, identity 1.000 px |

## Admission rule and limits

Compare the proposed transition from two absolute maps,
`M_after · inverse(M_before)`, against an independent adjacent hypothesis
on the same held-out, reciprocal native correspondences. Rank by median
forward and inverse transfer errors. Require the advantage to exceed the
measurement uncertainty in both directions. This tests geometry against
evidence and permits arbitrarily large motion with adequate support.

If each held-out target location has error bounded by `r` in a chosen
coordinate direction, a transfer-error median changes by at most `r`.
The preference between two hypotheses in that direction is guaranteed
when their measured median difference exceeds `2r`. The same argument
can be applied separately in reverse with its own bound. This assumes
fixed query coordinates and a valid localization bound for the matched
locations. Round-trip consistency alone does not establish that bound.
The archived forward margins are 101.893 px and 6.876 px between the two
hypotheses on the false and genuine transitions respectively. These are
observed margins and do not set a release threshold.

An adjacent matcher must provide positive evidence before its proposal
can replace a distant fit. If both directions abstain or the evidence
cannot discriminate the proposals, report that uncertainty. A transition
contradiction also leaves which of the absolute poses is wrong unresolved.
Composing an adjacent replacement requires an already supported previous
pose in the common canvas. Long-range absolute tracking, crop geometry and
output quality therefore need their own checks. A valid result keeps the
chosen coordinate anchor, finite invertible maps within the existing model
limits, and a crop derived from those admitted maps. The reference output
dimensions supply no geometry target for this diagnostic.

This is a focused admission hypothesis. The archived production estimator
has no positive nearby fit at the genuine transition. Its current outputs
are insufficient for a simpler confidence veto, magnitude cap or
one-outlier change. Current product source is unchanged.

## Nine-original CPU discriminator

Use the existing first anchor and two four-frame windows. All nine files
have frozen byte sizes and digests in `audit.json`. The extra capture on each
side controls the abrupt change and the following unreliable admission.
The protected archive materializes only these nine images. The CubeCL CPU
probe calls the production decoder, thumbnail and pair estimator with scale
6, 5% breathing step limit and ordinary precision, in both directions. It
records the actual transform and confidence for eight anchor pairs and six
neighbor pairs. Zero-confidence product outputs are not scored as measured
motions.

The independent optical probe trains in each direction outside a reserved
grid, matches the native held-out patches reciprocally, and reports transfer
errors both ways. Flat controls and ambiguous evidence give an explicit
abstention. The synthetic self-test checks a known affine move under a gain
change, rejection of a false stationary jump, tie abstention, and inadequate
texture. The actual D150 follow-up reports the measured witness count and
both errors without assuming the synthetic localization bound on real
focus-blurred texture. Names and the ordering are fixture inputs.

```bash
audit=pancake/docs/d150-geometry-audit/motion-admission
scratch=artifacts/d150-motion-audit
mkdir -p "$scratch"
git show 99e74b2:pancake/docs/d150-geometry-audit/breathing-sampling/release-59d0bcb/bilinear/events.jsonl.gz \
  > "$scratch/packaged-events.jsonl.gz"
python3 "$audit/audit.py.txt" --packaged-events "$scratch/packaged-events.jsonl.gz" > "$scratch/audit.json"
taskset -c 2,3 python3 "$audit/acquire-probe.py.txt" --audit "$scratch/audit.json" --output "$scratch/subset"
mkdir -p pancake/examples
cp "$audit/probe.rs.txt" pancake/examples/d150_motion_probe.rs
(
  cd pancake
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
    taskset -c 2,3 cargo build --locked --example d150_motion_probe
)
mapfile -t originals < <(python3 -c 'import json,sys; print(*[r["acquired_path"] for r in json.load(open(sys.argv[1]))["records"]],sep="\n")' "$scratch/subset/acquisition.json")
RAYON_NUM_THREADS=2 taskset -c 2,3 pancake/target/debug/examples/d150_motion_probe \
  "$scratch/estimates.json" "${originals[@]}"
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 taskset -c 2,3 "$reference_python" \
  "$audit/bilateral.py.txt" --self-test > "$scratch/bilateral-self-test.json"
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 taskset -c 2,3 "$reference_python" \
  "$audit/bilateral.py.txt" --acquisition "$scratch/subset/acquisition.json" \
  --trace "$scratch/estimates.json" --archive-audit "$scratch/audit.json" > "$scratch/bilateral.json"
python3 "$audit/direction-replay.py.txt" --trace "$scratch/estimates.json" \
  --bilateral "$scratch/bilateral.json" > "$scratch/direction-replay.json"
rm pancake/examples/d150_motion_probe.rs
```

## Actual nine-original result

The authorized CPU probe ran unchanged `287828a` application source on
logical CPUs 2 and 3 of the Ryzen 9 7950X3D, Linux 6.8.0-147-generic, with
Rust 1.100.0. The temporary Cargo example is the observer and sole source
overlay. Its exact locked development build produced SHA-256
`45cc38fab655c8bc57cafba0bb38e012b023a23c750cc9f5cda582036328f03f`
with Cargo lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The original ZIP and all nine extracted records were verified byte-for-byte.
No full stack ran. [Acquisition receipt](acquisition.json.gz) ·
[Build log](build.log.gz) · [28 estimator observations](estimates.json.gz).

The fixed-grid independent probe recovered the synthetic affine step with
81 reciprocal witnesses in each direction. Median error of the forward
native estimate was about 0.014 px versus 4.353 px for identity. The
stationary false-jump and flat abstention assertions passed as well.
[Self-test](bilateral-self-test.json).

In D150, all 12 ordinary neighbor fits, including their reversals, returned
zero confidence. Only two of the eight ordinary anchor fits were positive.
Those two reproduced the large wrong maps exactly. All eight **source to
first anchor** fits were positive. The two directions are empirically
different estimators because the source texture and interpolation direction
change. Inverting each positive reverse absolute map supplies another
proposal. Differencing the inverted maps measures the implied step on the
same before-to-after coordinates used by the independent test.

| Transition | Held-out forward/reverse | Direct-anchor median error | Inverted reverse-anchor median error | Inverted reverse-anchor max canvas step |
| --- | ---: | ---: | ---: | ---: |
| `P9160580` → `P9160581` | 47 / 44 | 7.937 / 8.084 px | 1.231 / 1.173 px | 10.470 px |
| `P9160654` → `P9160655` | 7 / 7 | 107.062 / 103.573 px | 0.820 / 0.803 px | 1.171 px |

Independent matching in reverse starts at its own fixed native query grid.
Neither direction reuses fitting windows as held-out query centers. The
two evaluations support an inverted reverse candidate at both important
transitions. Near the genuine motion, its maximum canvas step exceeds the
other estimated 8.49–8.67 px step estimates, so the held-out fit does not
certify the canvas extremities. On two small-motion controls in the last
window the forward route produced lower errors. This gives a useful
candidate generator with the current shared estimator. Choosing the
direction solely by confidence or always inverting the reverse fit remains
unjustified. [Bilateral observations](bilateral.json.gz) ·
[Direction comparison](direction-replay.json).

The smallest next product experiment is a forward/inverse-reverse choice
for fixed-anchor Breathing, using independent adjacent transfer evidence to
rank the resulting transition proposals. It needs an abstaining witness
route that works with the shared numerical kernels. A valid candidate must
check an inverted fit against the existing forward model bounds before
publication, since inverting an allowed scale can move beyond the same
interval. The existing first canvas and admitted maps determine the crop.
No crop target, filename, transition magnitude cutoff or confidence change
is justified by these samples. Full-stack trajectory, fused quality, parity
and performance remain separate obligations.

The verified archive was removed immediately after extracting the nine
originals. Those originals were rehashed and removed after the probe. The
source and trace receipts are retained. [Cleanup](cleanup.json).
