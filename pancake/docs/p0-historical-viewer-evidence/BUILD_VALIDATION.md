# Build validation

Verified locally on Apple M4 Pro (Metal), 2026-11-16:

- Rust 1.100.0 release build completed; executable is approximately 56 MiB.
- `cargo tree -i wgpu` resolves one wgpu 30.0.2 shared by CubeCL and Slint.
- Two meaningful CPU contract tests passed. First tests were observed failing
  for stub implementations before implementing stride/exposure behavior.
- Real GPU benchmark passed at 257×193, 1920×1080, 6000×4000, maximum CPU-oracle
  preview difference zero across validated pixels. Three repetitions are only
  initial integration evidence, not adequate performance statistics.
- Native Slint GUI synthetic four-image smoke and two real JPEG smoke both
  exited successfully after four seconds, with readback/oracle check on first
  image. UI interactions and presentation were not visually verified by builder.

Raw artifacts are ignored under `artifacts/`: `first-benchmark.json`,
`gui-smoke.stderr`, `real-jpeg-smoke.stderr`, and `tests.log`.
The separate performance audit supersedes initial three-sample timing evidence.

Programmatic rendering smoke was added after native UI attachment could not
be verified. It captures Slint renderer output before/after actual application
callbacks; it does not claim native input or window-manager presentation proof.
Smoke errors now propagate to a nonzero exit status. Two new tests were observed
failing before implementation for missing/failed completion and unchanged/missing
rendered pixels. Full test count is now four.
