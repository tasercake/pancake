# Linux Vulkan frame queue diagnostic

The [quiet 400-event texture-cache run](../p10-viewer-cache-evidence/README.md)
improved loaded viewer P95 to 43.355 ms and measured idle P95 of 43.274 ms.
Both exceeded 33 ms. The next diagnostic evaluated one pending frame while
keeping the original Vulkan present mode.

The separate local source `39a646c` changed the Linux Vulkan
`desired_maximum_frame_latency` from wgpu's default of two to one.
The published Slint FemtoVG 1.19.0 archive exactly matched the previously
locked SHA-256 `af1892857762af5466d723f41febf4f80f92d1455de839ca9a76c955bf8793b9`.
The local package matched every archived file except the six-line change
in `wgpu.rs`. Cargo resolution moved only that renderer to the local
source path. The source policy, dependency gate, vendor contracts and
locked release build passed. [Patched build](fifo-one.build.json) and
[original build](original.build.json).

Each arm completed 25 idle and 25 loaded samples using the same exact
T50 TIFFs, real actions, pixel witnesses and color tolerance. The patched
arm ran first. Both drove an actual physical Vulkan RTX 3090 job and
accepted 50 visible pixel transitions each. Both collector invocations
exited successfully. The separate private Xvfb and D-Bus namespace
isolated input from concurrent instrumented GUI assurance. The run
recorded its NVIDIA process attribution and supplied shorter diagnostic
samples while other assurance work continued.
[Exact summary](summary.json) and
[concurrent process inventory](concurrent-gpu-processes.txt).

| Arm | Idle P95 | Loaded P95 | Idle samples over 33 ms | Loaded samples over 33 ms |
| --- | ---: | ---: | ---: | ---: |
| Original queue | 39.243 ms | 38.897 ms | 10/25 | 10/25 |
| One pending frame | 39.085 ms | 40.438 ms | 4/25 | 7/25 |

One pending frame reduced the number of slow observations. It left idle
P95 nearly unchanged and increased the loaded P95 in this diagnostic.
Both remained above 33 ms, so the experimental renderer remains on its
separate branch. The frozen viewer qualification still requires 200
observations in each condition. [Original trace](original-native-latency.json.gz),
[one-frame trace](fifo-one-native-latency.json.gz),
[original GPU event](original-physical-events.jsonl.gz), and
[one-frame GPU event](fifo-one-physical-events.jsonl.gz).

Authenticated cleanup removed all 52 owned T50 TIFFs, totaling
2,775,174,405 bytes. The input directory and temporary diagnostic lease
were removed after preserving the receipts and exact executable bytes.
[Cleanup receipt](t50-cleanup.json.gz) and
[scratch cleanup verification](scratch-cleanup.json).
