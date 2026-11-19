# Hybrid per-level fusion review

Keep the seven-tensor kernel at `80da4019b79617b25a86d644c9d7ce8c1c5fd941`
experimental. It can select a shader layout that a supported imported wgpu
device cannot create. Recompute the coefficient in each existing consumer to
remove the temporary band with the previous binding limit. The review used
clean base `b820da05d3982a7912cb7c7c9d8aa926ece14a9c`, tracked tests from
`180f131e85068c024b5b0f8763f7646f5c01720f`, and lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.

## Imported device limit

`Level::accumulate` selects `add_hybrid` at seven reported hardware bindings.
The kernel supplies seven data tensors. The pinned CubeCL WGSL backend appends
a read-only storage binding for nonempty kernel metadata. This kernel needs
metadata for its dimensions and frame flag, giving indices 0 through 7.
CubeCL's wgpu hardware count derives solely from the device's per-stage
storage limit minus one.

A caller can request a physical wgpu device with eight storage buffers per
stage and `max_bindings_per_bind_group = 7`, then use CubeCL's safe existing
device registration. CubeCL reports seven and accepts the imported setup.
wgpu rejects the fused layout because binding index 7 is greater than or equal
to the independent group limit. The original Hybrid level kernels use at most
five data buffers and one metadata buffer, so their indices fit. A test that
reduces only the storage cap exercises the old fallback but misses this
independent boundary.

The normal CLI and GUI path creates the wgpu device with the adapter's normal
limits. The observed defect is in the supported library/imported-device
boundary. This diagnosis comes from source. No device was opened in the
review. The pinned sources are CubeCL 0.11.4 `cubecl-wgpu/src/runtime.rs`
(`try_init_device_with_api`, `try_create_server`),
`cubecl-wgpu/src/backend/wgsl.rs` (`bindings`),
`cubecl-wgpu/src/backend/base.rs` (`create_pipeline`), and wgpu-core 30.0.2
`src/device/resource.rs` (`create_bind_group_layout_impl`). Native direct
CUDA and HIP report 1,024 bindings, Metal reports 30 and CPU reports `u32::MAX`.
Those caps do not prove the behavior of portable wgpu imports.

## Arithmetic and state

Each thread owns one score and all three packed RGB samples for its pixel.
The first frame initializes both sums, the winner score and all winner
channels without reading uninitialized state. Subsequent frames use the
original strict `>` comparison and the same per-pixel accumulation order.
A tie leaves the earlier winner. Output tensors belong to the same client
and remain distinct from the input planes. At the coarsest level the lower
binding aliases the fine input read-only, and zero lower width prevents any
expansion. The earlier weighted kernel already had this alias pattern.
The per-frame drain still covers all four output handles.

The checked `ImageShape` bounds `3 × width × height` and the kernel exits
before the first read on excess work items. The existing ceil-half
pyramid and `expanded` helper keep the final odd row or column and clamp
its neighbors. The `1 × 1`, `33 × 1`, `1 × 33` and odd multi-level test
cases cover these shapes. The review rehashed both native CPU differential
outputs. Their 235,962 f32 channels were byte-identical across 18 cases
(six shapes, each with random, equal-contrast and zero-contrast frames).
Each output was 943,848 bytes with SHA-256
`19d204268113dcb8178898d06aa523f6a9fae38233b180539a74bec252e38422`.
The red test reported 50 launches over two four-reduction frames before
fusing the level passes.

The old code stores each subtracted coefficient as f32 between kernels.
Eliminating that store changes the optimizer's scope. CubeCL's pinned LLVM
lowering explicitly permits contraction on GPU `fadd`, `fsub` and `fmul`
and keeps native CPU strict. Source equivalence and CPU output equality
are insufficient for GPU bit parity. The qualified portable Vulkan profile
needs a same-device raw f32 A/B with actual fused-path proof and retained
odd, signed-band, positive tie and zero-focus coverage. If the revised
code independently recomputes the coefficient in weighted and winner
kernels, this comparison also checks both consumers after lowering.
Retained native CUDA needs its own focused A/B because the compiler differs.
Continue existing Metal build and hardware evidence accounting.

A revised kernel that keeps five data bindings can avoid raising any
layout limit. For the two-pass revision, a dispatch witness should record
actual kernel names and counts. A maximum alone cannot distinguish a
required fallback from a faster path. Preserve prior numerics, cancellation,
slab and complete-stack regression evidence on the final integrated source.
