# Positive Canon CR3 and full R50 replay plan

## Authenticate the minimum inputs

The frozen target file has SHA-256
`58bb68533311c48696a3a844d80eef2f37a3c1f39649305948c838b7002663dc`
and pins the input manifest at
`dd7a7e8a396d16e952d3d24d258afc1e05b77bb1664b179d3a43a7f66e413fab`.
The manifest gives an individual revision-addressed URL for each original
in Johannes Sood's focus stack, revision
`0f8256edf2d2e3f1f1ea162f78a8bca36eb6915c`, licensed CC BY 4.0.
Read URLs, paths and rights provenance from that manifest. The first and
last rows are the two previously tested Canon originals.

| Manifest row | Name | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| 0 | `frame_000.CR3` | 8,869,034 | `4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca` |
| 49 | `frame_049.CR3` | 8,813,738 | `98fe6e370fc2ab58ba7b45d242a8c2f2a1942404ee0e0524c5cb33048969dc36` |

## Two-original native correctness replay

1. Create separate clean stock and candidate worktrees at the exact
   commits above. Capture commit, tree, equivalent production build
   settings, lock, Rust 1.100.0, affinity and ELF hash/build ID for each
   artifact. Build separate headless release CLIs with
   `cargo +1.100.0 build --offline --locked --release -p pancake-cli --jobs 2`
   in each worktree's `pancake` directory. Disable both Rust wrapper
   variables, set `CARGO_INCREMENTAL=0` and use the same permitted
   two-logical-CPU build affinity for both. A previously sealed stock
   binary is suitable if its full build receipt matches the candidate
   product profile. Keep the tracked sources clean when sealing CLIs.
2. Copy the previously used native `raw_decode_probe.rs.txt` to
   `pancake/examples/raw_decode_probe.rs` of each checkout. If Cargo's
   workspace dependency lint flags unused direct dependencies in the
   example, use the identical anonymous dependency imports from the
   retained DNG helper at both revisions. Archive the final helper bytes.
   Build the helper in each checkout with
   `cargo +1.100.0 build --offline --locked --release -p pancake --example raw_decode_probe --jobs 2`.
   Run `raw_decode_probe bytes "$first" "$last"` under the same granted
   CPU affinity for both, redirecting stdout to a private binary file
   per arm and stderr to its metadata log. The archived probe emits
   native-endian f32 bits, so use little-endian Linux x86-64 when checking
   the earlier digest. Require 5088×3392 U16 and 207,101,952 decoded
   bytes per frame. Split at that byte offset for per-frame digests and
   compare both complete 414,203,904-byte streams with `cmp`. Together they
   require 828,407,808 B of owned disk before TIFFs or build outputs. The earlier
   exact digest was
   `ed3351ff2b8f5f1f3b278c806258170d6a2b89b75da55014f5dc875535c8a0b2`.
   Preserve any change from that historical observation separately.
3. Run the probe again as a separate debug correctness check if a
   source-level witness is required. Build the same example without
   `--release`, then use the retained `observe-validation.py.txt` with
   GDB and a new `PANCAKE_VALIDATION_TRACE` path. Invoke the two-file
   probe in `timing` mode to avoid the bit dump under the debugger.
   Require the symbol to resolve in both binaries, a clean inferior
   exit and two stock `validate_samples` hits attributable to
   `normal_developed_rgb`, with zero such hits in the candidate.
   Reuse the existing DNG and EXR traces as the small and float controls.
   For both the native replay and the full diagnostic, CPUs 0–14 would
   permit 14 existing RAW workers on the recorded host. Each Canon
   raster has 51,775,488 samples, with 790 full 65,536-sample chunks
   and a final 2,048-sample chunk. That passes the 16-chunk threshold.
   A two-CPU-only run has one worker and supplies serial parity only.
4. Run the exact release CLI on both originals with the settings below.
   Capture a real CPU completion and compare the two encoded TIFFs,
   independent RGB16 sample/dimension receipts and both pairs of ordered
   alignment and sequential matrices. The prior output was 5086×3390.
   Use the full-workload settings even for this two-file correctness check.

```sh
"$cli" --input "$first" "$last" --cpu --method hybrid --align \
  --alignment-mode similarity --alignment-ref first --kernel-size 6 \
  --pyramid-levels 8 --scale-factor 6 --slab-size 0 --auto-crop \
  --bit-depth 16 --output "$owned_output"
```

[Previously used Canon probe and commands](../p10-r50-raw-audit/host-aware-workers/README.md)
[Probe source](../p10-r50-raw-audit/host-aware-workers/raw_decode_probe.rs.txt)
[DNG example imports](typed-raw-probe.rs.txt)
[Native validator observer](observe-validation.py.txt)

## One matched full R50 diagnostic

Use a quiet CPU-only window and the same CLI pair once the two-original
results are known. Reuse `compare-r50-local.py.txt` and its unchanged
`diagnose.py.txt` process observer in an owned directory with suffixes
removed. Keep the archived CPU 0–14 affinity, thread environment,
repository campaign lock, same fresh per-arm application caches,
verified pre-read OS file cache and 32 GiB child address-space cap.
Census competing applications before launch and record available memory
and thermal state at both boundaries. Earlier same-host single-arm
observations were roughly 37–42 s. Reserve two to three minutes for
the timed pair after build, parity and acquisition have completed.

```sh
PYTHONPATH="$checkout/qualification/p10-performance" \
  prlimit --as=34359738368 -- taskset -c 0-14 \
  "$frozen_python" "$scope/compare-r50-local.py" \
  --inputs "$inputs" --output "$scope/full-pair" --lease "$scope/quiet-lease.txt" \
  --baseline "$stock_cli" --candidate "$candidate_cli"
```
