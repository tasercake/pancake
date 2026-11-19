# Reuse the registration RGB upload for slab-free exposure fusion

The exact product candidate `481d8052e288c54cf027808b3eaad1ea5655e128`
extends the existing frame-local RGB handoff to slab-free Exposure Fusion.
It branches from `e81079dee4078d7a1149da0f5950aab8c89f8d44`. The Cargo
lock and all shared numerical kernels are identical. An existing host job
test now also exercises early cancellation and late corruption on aligned
Exposure Fusion.

## Motivation

The frozen corrected CUDA/OpenCL comparison on `e81079d` completed 34
primary and two diagnostic jobs. Warm medians were 398.120214 ms native
CUDA and 787.6511635 ms OpenCL. The 0.505452455 ratio missed 0.50 by
4.294632 ms. The cold and process-RSS gates passed. The GPU-memory gate
was pending in that report. This receipt retains only typed timing,
quality-hash and provenance fields from the stock campaign.

The [older CUDA phase capture](../e9-cuda-phase-attribution/README.md)
recorded nine second full-RGB uploads in every trace arm, between 8.619
and 9.585 ms combined with an 8.908 ms median. The original input extent
is 339×512. Each packed f32 transfer carried 2,082,816 bytes, giving
18,745,344 bytes across nine calls. These were host-observed call
intervals on an earlier source revision. The current copy elimination
and its end-to-end latency need their own measured result.

## Contract and red/green proof

Registration already creates one immutable packed f32 RGB allocation
to build the luma proxy. With an exact identity map and matching canvas,
the job passes this allocation to the existing advanced accumulator.
The consuming handle checks the originating CubeCL service, extent,
dtype, contiguous shape and stride. The handle is confined to the
current frame and owner thread, and comes from a validated finite
`RgbImage`. The old allocation already occupies the original frame
slot in the registration and fusion budgets. The per-frame checked
completion, cancellation boundaries, finite-result readback, weighting,
and four-frame batch tree stay in place.

The private test-only observer runs normal job and export entry points
on synthetic 113×95 RGB16 exposures. At exact `e81079d` the identity
run reached full finite f32 and TIFF parity with the forced-host arm,
then failed because all nine frames performed a second fusion upload.
The translated run took the real warp and failed on five fusion uploads
where only the warped input needed one.

On `481d805` both tests passed, covering these five paired controls:

| Workload | Stock fusion uploads | Candidate fusion uploads | Warps | Batch sizes |
| --- | ---: | ---: | ---: | --- |
| Nine aligned identity frames | 9 | 0 | 0 | 4, 4, 1 |
| Five frames with one translated original | 5 | 1 | 1 | 4, 1 |
| Five frames with one mismatched canvas | 5 | 1 | 1 | 4, 1 |
| Five frames with explicitly configured slabs | 5 | 5 | 0 | 4, 1 |
| Five unaligned frames | 5 | 5 | 0 | 4, 1 |

Each pair asserts finite f32 bits, encoded TIFF bytes, real completion
receipts, transforms, progress, exposure-weight dispatch and completed
per-frame waits. The test-only observer was removed before linking the
candidate release CLI. Pinned rustfmt and the reviewed production-source
inventory passed. The uninstrumented native CPU library, job, advanced
fusion, fusion contract and stream plan tests passed 48/48. Effective
dependency-policy Clippy with warnings denied passed on those targets.

## Compiled code cache decision

The pinned CubeCL library offers optional persistent PTX caching. It
would add executable cache contents, a database, and configuration to
the product graph. The first-line key omits the runtime-selected
`libdevice.10.bc` identity, module-load failures propagate without
recompilation, and initial CubeCL configuration can replace the cache
root after Pancake sets it. That change stayed outside this experiment.
[Compiled-code cache source audit](../e9-lut-75c-host/README.md)

## Native targets linked

The uninstrumented `481d805` release CLI links native CUDA, CubeCL wgpu,
and Vulkan. Its SHA-256 is
`875e8800fe95806b840bf062dc211940acc12191dec3a19a4561698a99fe355f`,
and GNU Build ID is `41017f8eabeb5a25f4dc84eb3a012f1a06848dc1`.
The stock `e81079d` executable is independently retained with SHA-256
`5e43482a5e488d1faaeafd13747f20261381bbdca503f84bc200eaf112fcccb0`.
Both builds use Cargo.lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.

The separately linked `--no-run` test executable includes the synthetic
physical witness. Its SHA-256 is
`ea2cbfecc2165d1cb831d5df6c97aebf66c2cc2af76a697d6b16afc730ee946d`.
Both candidate artifacts use pinned Rust 1.100.0 and native CUDA Toolkit
12.5 build settings, two Cargo jobs on CPUs 12–13, with no Rust wrapper.
This host receipt records compilation and linkage. Physical verification
and completed E9 timing belong to separate runs.

## Exact physical parity

The synthetic probe ran both runtime arms on the RTX 3090 at PCI
`0000:01:00.0`, driver 580.178.04. Each pair compares the existing
host-upload route, enabled by the test-only control, with the resident
route in one instrumented `481d805` binary. Both Vulkan
(`spirv_4318_8708`) and CUDA (`ptx_sm86`) completed all five paired
fixtures, including the unaligned and explicit-slab controls. All 20
jobs reported physical GPU completion with no fallback.

The device and all probe processes were released at 2026-11-18
16:09:48 UTC, with exit 0. These runs verify the implementation
on the selected physical backends. E9 input, performance acceptance
and platform-specific execution have separate scopes.
