# Sealed physical 24 MP pixel calibration

The exact `5216ba4` Linux GUI (`57053ae2…`) selected the original 6000 × 4000
RGB16 T50 `focus_000.tif` and `focus_006.tif`, then completed a real Hybrid
stack on the physical RTX 3090 through `wgpu-vulkan`. The GUI had 117,088,256
bytes of GPU residence at its first visible launch, before any worker was
started. It exported the actual uncropped retained 6000 × 4000 RGB16 result.
Independent TIFF decoding confirmed the geometry and receipt SHA-256
`791efd8d1ef6c6e8367652264a4ea2b596cee31796c8529e346deb702a485844`.

Independent projection used the native TIFF RGB pixels, the measured viewport
geometry, the bounded app preview sampling, and the pinned FemtoVG-wgpu
bilinear sampling at physical pixel centers. The projected `focus_006.tif`
and retained-result regions matched all 18,432 actual screen pixels within
one level per RGB channel. The first `focus_000.tif` region had 257 mismatches
clustered in a 16 × 23 cursor-shaped area. The preflight’s every-pixel check
therefore failed, and the archive preserves that observed region alongside
its independent projection. Subsequent calibration must park the pointer
outside the witness area before acquisition.

This acquisition supplies source/result calibration evidence and zero timed
viewer observations. The full scenario needs fixed independent witnesses for
all five operations in both worker conditions. Authenticated cleanup removed the
two generated TIFFs and the temporary exported result. The raw receipt retains
paths, hashes, command source, hardware, and actual screen RGB.
