# Corrected Linux native viewer latency

The corrected Linux GUI captured all 400 frozen viewer observations on a
physical wgpu/Vulkan RTX 3090. Both conditions exceeded the 33 ms P95 limit.

| Condition | Samples | P50 (ms) | P95 (ms) | Max (ms) | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| Idle | 200 | 35.649 | 42.310 | 657.914 | Fail |
| Real numerical work | 200 | 34.670 | 54.054 | 71.558 | Fail |

Each condition includes 40 events of each type: source selection, result
selection, pan, zoom, and comparison drag. The observer checked that the
actual worker was at the expected state around each native display event.
Idle observations followed a real completed 50-frame stack. Each loaded
round started an actual stack, checked numerical activity, and cancelled
after the five actions. Of the loaded inputs, 134 began in Decode and 66 in
Fuse. The highest P95 event type was source selection at idle (52.285 ms)
and retained-result selection under load (64.099 ms). The exact per-action
statistics retain the outlier and observation gaps.

The locked release source was `74dfaa359ea9e285baac16a16f485977f6c10b6c`,
GUI SHA-256 `983735af5db3b00885678a1b6cf1e2160949463b3eb54ce98e307c1e4e1aabbb`,
and lock SHA-256 `639c5c88b9286e5583174f05168412db19f39dc3f651b6682b685d1db801e0dd`.
The uncontended window was 2026-11-18 12:15:39–12:20:18 SGT on the Ryzen 9
7950X3D, with the RTX 3090 at PCI `0000:01:00.0` and driver 580.178.04.
The GUI had 117,088,256 bytes of attributable GPU residency at the first
visible launch, before any numerical work. An actual full T50 job completed
on that device with `wgpu-vulkan`. The virtual true-color screen was
1440 × 1000 × 24, using the normal application renderer selection. The native
XTest input and readout of the actual mapped framebuffer yield a conservative
upper bound for visible Xvfb presentation. Maximum observation gaps were
2.147 ms idle and 1.243 ms loaded.

The independent calibration decoded all selected source/result TIFFs and
projected their known RGB values through the exact native viewport geometry
and renderer sampling. Both original 24 MP source states and the actual
retained-result, zoom, and pan states matched all 18,432 checked pixels per
region within one RGB level. Both sides of the repaired comparison matched
3,456 of 3,456 pixels within one level. The real independently decoded GUI
result was genuine 6000 × 4000 RGB16, TIFF SHA-256
`e77f22ddbc7b4d809bf82ed3deae7f99738b6d194dfee2e5731d43065c33006b`.

The first scenario misspelled its pixel tolerance field and timed out on
setup before the first sampled dispatch. Its preserved screenshot matches
all three setup witnesses within one RGB level. The corrected reader tests
the field explicitly and the actual capture used the calibrated two-level
tolerance. Both attempts and all 400 valid observations are archived.
The collector rehashed every ordered input. Authenticated cleanup removed
all 52 generated TIFFs (2,775,174,405 bytes), and the temporary calibration
export and private caches were removed.
