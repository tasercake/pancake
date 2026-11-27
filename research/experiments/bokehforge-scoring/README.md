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
