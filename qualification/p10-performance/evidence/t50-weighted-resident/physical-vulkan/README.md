# Weighted resident input on physical Vulkan

The exact default `a49ce69` candidate probe completed six paired synthetic
jobs on the RTX 3090 at PCI `0000:01:00.0`, UUID
`GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver `580.178.04`,
and fingerprint `spirv_4318_8708`. Both arms used the same instrumented
binary and physical backend. The control used the existing host-upload
route. The candidate used its normal resident policy.

All 12 jobs completed without fallback, with identical complete
f32 and RGB16 TIFF output, maps, progress and completion events within
each pair. The nine-frame identity run went from nine Focus uploads to
zero. Each warped fixture transferred only its transformed frame.
Separate unaligned, real slabbing and mixed-dimension resize pairs
exercised host staging. Energy, combine and checked frame-wait
counts matched in all pairs.

A separate native test rejected both a wrong-layout Vulkan tensor and
a correctly shaped tensor owned by the actual CPU service before creating
initial Focus state. The next four valid Vulkan frames matched the
control's per-frame weight and score bits and final f32 result, including
the first flat frame's zero weights. The private observer and explicit
readbacks were test-only.

Both probe processes and the physical lease were terminal at
2026-11-18 17:42:44 UTC, exit 0. This evidence covers synthetic physical
execution and parity. It supplies no native CUDA result or full T50
measurement.
