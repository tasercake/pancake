# E9 startup and device identity audit

The reviewed Pancake source is `8694b555666a1ddf4051e2839bbced4bcc625859`,
with Cargo.lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
A native CUDA loader performs two library opens during first discovery.
Retaining the successful first handle is a bounded upstream experiment.
Discovery itself accounted for only 13.075 ms of the archived 116.574 ms
runtime setup. The separate frozen native CUDA warm gap is 51.270 ms.
This audit conducted no new application builds, device calls or measurements.
[Prior frozen CUDA measurements](../integrated-e9-cuda-12f2151/README.md)

| Historical trace phase, five measured fresh processes | Min | Median | Max |
| --- | ---: | ---: | ---: |
| Native device discovery | 12.598 ms | 13.075 ms | 13.965 ms |
| Client acquisition | 86.430 ms | 87.007 ms | 92.718 ms |
| Checked variance probe | 15.589 ms | 16.006 ms | 16.111 ms |
| Complete runtime setup | 115.059 ms | 116.574 ms | 121.870 ms |

The existing Nsight capture skips some CUDA API calls. It records one
`cuInit` lasting 0.413 ms and the first `cuCtxSetCurrent` roughly 147.5 ms
later. The gap cannot distinguish primary-context retain, driver and
runtime property work, or profiler effects. The application trace
separates the setup stages but leaves their internal timing unresolved.
[Raw historical attribution](../e9-cuda-phase-attribution/README.md) · [Extracted times](summary.json)

## Native CUDA

Pancake calls `Device::cuda` through `accelerator_runtime` and then calls
`device.client()` in `qualify_device`. The checked factory preserves
`NotLinked`, asks CubeCL for the visible device count, and checks the exact
ordinal. `CudaRuntime::enumerate_devices` calls the pinned Cudarc 0.19.10
`driver::sys::is_culib_present`. That function opens the candidate library
and discards the temporary handle. Next `CudaContext::device_count` calls
`result::init`, which opens the library again via `driver::sys::culib` and
retains it in a process-local `OnceLock`, then queries the count. The CUDA
server initializer calls `cuInit` again before acquiring the named device,
reading its capability and PCI identity, retaining and setting its primary
context, and creating its streams. The source proves the redundant loader
calls. It does not show that the two calls necessarily unload and remap the
library, or establish how much time the second call takes.

One change would give the existing availability check and the existing
Cudarc accessor one private fallible loader and one success-only slot.
Keep the same ordered library candidates. An unavailable library must
remain retryable and return the same availability result. Leave the
visibility query, selected ordinal, caller-requested backend, driver
context, actual PCI/fingerprint, job-budget check and real variance
readback in place. The successful handle would be shared only within
its existing process lifetime. The 13.075 ms discovery median bounds
possible savings there before accounting for work that still must run.
This requires a reviewed upstream or pinned dependency change.

The second `device.client()` call from `row_variance_u16` reaches the
same registered `Client::load` service. Reusing the local handle directly
would remove a registry and reference-count operation, without avoiding
another context creation. Directly constructing `Device::Cuda` to bypass
`Device::cuda` loses checked unlinked and invisible-ordinal behavior and
moves these errors to panicking client initialization. The variance check
really submits and reads back `[16.0]` from `[1,9]`; removing it changes the
qualification contract. CUDA server acquisition also checks for NCCL even
on single-device jobs, but the host loader cache lists no NCCL, and its
cost is unmeasured. There is no supported app configuration to omit it.

## Portable wgpu and next checks

The leading discrete Vulkan shortcut in Pancake is already present.
CubeCL 0.11.4 itself still creates a temporary `probe(api)` instance
and enumerates adapters before `request_adapter` creates or reuses the
retained instance and enumerates the same pinned adapter list. The
existing narrow prototype removes that duplicate for explicitly pinned
physical ordinals. It retains the configured instance, actual adapter,
feature negotiation, registered selector and real probe rather than
caching an ordinal across APIs or inventing an identity from a name.
That prior three-pair diagnostic observed 631.613 → 517.950 ms with the
same output and invalid-selection evidence. The RSS change was 1,436 KiB.
The prototype remains experimental and has no matching frozen pass.
[Existing Vulkan experiment](../e9-vulkan-overhead/README.md)

For the native loader proposal, a test should first show that a successful
presence check and accessor invoke the driver loader twice and then once,
while unavailable and retry cases preserve errors. On a linked release,
collect isolated loader traces separately from time measurements. Repeat
fresh-process startup with the original checked variance and real GPU
cases. Cover an absent visible ordinal, a missing linked runtime, a CPU
forced run, no hidden accelerator fallback and real selected-device
identity. Match actual RTX PCI/fingerprint, decoded and encoded output,
and cleanup before comparing end-to-end time. None of these prospective
checks were executed in this read-only audit.
