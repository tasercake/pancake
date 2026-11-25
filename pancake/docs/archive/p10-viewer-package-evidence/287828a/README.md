# Packaged Linux viewer replay

The exact `287828a` Debian package passed the idle viewer gate and missed
the loaded gate by **0.818 ms**. All 400 ordered events were valid, with
200 per condition and 40 of each action per condition. The independently
decoded 1,120 actual presented RGB patches matched the original witnesses
within one level per channel against the frozen two-level tolerance.
[Verified summary](summary.json) and [unchanged reducer output](frozen-reduction.json).

| Condition | Median | P95 | Events over 33 ms | Frozen result |
| --- | ---: | ---: | ---: | --- |
| Idle | 19.635 ms | 21.713 ms | 1 / 200 | Pass |
| Loaded | 20.733 ms | 33.818 ms | 12 / 200 | Fail |

The idle cold source event took 516.469 ms and remains in the reduction.
The collector exited 0 and the frozen reducer exited 1. Pause and cancel
remain independently pending in this viewer-only capture. This was a
single measured attempt.

## Exact provenance

- Source `287828aac07ffad2252785b73a52a59fc07b3409`. Debian archive SHA-256
  `34a63edea55c411a6082924c2fbe2c50094eea98753b1f75378459da78e4db99`.
  Actual installed-member executable SHA-256
  `f27ead9db3b79429a1f924dd098ae2c253e7fd5bda28396238e21f9f72723961`.
  The build used lock SHA-256
  `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
  The local replay extracted and executed the package member without
  rebuilding it. [Package-bound build receipt](build.json).
- Linux Ubuntu 24.04.5, AMD Ryzen 9 7950X3D, RTX 3090 on PCI
  `0000:01:00.0`, driver 580.178.04. A private Xvfb desktop at
  1440 × 1000 supplied the first visibly correct XWD framebuffer endpoint.
  The app completed one verified physical `wgpu-vulkan` T50 job and
  exited cleanly. Raw capture ran 18:59:30–19:03:59 SGT on 2026-11-18.
  [Physical completion and display witness](raw-native-viewer.tar.gz).
- The 50 ordered T50 input identities, 19 independent PNGs, original
  framebuffer regions, color tolerance and ordered action objects were
  identical to the earlier scoped run. The collector verified all 50
  TIFF bytes before and after the capture. The shared authenticated
  materialization SHA-256 was
  `62200bca0567ae06e2b6d3eba406aea259e601904c756b64ea7b245ede2625a4`.
  [Original calibrated witness identities](../../p10-viewer-redraw-evidence/calibration-preflight.json).

## Loaded tail

The earlier `6489af6` candidate passed at 23.827 ms loaded P95.
Compared against that historical run, the package's 12 loaded deadline
misses were four pans and eight zooms. The earlier run missed five,
all zooms. Every loaded source, result and comparison event was
within 33 ms in both runs. The new package moved
more observations into Fuse, from 58 to 119. Within Fuse the P95 also
rose from 22.959 to 35.866 ms. Stage mix alone does not explain the tail.

All eight late package zooms straddled the transition from Fuse at one
completed frame to Decode at two completed frames. The earlier five
late zooms straddled Decode to Fuse at one completed frame. Eleven
of 12 package misses began in Fuse. Late loaded samples were polled
at maximum observation gaps of 1.101–1.159 ms. Both releases use the
same Slint viewport and unchanged capture/reducer code. The recorded
timing and worker stages establish coincidence, with insufficient detail to
locate the delay in viewport callbacks, frame preparation or the GPU
queue. [Exact action, stage and event distributions](attribution.json).

The two runs had identical CPU affinity, frequency policy, kernel,
clock source, validator versions, GPU identity and configured clocks.
The device reported 69 °C at start and 68 °C at end for both, with
2% utilization and the same nine pre-existing GPU processes in all
four snapshots. Both GUIs reported 117,088,256 GPU-resident bytes at
visible launch. These pre/post snapshots leave transient background
contention unmeasured. [Environment comparison](environment-comparison.json).

## Queue path review

`configure_backend` gives Slint the wgpu device and queue from the
prepared runtime. `choose_runtime` reuses it for the matching physical
worker. The pinned Slint FemtoVG renderer acquires the next swapchain
texture, submits its draw commands, then presents through that queue.
`AdvancedAccumulator::submit_frame` enqueues the frame's ordered
pyramid work and synchronizes its actual output buffers. This is a
real per-frame completion boundary.

CubeCL wgpu 0.11.4 already supports a configurable batch cap via
`RuntimeOptions::with_tasks_max`. Its default cap is 32 tasks per
submitted command buffer. The scheduler can submit several commands
before reaching the existing sync boundary. A lower cap alone cannot
limit queued work ahead of presentation. An added flush can submit
earlier and provides no scheduling guarantee. Added waits or a new
queue would need evidence that they address the observed delay and
preserve throughput and lifecycle behavior.

An instrumented diagnostic on the private physical lane can timestamp
actual Winit pointer/wheel delivery and viewport property changes, and
correlate those with Vulkan frame acquisition, graphics and compute
submissions, and queue presentation. The original XWD witness should
remain the visible endpoint. The pinned FemtoVG rendering notifier
submits an additional command buffer before its callback. A queue/API
trace avoids relying on that notifier and should remain separate from
frozen measurements. If it shows a long compute submission ahead of
a ready graphics frame, the existing batch cap is a small testable
control for a subsequent diagnostic. The package replay established
no product change.
