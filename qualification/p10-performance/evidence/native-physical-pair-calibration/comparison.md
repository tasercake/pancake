# Sealed 24 MP comparison rendering miss

A second untimed run used the same sealed Linux GUI, exact two T50 TIFFs,
physical RTX 3090, private true-color desktop, and real exported Hybrid
result. The native XTest pointer was parked outside the witness region.
Both source views, actual retained-result reselection, 1:1 zoom, and pan
matched all 18,432 test pixels each within one RGB level.

Comparison did not match the independent image projection. At published
split `0.5`, the right-band region differed from the expected retained-stack
region by up to 20 RGB levels. After a native drag to `0.73961216`, the
region differed from the intended left-source region by up to 16 levels.
The divider and accessible split did move.

The comparison clip in `ViewCanvas` binds its width to the current split
and leaves `x` unset. The locked Slint compiler’s default geometry centers
that rectangle within its parent. On the actual 722-pixel viewport this
places the clip at local `180.5` and `94` pixels, depending on split.
The renderer snaps the resulting source-image origins to shifts of 181
and 94 physical pixels. The actual comparison regions match the independently
projected first source at those wrong positions, within one RGB level on
2,208 of 2,208 and 3,456 of 3,456 overlapping pixels respectively. The
geometric prediction and captured pixel values are retained separately.

The comparison clip needs to share the divider’s left origin. A new release
artifact must verify the actual corrected pixels before establishing the
full-workload independent witnesses and timing 200 idle and 200 loaded events.
This preflight produced zero latency samples. Authenticated cleanup removed
both generated source TIFFs and the temporary result.
