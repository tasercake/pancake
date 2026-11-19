# Preregistered 11-source D150 anchor and skip-edge diagnostic

This plan pins the investigation from `af964f5dab976a276e1b91cf4db0b2d63e8063c1`. It tests whether the independent optical witness sees a link to the First anchor and whether the existing shared matcher can produce supported alternatives across the late ambiguous interval. It gives no full D150, production geometry, quality, or performance result. The adapters have synthetic and compile-check evidence. Real data execution remains pending. [Anchor evidence](../anchored-recovery-review/README.md) · [Exact manifest and evaluation plan](plan.json).

## Inputs and allowed comparisons

| Fixed cohort | Frozen zero-based indices | Original captures |
| --- | --- | --- |
| First-anchor neighborhood | 0, 1, 2 | 0516, 0517, 0518 |
| Genuine-step control | 63, 64, 65, 66 | 0579, 0580, 0581, 0582 |
| Late ambiguous interval | 137, 138, 139, 140 | 0653, 0654, 0655, 0656 |

The exact named JPEGs, sizes and bit digests appear in `plan.json`. Fresh edges are `0→1`, `0→2`, `1→2`, `137→139`, `137→140`, and `138→140`. Replay the three old adjacent edges in each of the two four-source controls against their fixed prior candidates. `64→65` is the genuine-motion control and `138→139` is the false-jump control. Keep the other edges in the report even if they abstain. The arm needs at most 12 fresh directed product estimates and never scans new edges in response to a result. [Prior six-edge results](../witness-batching/real-summary.json).

Estimate the fresh directions on the shared CPU path with Breathing, scale six, 384×288 proxy, 5% maximum scale step and ordinary precision. Each edge has at most three proposed maps in the native `before→after` coordinate system. Use the positive forward estimate, the inverse of the positive reverse estimate, and an identity comparator. Reject a measured proposal with nonfinite or singular coefficients or an isotropic scale outside the existing forward interval after inversion. A zero-confidence result leaves the slot unavailable. Collapse exact duplicate maps and keep the slot provenance. The old controls use the same archived direct and inverse-reverse proposals plus identity. Both separate optical fits from the existing bilateral tool are useful controls. They cannot become selected product proposals in this trial. Record every proposal status, matrix, confidence, measurement and disagreement.

## Prespecified independent native screen

An edge abstains if the data are insufficient, the directions disagree, a leave-one-cell-out check changes the result, or the candidates tie. Identity alone supplies no proof of stationary motion. Label an edge `native_supported_proposal` only after the entire screen passes. Record absolute forward and inverse medians and per-point errors even when a comparative check abstains. The packed witness can record corroboration or contradiction when its independent proxy cells support a uniquely ranked proposal. It uses at most nine cells, two lattices, three proposals and 54 queries per direction. Keep common valid overlap, the existing cross-fit comparison and two-axis footprint, and the `1 / 65,536` floor. Native and proxy disagreement stays unresolved. A clean local result needs its own verified link to a trusted pose. In particular, `1→2` alone never connects either node to First.

The control comparison must recover a supported nonidentity alternative at `64→65` and reject the historical direct jump at `138→139`. Report a control miss as inconclusive for the proposed recovery. Preserve its observations and the original bounds for review.

## Owned stage and later commands

The prior owner removed the authenticated archive and all nine originals. The new `D03` download is exactly 53,459,291 bytes before extraction and the 11 verified originals total 3,582,818 bytes. ZIP and originals can temporarily coexist at 57,042,109 bytes. Use the exact ordered entries and SHA-256 values in `plan.json`. The frozen manifest, source revision, rights, archive digest and download limit are locked there. The manifest describes the intended stage. [Previous cleanup](../witness-batching/real-cleanup.json).

`anchor_skip_trial.py prepare` validates the exact preregistration and seven pinned files. It generates an exclusive stage containing a copy of the old bounded acquisition with an eleven-record guard and a CPU probe that uses the existing Rust decode, thumbnail and estimator functions. The original scripts stay pinned. `verify-inputs` checks the exact order, canonical owned paths, byte counts and SHA-256. `score` reuses the bilateral and independent native tracker, rehashes the originals after scoring and emits an optional packed input list. Edges without three actual distinct maps stay visible as unavailable for the fixed packed implementation. The packed control patch changes the diagnostic input roster and omits repeated timings. The original serial and packed moments keep their bit comparisons. A three-map edge with zero common bidirectional cells records `no_common_bidirectional_cells` and zero dispatches, then the scorer continues through the remaining edges. [Existing bounded acquisition](../motion-admission/acquire-probe.py.txt) · [Existing product probe](../motion-admission/probe.rs.txt) · [Pinned overlay](../witness-batching/real-overlay.patch.gz).

The following commands are for the later authorized trial from a clean review checkout. Use the exact reference Python with OpenCV 4.14.0 and the granted CPU list. Build the case-owned example as a locked debug target before acquiring any data.

```bash
set -euo pipefail
repo="$PWD"
folder="$PWD/pancake/docs/d150-geometry-audit/anchor-skip-prereg"
plan="$folder/plan.json"
trial="$folder/anchor_skip_trial.py"
scratch="$PWD/artifacts/d150-anchor-skip-011/run-001"
acquisition="$scratch/materialized/acquisition.json"
export CARGO_TARGET_DIR="$scratch/cargo-target" CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0 OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1
python3 "$trial" --plan "$plan" prepare
mkdir -p pancake/examples "$scratch/bin"
example="$repo/pancake/examples/d150_anchor_skip_probe.rs"
(
  test ! -e "$example"
  cp "$scratch/d150_anchor_skip_probe.rs" "$example"
  trap 'rm "$example"' EXIT
  cd pancake
  taskset --cpu-list 4,5 cargo +1.100.0 build --locked --offline --example d150_anchor_skip_probe
)
cp "$CARGO_TARGET_DIR/debug/examples/d150_anchor_skip_probe" "$scratch/bin/"
sha256sum "$scratch/bin/d150_anchor_skip_probe" > "$scratch/probe.sha256"
timeout --signal=TERM --kill-after=5s 180s python3 "$scratch/acquire-probe-011.py" \
  --audit "$plan" --output "$scratch/materialized"
python3 "$trial" --plan "$plan" verify-inputs
timeout --signal=TERM --kill-after=5s 150s taskset --cpu-list 4,5 \
  "$scratch/bin/d150_anchor_skip_probe" --plan "$plan" --acquisition "$acquisition" \
  --output "$scratch/product-proposals.json"
timeout --signal=TERM --kill-after=5s 150s taskset --cpu-list 4,5 \
  "$reference_python" "$trial" --plan "$plan" score --trace "$scratch/product-proposals.json" \
  --output "$scratch/native-holdouts.json"
```

Only consider packed corroboration if the native controls reproduced and the emitted list has actual distinct triples. Run it in the same clean, disposable checkout. Restore both overlays before any production or assurance work in that checkout.

```bash
set -euo pipefail
jq -e '.controls.status == "controls_reproduced"' "$scratch/native-holdouts.json"
jq -e '.results | length > 0' "$scratch/packed-directions.json"
archived="$PWD/pancake/docs/d150-geometry-audit/witness-batching/real-overlay.patch.gz"
control="$folder/packed-control.patch.gz"
zero_overlap="$folder/packed-zero-overlap.patch.gz"
python3 "$trial" --plan "$plan" verify-inputs
(
  set -e -o pipefail
  gzip -dc "$archived" | git apply
  if ! gzip -dc "$control" | git apply; then gzip -dc "$archived" | git apply -R; exit 1; fi
  if ! gzip -dc "$zero_overlap" | git apply; then
    gzip -dc "$control" | git apply -R
    gzip -dc "$archived" | git apply -R
    exit 1
  fi
  cleanup() {
    gzip -dc "$zero_overlap" | git -C "$repo" apply -R
    gzip -dc "$control" | git -C "$repo" apply -R
    gzip -dc "$archived" | git -C "$repo" apply -R
  }
  trap cleanup EXIT
  cd pancake
  PANCAKE_WITNESS_ACQUISITION="$acquisition" \
  PANCAKE_WITNESS_DIRECTIONS="$scratch/packed-directions.json" \
  PANCAKE_WITNESS_OUTPUT="$scratch/packed-corroboration.json" \
    taskset --cpu-list 4,5 cargo +1.100.0 test --locked --offline --lib \
      packed_witness_real_d150_proxy_diagnostic -- --test-threads=1
  cd ..
)
python3 "$trial" --plan "$plan" verify-inputs
```

Preserve the exact binaries, commands, environment, exit codes, stage hashes and all abstentions. The previous six-edge CPU diagnostic completed in 8.92 seconds including proxy preparation and repeated parity samples. Allow a cold two-job debug build of roughly 2–5 minutes and plan 15–90 seconds for the expanded CPU observations. The separate authorized archive transfer has a 180-second watchdog. These are scheduling estimates. The named owner checks that every reader is terminal, rehashes the originals, removes only the owned ZIP and originals, and retains the input receipts, output records and diagnostic source. No original file enters Git.

## Adapter verification

The old synthetic screen admitted the 0.75 px rival and failed the new abstention witness. The new screen passed all seven native-render and identity controls. The generated product probe passed the locked offline debug check. The pinned serial-versus-packed native CPU test preserved its exact decisions after applying both diagnostic overlays. The real edges and inputs remain unrun. [Verification receipt](adapter-verification.json) · [Raw logs and generated controls](adapter-verification.tar.gz).

## Empty common support

The exact 384×288 diagnostic grid has no common cell for the synthetic three-map comparison containing the inverse of `0.953 y − 124`. The old path stopped the report with the one-to-54-record error. The new diagnostic guard records an unresolved edge with zero dispatches and continues. Both native synthetic tests pass. The nine-cell neighbor reproduces all 540 serial/packed moment bits and its complete report is byte-identical before and after the guard. The same result follows the empty edge in the new run. The base overlay, packed kernel, fixed grid and preregistration retain their earlier hashes. [Native proof](zero-overlap-verification.json) · [Raw red and green](zero-overlap-verification.tar.gz).

## Real borrowed CPU trial

The exact locked CPU diagnostic at `914ec559fdfcea07465bd02e65dc8e2e3e5f977c` completed once. The borrower verified the original owner's acquisition and archive-removal receipts, copied exactly 3,582,818 bytes across 11 prescribed JPEGs into a private stage, and preserved the owner's hashes, paths, modes, inodes and modification times. The first preflight used a flat lender path and stopped before reading or copying a JPEG. The corrected path used each frozen manifest-relative name. No diagnostic result was retried. The final source and private-copy rechecks passed. All 11 private copies were rehashed and removed. [Source custody and raw receipts](borrowed-cpu-verification.json).

All 12 fresh directed product estimates returned zero confidence. Each of the six fresh edges remains unresolved. The `0→1` and `0→2` anchor checks each retained one reciprocal witness per direction. `1→2` retained four in each direction but supplied no positive product proposal or link to First. The three late skip edges retained witness counts of `6/5`, `2/4`, and `5/5`, with both directed candidates unavailable on every edge. The archived genuine-region edges `64→65` and `65→66` supported the inverse-reverse proposals.
