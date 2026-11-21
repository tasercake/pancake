# Dataset portfolio

The [eight-source catalogue](dataset-catalogue.csv) and [Bokehforge data guide](../../bokehforge/docs/data.md) describe the active portfolio. [Source evidence](sources.md) records provider claims and file checks.

| Track | Sources | Benchmark use |
|---|---|---|
| Photographic RAW | D02 Sood 999-frame sweep | Long-stack alignment, memory, motion and precision. Treat the full sweep and derived subsets as one scene family. |
| Optical microscopy | D04 Amber spider | Thin structures, deep focus and difficult materials. Group repeated views of the same specimen. |
| Photographic pairs and sweeps | D10 EDMF, D11 HRMFF, D19 Mobile Depth | Seams, scale and handheld geometry. EDMF has 950 matched pairs and 50 unmatched B images. |
| Synthetic focal stacks | D09 MFI-WHU, D20 FoD500, D29 InfinigenDefocus | Synthetic blur controls and rendered reference tests. Track range, aperture, and shared assets when comparing results. |

InfinigenDefocus supplies a rendered all-in-focus RGB image per view plus native depth and camera data. Use each aperture as a separate nine-frame stack. Group related views and apertures by scene seed. The renders support controlled indoor tests. Production macro validation still needs independently captured macro groups with calibrated targets and known focus coverage.

See the [acquisition plan](acquisition-plan.md) and [split protocol](split-protocol.md).
