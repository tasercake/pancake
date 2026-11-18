# Pancake native output witness preflight

The debug GUI at `8f1da27b65c2138768d2b118ee7701501368c2ca` opened two exact
6000×4000 native RGB16 T50 frames on a private software Xvfb and AT-SPI
desktop. The build lock SHA-256 was
`4e705dc47a6fcc2ff46a9c0ace4307605463a9d57d3e1e4a916caf8b72c8da18`.
Actual XTest selection changed the reported native source index from 1 to 2.
The observer first matched the independently expected before pixels, then
required the actual visible screen to match a different source projection.
The viewport retained the same native fit scale and center.

The initial floating-point resampler predicted the wrong screen colors even
though the native source index changed. This is why a state-only observer
cannot establish presentation. With rounded destination geometry and the
pinned Slint software renderer’s fixed-point nearest sampler, all six
separated witness locations matched their independently TIFF-derived
selection colors exactly. The host used `SLINT_BACKEND=winit-software`.
Candidate release latency and the physical wgpu witness calibration remain
open.

The [pair cleanup receipt](../scale-functional/t50-pair-cleanup.json)
records authenticated deletion of each original temporary TIFF.
