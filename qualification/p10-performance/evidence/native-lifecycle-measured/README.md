# Physical Linux lifecycle latency

The sealed Linux release met the frozen 250 ms deadline on all 120 native
lifecycle requests. Each workload supplied 30 pauses and 30 cancellations.
The figures are the conservative observed endpoints in the private desktop.

| Workload | Operation | Samples | P50 (ms) | P95 (ms) | Max (ms) |
| --- | --- | ---: | ---: | ---: | ---: |
| Original 64-path fixture | Pause | 30 | 73.777 | 148.261 | 148.341 |
| Original 64-path fixture | Cancel | 30 | 146.849 | 148.792 | 148.799 |
| Real D150 | Pause | 30 | 147.484 | 149.559 | 155.640 |
| Real D150 | Cancel | 30 | 147.563 | 221.666 | 221.802 |
| Combined | Pause | 60 | 147.161 | 148.971 | 155.640 |
| Combined | Cancel | 60 | 147.037 | 221.086 | 221.802 |

The quiet, serial window was 2026-11-18 11:49:29–11:51:21 SGT. The host was
an AMD Ryzen 9 7950X3D with an NVIDIA RTX 3090, driver 580.178.04,
PCI `0000:01:00.0`, UUID `GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`.
Before launch the device reported 0% utilization; unrelated ordinary desktop
clients remained connected. Both isolated 1440 × 1000 × 24 Xvfb sessions
used automatic Slint presentation. Each actual visible GUI process had
83,533,824 bytes resident on the target GPU before stacking. The last job in
each scenario completed on the same PCI device through `wgpu-vulkan`, with
no fallback. Both collectors exited zero and reverified every ordered input.

The source was sealed revision `5216ba4227e14033483a21bb5eca1c6c168ef840`,
lock `639c5c88b9286e5583174f05168412db19f39dc3f651b6682b685d1db801e0dd`,
GUI SHA-256 `57053ae2b7e15e4e5f21559149b0e8dc17d43dec7f218169c799ce194e5b3598`,
previously built with locked Cargo release settings and copied into the exact
owned source checkout. The archive retains the build log and receipt, harness
revision and file hashes, hardware and frozen target hashes, launch commands,
procedure, scripts, scenarios, ordered inputs, and raw event timestamps.

Each sampled event started during an actual numerical stage. The 64-path
fixture used Laplacian and Breathing on its true 1024 × 1536 fixture. D150 used
Hybrid and Breathing on all 150 exact native JPEGs. The observer verified
worker-confirmed Paused on all 60 pauses. It saw one actual Cancelling state,
on D150 Generation(13), after 73.015 ms, then reusable Idle after 146.844 ms.
The other 59 cancellation requests first exposed actual acknowledgement and
settled Idle together through the same worker generation’s durable marker.
The combined settled time statistics in the reducer match the acknowledgement
statistics at the reported percentiles. Raw times preserve the distinct
settled observation in Generation(13).

The collector removed its generated fixture, temporary profile and caches
after both sessions. D150 remained available for other evidence at that point.
The frozen reducer reports both lifecycle operations and workload coverage
passing. Viewer coverage is still pending at zero of 200 idle and zero of 200
loaded observations, so the combined latency reduction exits 1.
