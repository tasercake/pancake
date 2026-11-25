# Shared registration luma lattice

The prototype at `7e29d6e` caches the clamped RGB-to-luma sample for each
shared location in the thumbnail's two-pixel halo. The output kernel
preserves the existing row-major 25-tap summation. The same `CubeCL`
kernels serve CPU and GPU. Checked arithmetic validates the halo and
the buffer size. A thumbnail whose existing legal budget or device
cannot hold the extra buffer still uses the original sampling route.
The change leaves the caller's cancellation checkpoints and completed
readback boundary in place. [Source and host](source-and-host.txt)

The first test at `2e2e7c2` ran the CPU thumbnail through real
device readback. Its exact-budget f32 parity assertion passed, and
it failed because the equally valid ample-budget request launched only
`smooth_luma`. The candidate issues the lattice sampler and filter in
order and returns the same f32 bits. This uses the existing `CubeCL`
launch observer alongside complete readback.
[Actual red](test-red.log.gz) [First green](test-first-green.log.gz)

All 24 library, 13 CPU job, and one CPU registration integration tests
passed on Rust 1.100.0. New cases cover single-pixel axes, tiny, odd
and resized dimensions, independent clamped RGB luma, exact old and
new working budgets, device page bounds, and a full-width halo with
indices beyond u16. Focused registration, job, and library-test Clippy
passed with the reviewed dependency configuration. The broader
all-target check flagged 20 existing errors in
`tests/cpu_fusion_contract.rs` at the starting revision.
[CPU tests](cpu-final.log.gz) [Library unit-test lint](clippy-lib-tests.log.gz)
[Focused lint](clippy-final-targeted.log.gz) [All-target log](clippy-final.log.gz)

## Bounded CPU observation

The public thumbnail entry processed one generated 384×256 f32 RGB
fixture with the native CPU runtime. A single warmed process alternated
the two exact budget routes on Ryzen 9 7950X3D logical CPUs 2 and 3.
Round zero initialized each route and twelve subsequent requests per
route were measured from call to completed readback. The median fell
from 13.707 ms to 6.270 ms, a 0.457 ratio. The synthetic observation
covers one thumbnail stage. Frozen whole-job and physical GPU
verification remain pending for this prototype.
[Raw samples](samples.csv) [Summary](summary.json)
[Probe source](thumbnail-probe.rs.txt) [Build and execution](thumbnail-probe.log.gz)

The exact focused CPU replay from `pancake/` is below. The Clippy
configuration comes from the repository's locked dependency policy.

```sh
CARGO_INCREMENTAL=0 taskset -c 2,3 cargo +1.100.0 test --offline --locked \
  -p pancake --jobs 2 --lib --test cpu_registration_contract --test cpu_job
CLIPPY_CONF_DIR="$PWD/target/dependency-policy/clippy" CARGO_INCREMENTAL=0 \
  taskset -c 2,3 cargo +1.100.0 clippy --offline --locked -p pancake --jobs 2 \
  --lib -- -D warnings --test
```

## Full R50 follow-up
