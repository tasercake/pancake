# Physical viewer input, redraw, and present attribution

This is a read-only source and evidence audit at `cd248710463540ea73c004b551e9f6c57e1977ee`.
The diagnostic would reuse the stock application's scheduling policy and the
frozen native observer. The audit compiled nothing and ran no desktop or device.

## Existing measurements

The stock `fb9c399` same-device trace confirmed a renderer write wait behind the
CubeCL poller's read hold for nine of its 16 late loaded observations. For those
nine, Rust present began 1.388–2.235 ms after dispatch and driver present took
16.815–19.491 ms after the write-acquisition stall. The other seven late
observations had two plausible presents each. Six were retained-result clicks
and one was comparison. All seven were admitted in Fuse. The post-dispatch new
submit candidates in the independent replay began 13.495–26.049 ms after
dispatch. One submit candidate is insufficient to prove which completed draw
first displayed the independently verified pixels.
[Same-lock evidence](../p10-vulkan-lock-attribution/fb9c399/README.md).

Reducing the shared poll wait to 1 ms moved loaded P95 from 52.322 to 66.942 ms
on the matched `c1e2cac` pair. The separate parked nonblocking poll moved it
from 42.208 to 68.007 ms on the matched `fb9c399` pair. Both interventions
increased late zoom and comparison samples. The captures do not have matched
internal host spans, so these outcomes cannot assign the new delays to polling,
queue work, or presentation. Earlier X11 redraw and one-pending-frame
experiments also missed the target.
[Bounded poll](../p10-shared-poll-evidence/c1e2cac/README.md),
[nonblocking poll](../p10-shared-poll-evidence/fb9c399/README.md),
[redraw](../p10-viewer-redraw-evidence/README.md), and
[frame queue](../p10-viewer-fifo-diagnostic/README.md).

## Controlled capture scope

Bind the current sealed stock product commit, executable, Cargo lock,
descriptor, and target hashes before building instrumented overlays. Verify
that the application source and runtime behavior match that seal. Retain
independent overlay hashes, linked package versions and diagnostic build
identity. Keep stock queue policy, callbacks, action order and release
behavior. Only collect one bounded run after both scheduled CPU pairs have
finished and the parent grants the exclusive RTX, quiet host, and private
desktop interval. Use the Stage 6 owner's authenticated all-52 receipt, read
the original 50 ordered focus TIFFs, and rehash before and after. The stage
owner keeps cleanup authority. Release that role after all reads are terminal.

Use the full original Hybrid retained result and physical NVIDIA Vulkan worker.
Keep the first 200 idle observations and the first 16 complete loaded rounds
through step 593, totaling 80 loaded observations. The original predispatch
state rules and all 19 witness PNGs apply. Use the independently observed
correct XWD endpoint and preserve every attempt including any failed witness.
No phase forcing, private test controls, queue markers, additional native
input, or changed redraw requests. The existing Vulkan host hooks suffice for
the first capture. Use a 240-second application/observer bound and a 280-second
outer cap, with scoped cleanup. This trace has no frozen latency gate credit.
[Original selector and endpoint](../p10-vulkan-retained-result-evidence/8694b55/PREREGISTRATION.md).

## Recording design

Add typed boundaries to the local, same-version `i-slint-backend-winit 1.19.0`
overlay and reuse the diagnostic-only `wgpu-core 30.0.2` recorder for queue,
device-lock, and present spans. Add a surface-acquire span at the existing
`present.rs::Surface::get_current_texture` call. Keep the renderer and
production dependencies unchanged. Slint's public FemtoVG rendering notifier
would submit an extra background clear in `lib.rs` before firing
`BeforeRendering`. It is outside the capture design.

1. Record arrival of the actual Winit event and both sides of the real Slint
   input handling. Use a local sequence and fixed event discriminator. Record
   logical destination coordinates for clicks and the actual buffered drag
   motion at dispatch. For wheel actions record direction and the native
   handler scope. Slint coalesces raw `CursorMoved` and flushes it before
   non-motion events or at `about_to_wait`. Raw arrival alone is insufficient
   to assert that Slint has handled the drag.
2. Record actual redraw request decisions, including already pending,
   immediate request, running frame timer, and timer-fired handoff. Pinned
   Slint X11 uses `TimerBasedFrameThrottle`, with a 60 Hz fallback rounded to
   an integer 16 ms interval. Record the actual phase in this capture together
   with request and handoff timestamps. The product's 16 ms worker status
   timer is a different timer.
3. Give each synchronous `WinitWindowAdapter::draw` a per-window ordinal and
   clock its entry and exit, including skipped/failed outcomes. Keep the
   same-thread nested surface acquire, Rust queue submit, actual typed device
   snatch-lock spans, and driver present. A completed present inside an older
   draw cannot contain an input that Slint handled after that draw ended.
   Link a new draw only to a uniquely matched input and unique fully nested
   successful present, with no intervening relevant mutation and a matching
   independent framebuffer witness. Preserve ambiguity if input coalescing,
   multiple candidate draws, or missing spans prevent a unique link.
4. Use Linux `CLOCK_MONOTONIC` end to end and the actual render TID, window
   ordinal, and existing typed device, surface, and lock information. No
   fitting an offset to the tail. Reserve bounded 32,768 records for the
   Winit plane and 32,768 for the graphics plane before measurement. The
   preceding run had 2,722 presents and 28,193 graphics spans. The latter
   would add one surface-acquire span per draw. If preflight shows the bound
   is inadequate, revise the preregistration before any capture. Reject
   overflows, truncation, stray same-role windows, reused identities, and
   incomplete spans. Hot callbacks use fixed-width fields only, with no
   allocation, disk writes, environment dump, or waits. Flush after exit.

## Attribution rule

Use the same original interval for scored and late observations. For each
uniquely bound action, first report the full breakdown from external XTest
dispatch through Winit arrival, internal handler end, draw entry, acquisition,
Rust submit, Rust and driver present, and first correct XWD.

- **Delivery while an older draw blocks.** A previous synchronous draw
  contains dispatch and remains on the actual UI thread until late in the
  observed interval. Bound only the proven overlap and note if the older draw
  is in acquire, submit, or present. Without overlap, dispatch-to-Winit time
  remains an arrival delay whose precise source may need OS scheduler data.
- **Redraw after input.** Slint input handling returns promptly and much of
  the remainder precedes draw. Attribute timer time only when the native
  redraw decision, still-pending flag and actual timer handoff agree. Report
  another event-loop or work boundary separately when measured. Do not infer
  timer involvement from a period near 16 ms.
- **Within the new draw.** Separate acquire, Rust submit, same-device
  lock ownership and driver present using the existing causal attribution
  rule. A renderer write waits behind the same lock's confirmed read hold
  only when the writer attempts inside that hold and acquires immediately
  after its release bracket. Host submit/present boundaries do not establish
  GPU execution start, queue residency or WSI display time. Preserve that
  unknown if the late interval ends there.

Report idle and loaded event counts, the original stage-at-dispatch groups,
and the same action categories. List exact unique and ambiguous counts.
Classify the longest nonoverlapping attributable interval with its actual
duration and retain other material overlapping spans. A missing tail or
ambiguous frame attribution remains unresolved. Any scheduling proposal
requires an independently supported boundary before a separate experiment.

## Pinned source references

| Source file | SHA-256 |
| --- | --- |
| `i-slint-backend-winit 1.19.0/frame_throttle.rs` | `11f4fab3b40a815756703c79d8aa71631fd39dcee936c0dec53d46f6b8199138` |
| `i-slint-backend-winit 1.19.0/event_loop.rs` | `1572e2d7d8905a325710caa7b750120565d89460f1e0b68485e006876a98689a` |
| `i-slint-backend-winit 1.19.0/winitwindowadapter.rs` | `aa97919dd16ac6f7cc74f44def6a49c30c84cba3c8148a059edd3454eccc05ee` |
| `i-slint-renderer-femtovg 1.19.0/lib.rs` | `86661891411a79cc939398f458c0585d14c715ac6a437ae8721ae815b7005f3d` |
| `i-slint-renderer-femtovg 1.19.0/wgpu.rs` | `18d89c80589e6390c770784674174cdb6c5d996c63a2fd78a126620798bb329d` |
| `fb9c399/direct-monotonic-summary.json` | `8671024c0555c23dfdb0c08b9a745084224b861f8a88fa358aee199071e33d31` |
