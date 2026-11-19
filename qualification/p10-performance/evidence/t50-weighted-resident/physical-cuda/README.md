# Weighted resident input on physical CUDA

The exact `a49ce69` native test executable completed the same six
paired synthetic fixtures as the Vulkan probe, using native CUDA on the
RTX 3090 at PCI `0000:01:00.0`, UUID
`GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver `580.178.04`
and `ptx_sm86`. The pinned CUDA 12.5 toolkit paths accompany
the run commands. Each pair used one exact test executable before and
after enabling the normal resident policy.

All 12 jobs completed without fallback. Per pair, f32 and RGB16 TIFF
bytes, registration and completion events, progress, and energy,
combine and wait counts matched. The nine-frame aligned identity pair
removed all nine second full-frame Focus uploads. Warp, alignment-off,
slab and resize controls followed their checked routes.

The separate layout and foreign-service test rejected both a wrong-layout
CUDA tensor and a correctly shaped tensor from the actual native CPU
service before creating first-frame Focus state. Four subsequent valid
CUDA submissions matched the control's per-frame weight and score bits
and final f32 result. The first flat frame retained exactly zero weights.

Both probe processes and the exclusive device lease returned at
2026-11-18 17:50:57 UTC, exit 0. This is synthetic functional evidence.
A completed T50 benchmark has separate workload, timing, and resource
measurement requirements.

[Runtime and per-fixture audit](audit.json)
[Raw physical records](raw-physical-records.tar.gz)
[Archive checksum](raw-physical-records.sha256)
[Native probe linkage](../native-cuda-build/README.md)
[CPU proof and exact default release](../README.md)
