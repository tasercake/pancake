# BokehForge warm scoring

Build BokehForge and its native extensions independently in each checkout's
Python environment. Use the same machine, device, dependencies, and local D29
`InfinigenDefocus.zip` for both timing runs. For attention-weighted CUDA
scoring, prepare the [local Inspection model assets](../../../bokehforge/docs/benchmark.md#install-the-inspection-models)
before running the harness.

```sh
baseline=/path/to/baseline-checkout
candidate=/path/to/candidate-checkout
d29_zip=/path/to/InfinigenDefocus.zip
harness="$candidate/research/experiments/bokehforge-scoring/compare.py"

uv sync --project "$baseline/bokehforge" --frozen --extra inspection
uv sync --project "$candidate/bokehforge" --frozen --extra inspection

cd "$candidate"
"$baseline/bokehforge/.venv/bin/python" "$harness" run "$d29_zip" \
  --device auto --output artifacts/timing/baseline.json
"$candidate/bokehforge/.venv/bin/python" "$harness" run "$d29_zip" \
  --device auto --output artifacts/timing/candidate.json
"$candidate/bokehforge/.venv/bin/python" "$harness" compare \
  artifacts/timing/baseline.json artifacts/timing/candidate.json
```

The default fixture uses six 4000 × 4000 scenes. The harness sorts the
all-in-focus PNG paths and selects every `reference_count // count` entries
until it reaches the requested count. It bicubically resizes each reference,
decodes it to linear sRGB, and creates a delivery with a local blur and two
channel adjustments. Pass `--count` and `--size` for a smaller run. Use
`--device cpu --no-attention` for a smoke test without model files.

The first scene warms the process. Both reports include its output and time.
The warm statistics cover the following scenes, each with a fresh reference
and delivery. `scoring_seconds` comes from the bench scorer.
`wall_seconds` also includes delivery contract validation. Both measure
scoring with a summary explanation. ZIP reads, fixture preparation, hashing,
and report writes happen outside the timed interval.

The comparison checks the score versions, profile, ordered cohort, input
fingerprints, scoring and reference statuses, every functional result, and
each Inspection map digest. For functional parity across two different device
selectors, pass `--allow-device-change` to `compare`. The result hash covers
the complete score row and summary explanation with the timer removed. Failed
scores retain their status and error in the JSON report. Scoring failures or
parity mismatches return a nonzero exit status.

For migrations that permit small numerical differences, pass
`--include-results` to both timing runs and `--tolerant` to `compare`. The
comparison uses an absolute limit of 0.01 points on the 100-point scale
for scene, overlap, and cohort scores. Family damage, diagnostic values,
and per-tile damage use an absolute limit of 0.0001. It checks alignment,
tile grid geometry and selection, statuses, and input and Inspection digests
for equality and verifies each stored result against its own digest.

When comparing an alternative Inspection inference runtime, pass
`--include-inspection-maps` to both attention-weighted runs and
`--tolerant --allow-generated-map-change` to `compare`. The reports store the
original model-grid maps after timing. The comparison verifies each map
against the digest in its own report and prints per-scene mean absolute error,
maximum absolute error, and overlap among the highest-ranked 1% of pixels.
The regular score and measurement limits still apply.

Pass `--allow-alignment-trace-change` with `--tolerant` to investigate numeric
changes in alignment search traces. The comparison reports trial hypothesis
and regional sample count changes and the largest floating-point difference
at corresponding positions in the traces. It continues to check alignment
outcomes and selected hypotheses. Pass `--allow-native-rounding` with `--tolerant` to
allow up to 0.0001 absolute difference in selected placement geometry and
attention summaries. Embedded attention data must decode to identical bytes.

## Synthetic scorer parity

The synthetic harness calls `scoring.measure` and `bench.score_saved_output`
in each installation. It covers an exact match, an exact crop, detail and
color changes, rotated partial support, read-only strided inputs, input
range and Lab boundaries, and a failed match. It checks all four explanation
levels and scoring without an explanation. The attention cases use a
precomputed, unaligned heatmap, so this run needs no model files or network
access.

```sh
parity="$candidate/research/experiments/bokehforge-scoring/parity.py"
"$baseline/bokehforge/.venv/bin/python" "$parity" run \
  --device cpu --output artifacts/parity/baseline-cpu.json
"$candidate/bokehforge/.venv/bin/python" "$parity" run \
  --device cpu --output artifacts/parity/candidate-cpu.json
"$candidate/bokehforge/.venv/bin/python" "$parity" compare \
  artifacts/parity/baseline-cpu.json artifacts/parity/candidate-cpu.json
```

Repeat with `--device cuda` in both CUDA-equipped installations to check
the accelerated damage path. The synthetic report retains the inputs,
scores, explanations, and benchmark cohort aggregation. The comparator
prints the largest absolute score and measurement differences and returns
a nonzero exit status for a mismatch. The synthetic comparison also accepts
`--allow-alignment-trace-change` and `--allow-native-rounding`. Precomputed
Inspection map digests remain part of its equality checks.
