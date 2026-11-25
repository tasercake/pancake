# T50 Hybrid resident input reuse

The implementation carries one immutable packed RGB tensor
from its existing luma upload to Hybrid fusion when the registered map is
exact identity and the source already matches the canvas. A frame-local,
consuming handle checks the allocation's CubeCL service, extent, element
type, contiguous shape and stride before fusion. The owner-thread marker
and local lifetime keep it out of proxy and transform caches. Warp,
resize, other methods and slabbing use their existing upload path.

The registration readback and the original per-frame fusion drain remain
in place. Finite-sample validation still runs before accumulation. The
same resident allocation occupies the frame slot already charged by the
registration and fusion budgets, with no second full input allocation.
On rejection or interruption the optional handle drops with the current
frame. Cancellation still settles before publication.

## Test-first CPU proof

At `1e9602c`, the real headless aligned Hybrid fixture preserved TIFF
bytes, progress and completion, but issued two redundant fusion uploads.
The failing assertion required zero. The native A/B at `3f6cd9b` used the
same test binary in both configurations and passed two tests covering
three fixture pairs. The identical canvas/identity case dropped from two
fusion uploads to zero while preserving two original proxy uploads and
two original waits. The translated and mismatched-canvas cases preserved
their existing warp and separate fusion upload. All paired TIFF bytes,
transforms, backend receipts and progress streams agreed. The
uninstrumented core at `37af4e2` passed 14 CPU job, fusion and registration tests,
including aligned Hybrid cancellation and late source corruption.

## Integrated headless verification

The tested production revision is `9629212` on `c145327`. With pinned
Rust 1.100.0 and the original lock, all 119 tests across 18 headless
library, binary and CPU integration executables passed. The exact
scoped Clippy gate with the generated dependency policy and `-D warnings`,
workspace rustfmt, and production-source inventory checks passed.
The earlier lint failures and compiler errors are retained. The final
source uses the public stride iterator and places the crate-private
handoff at the registration module boundary. The opt-in timing and
baseline override are confined to the `3f6cd9b` diagnostic source.

## Same-binary physical T50 diagnostic

On the RTX 3090 at PCI `0000:01:00.0` with driver `580.178.04`, a
locked release built from `3f6cd9b` ran both arms under the original
32-CPU affinity and 28 GiB no-swap scope. The binary SHA-256 was
`1cca8258d6c819193af16e74bea7702c737d2cb997296cadecfda3b6cbb00e49`.
It read the sole authenticated T50 set, checked all 52 file hashes before
and after execution, and selected the same 50 RGB16 originals and frozen
Hybrid settings. Each arm used a new application cache. The baseline ran
first, with only resident reuse disabled.

| Physical observation | Baseline | Resident |
| --- | ---: | ---: |
| Parent wall through completed export and exit | 72.661 s | 57.064 s |
| Trace owner interval | 72.519 s | 56.913 s |
| Full RGB luma uploads | 50 | 50 |
| Full RGB fusion uploads | 50 | 0 |
| Fusion upload caller time | 13.073 s | 0 s |
| Existing fusion waits | 50 | 50 |
| Fusion wait caller time | 3.002 s | 0.327 s |
| Decode host interval | 30.135 s | 30.211 s |
| Luma upload host interval | 20.327 s | 20.433 s |
| Observed process-tree RSS peak | 1,545,895,936 B | 1,546,510,336 B |
| Observed attributable VRAM peak | 2,518,323,200 B | 2,518,323,200 B |
| Maximum VRAM sample gap | 65.293 ms | 75.400 ms |

Both completed on physical wgpu Vulkan with no fallback. Their exact
3998×5998 RGB16 TIFF bytes, decoded pixels and 50 identity transforms
match the earlier unoptimized `12f2151` arm. The entire export SHA was
`6160722086e2153b4dd3d8e3810dd22e156669fcafaa15fc72a3e88968b39302`,
and the decoded pixel SHA
`f54746784bcc278da2df734e9f034053a3b54105ddb2a30046baf6dab07a80a3`.
Completion event contents were byte-identical across both arms.

The resident route used 14.4 GB fewer logical full RGB upload bytes.
The 15.598 s wall reduction is 21.47% in this one sequential diagnostic
pair. The shorter existing wait is consistent with less preceding queue
work, while the call spans cannot partition device transfers and
execution. Holding one same-job tensor across the estimate did not
raise the sampled VRAM peak on this run. That observation leaves formal
device-memory performance pending because both cadence gaps exceed
20 ms. The pair also leaves frozen warm and cold speed comparisons open.

The verified receipt and raw archive retain the locked commands, source
patches, exact red and green logs, physical phase/identity witnesses,
inputs and cleanup. The shared input staging owner retains its separate
generation and authenticated cleanup receipts.
