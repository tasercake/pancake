# Exact packaged viewer replay at 8694b55

The `8694b55` Debian GUI passed idle viewing and missed the loaded viewer
gate. The collector exited 0 after all 400 native input-to-visible events.
Independent decoding matched all 1,120 displayed RGB patches to the
19 original fixed calibration PNGs within one level per channel. The
frozen tolerance is two levels. All 50 ordered source files verified
before and after the capture. There was one run.
[Verified scope](summary.json) and [original frozen reducer](frozen-reduction.json).

| Condition | Count | P50 | P95 | Over 33 ms | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Idle | 200 | 22.662 ms | 23.811 ms | 1 | Pass |
| Loaded | 200 | 22.784 ms | 59.598 ms | 33 | Fail |

The raw capture retains the idle maximum of 201.935 ms and the loaded
maximum of 190.435 ms. The reducer also records lifecycle as pending in
this viewer-only capture. The loaded miss independently exceeds its
33 ms gate by 26.598 ms.

## Scope and comparison

The exact source is `8694b555666a1ddf4051e2839bbced4bcc625859` and the
extracted GUI SHA-256 is
`62e77cb6305514c2a1bbd1bcf3fc4e1518cbae572d48483e4e5e2f938daacf06`.
The package SHA-256 is
`7dbc969e1813bc62aa4df206bb77982e3f7705298f753e12b7a493ae89ce3784`.
The fresh authenticated T50 materialization uses SHA-256
`317f4b0c9be5837cd7082227bf700ee232e7e995471ee44be8488cbf3511603c`.
The original 882 scenario steps and all scored controls stayed fixed.
Only the scenario's build commit and executable digest were rebound.
[Build identity](build.json) and [scope verification](preflight.json).

On 2026-11-18 from 20:30:50 through 20:35:07 SGT the exact GUI ran in
an isolated 1440 × 1000 Xvfb session on AMD Ryzen 9 7950X3D and NVIDIA RTX
3090. Physical Vulkan completion matched PCI `0000:01:00.0` under driver
580.178.04. The collector was uninstrumented and the launch census had
no compiler, Miri, profiler or other controlled product process. The
machine's GPU snapshots were 41 °C/P8 before and 49 °C/P5 after, with
reported utilization 0% at both. The historical `287828a` viewer capture
had 69 °C/P2 and 68 °C/P2 at 2%. Both used the same CPU affinity,
frequency policy, kernel and observer clock. These snapshots give no
in-run frequency history. [Exact comparison](tail-analysis.json).

The prior `287828a` package missed 12 loaded events over 33 ms. The new
package missed 33. The 33 late events were 14 zooms, nine comparison
drags, six pans and four retained result selections. All source switches
met 33 ms. Twenty-nine of the misses straddled native worker phase
changes, with 16 during Fuse to Decode and 13 during Decode to Fuse.
Sixteen of 19 Fuse-to-Decode observations were late, with a median
64.977 ms. Three of 127 Fuse-to-Fuse observations were late and that
group's P95 was 29.343 ms. The longest, step 458, was a zoom from Fuse
at two completed frames to Decode at three. The longest polling gap
among late events was 1.165 ms. Worker labels bracket the observed
transition and cannot distinguish renderer input scheduling, queue
blocking, and scanout delay.
[Per-case transition records](tail-analysis.json).

A separate earlier direct-target trace on this package proved host
Vulkan instrumentation. It used 12 source frames and did not reproduce
the full retained-result tail. A bounded follow-up diagnostic can use
all 50 exact focus inputs, the original full retained Hybrid result,
and all 19 witnesses. Keep the full 200-event idle prelude, then the
first 16 contiguous original loaded rounds through step 593. These 80
loaded samples include the observed step 458 zoom and other late
events. Keep the original load admission, input handling, and
correct-pixel endpoint, with no phase-forcing before an event. The
same private exact-GUI-first Nsight route can align the real GUI PID's
Vulkan acquire, submit and present calls with external host-clock
brackets. Compare dispatch-to-acquire, host API duration and
present-to-correct-frame intervals on a reproduced tail. That
instrumented run would carry no frozen qualification credit.

[Raw native replay](raw-native-viewer.tar.gz) and [archive checksums](SHA256SUMS).
