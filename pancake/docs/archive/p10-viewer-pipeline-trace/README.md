# Physical viewer input and presentation trace

The instrumented stock `e81079d` behavior reproduced the loaded tail in
one private Vulkan run. It captured all 200 idle and 80 loaded actions.
The independent verifier decoded 784 presented patches against the
original 19 witness PNGs. The maximum difference was one RGB level
against the allowed two. A real Hybrid job completed on the matching
physical Vulkan device. The inputs matched before and after, and GUI
shutdown returned 0. [Typed evidence](typed-summary.json).

| Condition | Observations | P95 | Maximum | Over 33 ms |
| --- | ---: | ---: | ---: | ---: |
| Idle | 200 | 23.851 ms | 186.908 ms | 1 |
| Loaded | 80 | 75.615 ms | 89.583 ms | 17 |

This was an instrumented 80-event loaded diagnostic. The frozen viewer
gate requires 200 uninstrumented loaded events and remains open.

## Where the loaded delay occurred

The preregistered conservative join uniquely matched 12 of 17 late
loaded observations. The remaining five were drags whose final motion
was followed by a release before the correct draw. All samples remain
in the primary result.

- **Eight inputs overlapped an older draw on the same synchronous UI
  thread.** In six, the previous draw was already in the driver-facing
  HAL present call at dispatch. Their old draws ended 12.071–14.753 ms
  after dispatch. In the other two, the previous renderer write was
  still waiting behind a confirmed CubeCL read on the same device lock.
  The read held for another 6.446 and 10.390 ms, then the respective
  driver-facing present calls took another 19.649 and 17.206 ms. The
  source of the driver-facing present time remains unresolved.
- **Four newly matched presents waited behind the same poll read lock.**
  The writer attempted inside the held interval, whose conservative
  overlap was 54.706–68.719 ms. The read had a nested physical HAL
  semaphore wait. These four repeats support the earlier lock
  attribution. [Previous trace](../p10-vulkan-lock-attribution/fb9c399/README.md).
- For the eight older-draw cases, Winit received the new input 0.159–2.268 ms
  after the old draw ended. The updated draw started 2.745–5.270 µs
  after the actual input handler returned. These cases leave no material
  post-handler redraw-timer stall to remove.

A secondary sensitivity check grouped exactly one final XTest drag
motion and its one ordered button release, at identical logical
coordinates, before the new draw. It assigned four of the previously
ambiguous late outcomes to an older draw and one to a new same-lock
present. The sensitivity result is 12 older draws and five new same-lock
presents. Complete raw and both analysis versions are retained. The
conservative result remains eight, four, and five.

## Configuration and remaining diagnostic boundary

The pinned Slint FemtoVG 1.19.0 Vulkan route creates the initial surface
configuration with wgpu 30.0.2 `Surface::get_default_config`, whose mode
is the first supported surface capability and whose desired frame
latency is 2. In `wgpu.rs::resize`, Slint explicitly requests
`AutoVsync`. Wgpu-core `device/surface_config.rs` resolves that policy
to supported `FifoRelaxed` first, then `Fifo`. The captured renderer
did not log the resolved surface mode. The six intervals were clocked
after wgpu-core's separate command-index lock, around the synchronous
HAL present call on the Winit thread. Vulkan HAL
`vulkan/mod.rs::Queue::present` takes the swapchain write lock, then
`vulkan/swapchain/native.rs::NativeSwapchain::present` locks semaphore
metadata and calls native `vkQueuePresentKHR`. These same-clock spans
do not split those internal boundaries or measure queue residence,
GPU execution, or compositor display time.

A future probe can retain the already-queried capability and resolved
mode, then clock the native `vkQueuePresentKHR` boundary exactly.
`Mailbox` is a supported no-tear alternative only when the actual
surface advertises it. It replaces the one queued presentation with
the latest frame. A distinct capability-gated arm would keep the
current queue, redraw cadence, pixel format, alpha, resize, viewport,
title and numerical output semantics, with the stock fallback when
unsupported. `AutoNoVsync` can select a tearing mode. The earlier
one-pending-frame experiment left the loaded P95 above target.
[Prior frame-queue experiment](../p10-viewer-fifo-diagnostic/README.md).

The capture does not establish that presentation mode caused the
six intervals. A mode experiment would be falsified if the old-frame
HAL/native-present span remains, if its delay moves into acquisition
or later visibility, or if actual rendering or compute submission
rate/residency rises, any witness transition fails, or native behavior
regresses. The mode experiment requires a distinct supported capability. No capability query or mode change was run.

## Provenance and retained data

- One run from 00:54:38–00:57:34 SGT on 19 November 2026 used the GeForce
  RTX 3090 at PCI `0000:01:00.0`, driver 580.178.04, and private 1440 ×
  1000 Xvfb. Both recorder planes share Linux `CLOCK_MONOTONIC`, the
  actual PID and start/stop anchors. They captured 30,648 graphics spans,
  22,028 Winit records and 2,698 completed draws, with zero overflows,
  incomplete slots or missing TIDs.
- Diagnostic revision `1714fedab4296dc3bd83d7d245871761744dfe11` was built
  from exact product base `e81079d`. ELF SHA-256 is
  `1c19a16feadcfc6c8c38a53de53abbe53a4edb43d52b0aa27dbb5a331e431e28`.
  The build used `taskset -c 2,3 cargo build --offline --locked --release
  -j 2 -p pancake-gui --bin pancake-gui` with the existing registry-only
  cache wrapper and the local same-version diagnostic overlays. The
  CPU recorder probe was red for the missing window plane and green
  3/3 after implementation. The CPU attribution probe covered older
  frame exclusion, handoff, same-lock assignment and ambiguity.
  [Build identity](build.json) and [overlay provenance](overlay-provenance.json).
- Stage 6 manifest SHA-256 is
  `1d9be3f1cd20a01083b11c10c4d17ab75a4f3b6bb7e2ebef682824e345ba649d`.
  All 52 frozen identities and sizes matched in the static preflight.
  The frozen collector fully rehashed the ordered 50 focus files before
  and after capture. The operator supplies only one allowlisted output
  path to the unchanged collector. It uses the original 280-action
  diagnostic selector and witness bytes. The only scenario changes
  were the expected diagnostic executable revision and digest.
  [Read-only preflight](stage6-readonly-preflight.json) and
  [preregistered design](PREREGISTRATION.md).
- The owner-only artifact directory `artifacts/p10-viewer-pipeline-e81079d/`
  retains the exact recorder planes, original native observations,
  two classification outputs, independent verifier receipt, complete
  overlay patches and authentic build and operator controls. The bounded
  replay archive contains 797 checked files. It includes the 784 actual
  small witness patches and excludes inherited process/ambient data,
  full framebuffer files and TIFF source bytes.
  [Retained hashes](sealed-capture.sha256).
