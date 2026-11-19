# Native registration variance floor

The exact Linux CPU mutation shard 9/16 at `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa` recorded a survivor at `src/registration/kernels.rs:309:26`. The saved patch replaces multiplication by division in the first Pearson variance check. The second check stays intact. The kernel always supplies the reference thumbnail as the first operand.

The public CPU oracle at `82801b343c8e417b06942c00bb091c5169534948` generates the same analytic 113 × 95 grayscale scene at a known reference-to-source translation of (+3, −3) pixels. The normal-exposure control recovers both signed coordinates at confidence above 0.5. Darkening the exact reference and source by gains of 0.002 and 0.008 retains the translated texture while reducing the reference below the useful contrast floor. The test requires the public estimate to abstain with the identity map and zero confidence within f32 precision. Both branches use the real `thumbnail` and `estimate_pair` APIs and the CPU CubeCL backend.

The same-size proxy preserves the sample coordinates. An independent double-precision calculation applied the separable [1, 4, 6, 4, 1] prefilter to the analytic scene and evaluated the population variance over the matcher’s interior sample grids. The shift and filter support fit inside the border. Multiplying by the chosen exposure gains gives the following values.

| Sample spacing | Samples | Reference variance | Aligned source variance | Identity source variance | Identity correlation without the floor |
| --- | ---: | ---: | ---: | ---: | ---: |
| 2 pixels | 1591 | 4.3337e−8 | 6.9339e−7 | 6.8828e−7 | 0.84651 |
| 4 pixels | 418 | 4.2985e−8 | 6.8776e−7 | 6.9906e−7 | 0.83748 |

The exact saved mutant passed the exposure control. It then reported confidence 1.0 and accepted (+3, −3) from the dark pair, so the oracle exited 101. The restored source passed 23 native library tests and all four public CPU registration integration tests. The focused strict Clippy, format and reviewed source inventory checks passed. The original mutation shard result remains a historical observation. This replay covers the saved operation on the Linux CPU backend.

## Replay

Start in `pancake/` of an isolated checkout of the test revision. Decompress the saved patch to a scratch path and apply it with `patch --batch --fuzz=0 -p0`. Keep a verified stock copy of the kernel source and restore it even if the mutated test fails.

```sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER taskset -c 4,5 \
  cargo +1.100.0 test --offline --locked -p pancake --test cpu_registration_contract \
  cpu_registration_requires_detail_in_the_reference -- --exact --test-threads=1
```

The restored run uses `--lib --test cpu_registration_contract -- --test-threads=1` without the exact name filter. Scoped Clippy uses the repository dependency policy generator’s effective configuration and `clippy --offline --locked -p pancake --test cpu_registration_contract -- -D warnings`. These commands ran on Linux x86-64 with two direct Cargo jobs on CPUs 4 and 5 of the Ryzen 9 7950X3D.

[Original patch](saved.diff.gz) · [Typed shard result](shard-outcome.json) · [Native mutated run](red.log.gz) · [Native restored run](green.log.gz) · [Exact inputs and outputs](identities.json)
