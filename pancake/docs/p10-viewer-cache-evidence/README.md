# Retained viewer textures on the shared GPU

The historical [400-event replay](../../../qualification/p10-performance/evidence/native-viewer-measured/README.md)
reproduced the viewer latency failure on production source `74dfaa3`.
Pinned FemtoVG discards texture wrappers when the corresponding Slint
items no longer refer to them. Returning to a previously viewed image
therefore reuploaded its pixels even when Pancake retained the decoded
display raster.

Candidate `89c6a22` imports the identical RGBA8 display pixels as an owned
texture on the existing Slint and worker device and queue. The workspace
retains up to four cached previews at 16 MiB each. The current and comparison
bindings may each retain a preview outside that cache, and a miss can stage
one new raster. Replacing or clearing the workspace releases its preview
bindings. Full-precision stack data and export stay in the shared engine.
Software presentation still uses the bounded CPU cache.

| Condition and action | Baseline P95 (ms) | Texture cache P95 (ms) |
| --- | ---: | ---: |
| Idle, all 200 actions | 42.310 | 43.274 |
| Idle source | 52.285 | 44.606 |
| Loaded, all 200 actions | 54.054 | 43.355 |
| Loaded source | 55.280 | 44.340 |
| Loaded result | 64.099 | 44.334 |

Both conditions still exceed 33 ms. The aggregate medians are 32.493 ms
idle and 32.493 ms loaded. The first cold source decode took 672.899 ms.
The collector accepted all 400 observations. The loaded state at input
was Decode 137 times and Fuse 63 times. The unchanged reducer returned
failure for both P95 gates. The trace retains the other action statistics
and full observed presentation timestamps.
[Summary](summary.json), [unmodified reducer output](frozen-reduction.json),
and [native trace](native-latency.json.gz).

Each native control, motion, condition, expected pixel file digest, screen
region, color tolerance, and ordered TIFF record matched the original
campaign. The seven independent calibration states stayed within one RGB
level. The real retained 6000 × 4000 RGB16 TIFF was byte-identical to the
historical replay, SHA-256 `e77f22ddbc7b4d809bf82ed3deae7f99738b6d194dfee2e5731d43065c33006b`.
The quiet run used the physical RTX 3090, driver 580.178.04, the Ryzen
7950X3D, and a case-owned 1440 × 1000 Xvfb screen at 2026-11-18
12:42:50–12:47:30 SGT. [Pixels](pixel-calibration.json.gz) and
[physical worker events](physical-events.jsonl).
