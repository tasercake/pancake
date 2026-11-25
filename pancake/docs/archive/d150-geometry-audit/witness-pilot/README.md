# Staged passive bilateral witness pilot

Diagnostic-only source is staged on `3cd43b5756bc7e6036a46a6be2824e45a094401e`.
The test calls the existing CubeCL `correlation_moments` kernel on two
fixed sampling lattices per region. It fits exposure gain and offset on
one lattice, validates on the other, and exchanges query and source for
the second direction. It checks a known signed translation in a continuous
two-axis texture with a photometric gain change, an aperture with two
indistinguishable horizontal maps, and motion confined to one column of
witness regions. The geometry oracle and test coordinates are confined to
the diagnostic. [Witness design](../witness-design/README.md).

The original short window ended before a fresh build could complete. In
the later authorized window, exact `8410863` production source and lock
matched `3cd43b5` and compiled on CPUs 2 and 3 with Rust 1.100.0. The real
CubeCL CPU synthetic test passed all three controls. Cargo recorded a
1m07s build and 0.06s test duration. The source and sampled geometry are
synthetic. Those durations supply no D150 performance evidence. [Synthetic test log](synthetic-8410863.log.gz).

## Minimal synthetic replay

Replay in an isolated worktree with its original application files clean.
The overlay is temporary and uses the private normal sampling kernel,
keeping all diagnostic symbols out of the released product. The synthetic
frame and translations guarantee interpolation support for all sampled
tiles. The fixed-layout helper and rounding screen are smoke-test machinery.
They require general common-overlap handling, independent uncertainty
calibration and bounded batching before use for production or cost analysis.

```bash
repo=$(pwd)
proof=$repo/pancake/docs/d150-geometry-audit/witness-pilot
kernel=$repo/pancake/src/registration/kernels.rs
overlay=$repo/pancake/src/registration/witness_pilot.rs
scratch=$repo/artifacts/d150-witness-pilot
git diff --exit-code -- "$kernel"
test ! -e "$overlay"
mkdir -p "$scratch"
cp "$kernel" "$scratch/clean-kernels.rs"
(
  set -e -o pipefail
  trap 'cp "$scratch/clean-kernels.rs" "$kernel"; if [ -f "$overlay" ]; then unlink "$overlay"; fi' EXIT
  cp "$proof/witness_pilot.rs.txt" "$overlay"
  printf '\n#[cfg(test)]\n#[path = "witness_pilot.rs"]\nmod witness_pilot;\n' >> "$kernel"
  cd "$repo/pancake"
  CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
    taskset -c 2,3 cargo test --locked --lib witness_bilateral_motion_and_ambiguity -- --nocapture \
    2>&1 | tee "$scratch/synthetic.log"
)
```

## Real-luma observation

The optional test module takes the authenticated nine-original receipt and
the archived directional candidates. It decodes each interval sequentially
and calls the product's scale-6 luma prefilter at 384×288. A common native
pixel-center transform projects all candidates into that sampled extent.
All three maps and their inverses must have complete support for each
scored tile. Within each cell, the strongest proposal must beat both rivals
in both cross-fits and directions. Positive gain, an informative contrast
denominator and the small floating-point screen are scoped pilot checks.
The test records every cell and the two-axis spread of candidate support.
The per-record device launches measure this diagnostic only and give no
grouped-kernel, full-estimation or whole-job performance claim.
[Native independent comparisons](../motion-admission/README.md).

After the synthetic test, the optional replay requires these additional
paths and module in the same temporary overlay. Use the verified acquisition
output and a fresh result path. Both test filters can run in this overlay.

```bash
motion=$repo/pancake/docs/d150-geometry-audit/motion-admission
stage=$scratch/subset
taskset -c 2,3 python3 "$motion/acquire-probe.py.txt" --audit "$motion/audit.json" --output "$stage"
real=$repo/pancake/src/registration/real_pilot.rs
test ! -e "$real"
(
  set -e -o pipefail
  trap 'cp "$scratch/clean-kernels.rs" "$kernel"; for path in "$overlay" "$real"; do if [ -f "$path" ]; then unlink "$path"; fi; done' EXIT
  cp "$proof/witness_pilot.rs.txt" "$overlay"
  cp "$proof/real_pilot.rs.txt" "$real"
  printf '\n#[path = "real_pilot.rs"]\nmod real_pilot;\n' >> "$overlay"
  printf '\n#[cfg(test)]\n#[path = "witness_pilot.rs"]\nmod witness_pilot;\n' >> "$kernel"
  cd "$repo/pancake"
  export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 RAYON_NUM_THREADS=2
  taskset -c 2,3 cargo test --locked --lib witness_bilateral_motion_and_ambiguity \
    -- --nocapture --test-threads=1 2>&1 | tee "$scratch/synthetic-recheck.log"
  PANCAKE_WITNESS_ACQUISITION="$stage/acquisition.json" \
    PANCAKE_WITNESS_DIRECTIONS="$motion/direction-replay.json" \
    PANCAKE_WITNESS_OUTPUT="$scratch/real.json" \
    taskset -c 2,3 cargo test --locked --lib witness_real_d150_proxy_diagnostic \
    -- --nocapture --test-threads=1 2>&1 | tee "$scratch/real.log"
)
```

## Measured CPU diagnostic

The actual overlay compiled with the same product revision and lock on the
Ryzen 9 7950X3D, Linux 6.8.0-147-generic, on logical CPUs 2 and 3. Both tests
passed again after replacing the pilot's fixed synthetic extent with its
checked pair extent. The real test linked the existing decoder,
registration prefilter, affine sampler and CubeCL moment kernel. It used
the six authorized intervals in the nine-source acquisition and the exact
old direct and reverse-anchor maps. The third hypothesis was identity.
[Acquisition](acquisition.json.gz) · [Synthetic recheck](synthetic-recheck.log.gz) ·
[Real test](real.log.gz).

| Interval | Direct strict wins | Inverse-reverse strict wins | Identity strict wins | Two-axis inverse-reverse support |
| --- | ---: | ---: | ---: | --- |
| `579→580` | 0/9 | 6/9 | 0/9 | Yes |
| Genuine `580→581` | 0/9 | 8/9 | 0/9 | Yes |
| `581→582` | 0/9 | 8/9 | 0/9 | Yes |
| `653→654` | 0/9 | 2/9 | 0/9 | No |
| False direct jump `654→655` | 0/9 | 1/9 | 0/9 | No |
| `655→656` | 1/9 | 0/9 | 2/9 | No |

Each strict win requires lower cross-fitted error than both alternatives
in both directions and both lattice fits. A tile with insufficient
comparable gain or contrast has no winner. On `654→655`, each of the three
cells where the direct estimate and both rivals had comparable losses
favored both alternatives over direct in all four fits. A single cell
distinguished the two alternatives, leaving the whole-canvas result
unresolved. The separately measured native optical correspondences support
that distinction. On `580→581`, the proxy retains detectable information
for corroborating the genuine nonidentity step. These are sampled
witness results, without a validated global pose, crop, or fused output.
[Every directional cell loss and dispatch count](real-results.json.gz).

This unbatched CPU diagnostic issued 108 small moment dispatches and
readbacks per interval. Witness cost across six intervals was 134.81 ms
total and 19.31 ms median, with 9.92–54.26 ms range. The median of the last
five was 18.47 ms. Separate JPEG decode, prefilter and pair upload consumed
8.053 s in the development-profile test. These observations exclude reverse
candidate estimation, represent two cores and supply no frozen performance
credit. The prior 5.32 ms per-edge warm headroom was from a different exact
release. This launch pattern cannot support a throughput claim. A later
bounded batched layout would still need its own backend and full-job proof.

All nine JPEGs were rehashed and removed. The ZIP was verified before
extraction and removed by acquisition. Product source and the frozen
suite remain unchanged. [Cleanup and executable receipt](cleanup.json).
