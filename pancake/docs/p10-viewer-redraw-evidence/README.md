# Physical X11 viewer redraw replay

The application-side redraw proposal did not improve the frozen viewer gate.
Both release binaries captured all 400 valid visible transitions, with the
same 200 idle and 200 loaded actions and exact T50 bytes. All expected witness
records matched across arms. The allowed channel error was two RGB levels.
Every observed patch matched within one. Neither arm passed the 33 ms P95
limit. The candidate remains on `codex/pancake-viewer-latency-audit`.
[Exact results](summary.json) and [archived native captures](raw-native-pair.tar.gz).

| Condition | Sealed P50 | Redraw P50 | Sealed P95 | Redraw P95 | Target |
| --- | ---: | ---: | ---: | ---: | ---: |
| Idle | 29.348 ms | 29.281 ms | 41.121 ms | 41.492 ms | ≤33 ms |
| Loaded | 30.458 ms | 30.481 ms | 41.363 ms | 41.335 ms | ≤33 ms |

Idle result P95 fell from 42.522 to 35.927 ms and divider P95 from 31.238 to
27.897 ms. Idle source, pan and zoom P95 increased from 34.793, 36.958 and
34.677 ms to 41.492, 42.352 and 39.082 ms. The idle cold-source observation
remains in each arm at 653.029 and 638.529 ms. The mixed action results do
not support adopting this redraw bypass as a fix for the visible endpoint.
[Sealed reduction](a.frozen-reduction.json) and [redraw reduction](b.frozen-reduction.json).

The test-first change at `a29a3bb` failed at the actual Slint callback binding
with `NoSuchCallback`. All 15 GUI tests, formatting and the reviewed source
policy passed at `1a0ec8c`. The candidate requests a normal Winit redraw after
a committed viewer change on a shared wgpu Xlib/Xcb window. It keeps FIFO
presentation. The paired timings do not isolate how much time passes in input
scheduling, rendering, the Vulkan swapchain or XWD observation. Maximum
observed polling gaps were 1.333 ms and 1.336 ms.
[Red test](offscreen-red.log.gz), [green suite](offscreen-green.log.gz),
[source policy](source-policy.log) and [release build](release.log.gz).

## Provenance

- Redraw arm `1a0ec8ccf2eea215cfeb57f47ffd85d3e6779464` used the locked release
  workspace build, GUI SHA-256
  `f0e2f354fc561f6479b2e0930c691bd2fd6dd74013618c8918f8dc07c192cdad`.
  The exact candidate file is retained locally at
  `artifacts/p10-viewer-redraw/binaries/pancake-gui-1a0ec8c`.
  Both arms used the same Cargo lock, descriptor and collector bytes.
  [Build receipt](b.build.json).
- Both ran on AMD Ryzen 9 7950X3D and NVIDIA RTX 3090, driver 580.178.04,
  in separate private 1440 × 1000 Xvfb sessions. Each had 117,088,256 bytes of
  attributable GPU memory at visible launch and one matched `wgpu-vulkan`
  completion. The sealed arm ran 14:40:58–14:45:35 SGT and the redraw arm
  14:45:35–14:50:10 SGT on 2026-11-18. The GPU started at 43 °C and 51 °C
  respectively. Loaded actions sampled Decode/Fuse in counts 128/72 and 127/73.
  [Hardware and worker receipts](summary.json).
- Both consumed the same owner-staged 50 focus TIFFs from exact T50
  materialization `d7159a403d5d2f28cf79e990a5a424456e893246b39dc2a58a4b0e0b72c96517`.
  The collector verified input bytes before and after both arms. The
  materializer retained ownership for the subsequent T50 measurements and
  cleanup. The original 19 calibrated witness PNGs and all ordered steps
  matched the earlier physical campaign. Only each executable's commit and
  hash were rebound in its scenario. [Calibration identities](calibration-preflight.json).

The first attempt used an output path without the `/artifacts/` segment
required by the existing private desktop guard. Both arms returned before
Pancake launched and yielded zero samples. The fresh scoped output hierarchy
satisfied the unchanged guard. [Failed setup receipts](false-start.tar.gz).
