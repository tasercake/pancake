# Native Advanced Laplacian blue detail

The exact Linux mutation shard 9/16 at `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` recorded a survivor at `src/fusion/advanced/weights.rs:68:53`. The saved operation subtracts the blue term from the RGB luminance projection.

The public CPU test uses two 9 × 7 checkerboards. Both alternate red between 0.25 and 0.75 over green at 0.5. The first has constant blue at 0.5. The second has an in-phase blue checkerboard at the same values as red. Standard Rec. 601 gives blue a positive contribution, so its in-phase variation increases luminance contrast everywhere the red checkerboard has a nonzero difference. Each pixel, including the borders, has a contrasting neighbor and the spatial average preserves this strict ordering. Subtracting blue instead reduces the magnitude because red has the larger coefficient. The test uses this ordering and exact original RGB samples as its oracle.

The public Laplacian accumulator is configured with a three-pixel support window and zero pyramid reductions. It selects the unmodified RGB coefficient of the stronger focus plane at every coordinate. The test supplies both orders and expects the complete red-and-blue input image each time.

The saved mutant compiled and failed the first ordering with red-only RGB `[0.25, 0.5, 0.5]` at the first pixel, where the red-and-blue RGB is `[0.25, 0.5, 0.25]`. After restoring the weights source byte-for-byte, the new test passed both orders and the full focused run passed 23 native library tests and five CPU Advanced integration tests. Focused strict Clippy, formatting and the reviewed source inventory check passed. The original shard remains a historical observation. This replay covers the saved operation on the Linux CPU backend.

## Replay

Use the test revision from the evidence manifest in a clean isolated checkout. From `pancake/`, decompress the saved patch to a scratch file and apply it with `patch --batch --fuzz=0 -p0`. Keep a verified stock copy of the weights source and restore it even if the mutated test fails.

```sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER taskset -c 4,5 \
  cargo +1.100.0 test --offline --locked -p pancake --test cpu_advanced_fusion \
  cpu_laplacian_prefers_in_phase_blue_detail -- --exact --test-threads=1
```

The restored run uses `--lib --test cpu_advanced_fusion -- --test-threads=1` without the exact name filter. Scoped Clippy uses the repository dependency policy generator’s effective configuration and `clippy --offline --locked -p pancake --test cpu_advanced_fusion -- -D warnings`. These commands ran on Linux x86-64 with two direct Cargo jobs on CPUs 4 and 5 of the Ryzen 9 7950X3D.

[Original patch](saved.diff.gz) · [Typed shard result](shard-outcome.json) · [Native mutated run](red.log.gz) · [Native restored run](green.log.gz) · [Exact inputs and results](identities.json)
