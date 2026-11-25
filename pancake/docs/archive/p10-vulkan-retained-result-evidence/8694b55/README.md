# Full retained-result Vulkan diagnostic at 8694b55

On 2026-11-18 from 20:43:41 to 20:46:44 SGT, the exact `8694b55` GUI
completed a physical Vulkan Hybrid stack with all 50 frozen T50 source
TIFFs. The preregistered sequence kept the 200-event idle prelude and
first 80 contiguous loaded events. The actual private GUI passed the
calibrated pixel checks and exited without force. All 784 observed
patches matched the 19 original PNGs within one level per RGB channel.
The trace belongs to GUI PID 585748 and contains 5,481 host submits,
2,674 acquisitions and 2,674 presents. The largest clock interval
uncertainty was 1.403 µs. No input changed.
[Preregistered scope](PREREGISTRATION.md), [independent proof](summary.json),
[observed GUI exit](gui-exit.json) and [actual visible pixels](presented-pngs.tar.gz).

The loaded tail recurred. Fifteen of the 80 loaded observations took
more than 33 ms, with a maximum of 148.177 ms. The trace was
instrumented and the loaded event count is below 200. It does not
replace the exact package's separate frozen viewer failure.

| Original step | Dispatch to correct frame | Graphics submit return to present entry | Other-thread semaphore wait | Last memory free to present entry |
| --- | ---: | ---: | ---: | ---: |
| 508 | 122.464 ms | 100.724 ms | 133.783 ms | 52.298 µs |
| 434 | 107.455 ms | 84.429 ms | 95.619 ms | 86.332 µs |
| 554 | 101.649 ms | 81.886 ms | 95.693 ms | 54.743 µs |
| 518 | 92.855 ms | 71.573 ms | 95.820 ms | 54.612 µs |

These four events each had one plausible present in the observer
interval. Graphics acquisition started within 0.62 ms of dispatch and
the last graphics Vulkan submit returned within 2.73 ms. The candidate
present itself consumed approximately 17.7–18.9 ms. The first correct
pixels followed its return by 0.47–1.45 ms. The API gap is between
**Vulkan** submit return and present entry. The Rust submit may still
have been doing maintenance or invoking callbacks. The longest event at
step 480 had two candidate presents. The trace retains that ambiguity.
[Exact ordered API and clock bounds](ordered-api-timeline.json.gz).

The trace labels two TIDs as `DSD-0-8192`. TID 585832 submitted 133
Vulkan workloads and recorded compute commands. TID 586031 made no
submissions. Its API calls covered 132 semaphore waits, 3,244
descriptor frees, 110 memory unmaps and 110 memory frees. The waits
and final frees above are on TID 586031. Host API durations establish
fence-wait overlap and timing of presentation. They do not establish
execution intervals, queue residency or which Rust lock the render
thread was waiting to acquire. Stage groups use the last observed
pre-dispatch state. Long event windows have more opportunity to cross
a worker stage, so the before/after labels alone do not identify a
cause.

The pinned CubeCL downstream device service names its thread with the
`DSD` prefix. Its WgpuStream creates an unnamed asynchronous WgpuPoll
thread. The observed wait-only API set is consistent with a child poller
inheriting the creator's Linux thread name. CubeCL's pinned WgpuPoll
requests indefinite `PollType::Wait`. wgpu-core
`Device::poll_and_return_closures` retains the read side of
`snatchable_lock` across its native fence wait and queue maintenance.
wgpu-core `Queue::present` requires the write side before entering
Vulkan present and later serializes command indices. The pinned
X11 Winit pre-present callback adds no work. These source reads supply
a concrete shared-device lock hypothesis without proving the exact
live Rust call stack. [Pinned source hashes](pinned-source-files.sha256).

CubeCL's public runtime configuration can set task aggregation and memory.
Attaching an existing wgpu device uses that same configuration. Neither
route supplies a poll policy. A Pancake-only flush and callback would
bypass per-buffer error and poison checks in `sync_buffers`.
A bounded candidate should preserve that method and every true completion,
limit each native wait only on the explicit shared UI attachment, and keep
headless or nonshared defaults. Existing lock and resource semantics should
stay intact. Another targeted trace can separately label the wait call
site if needed. No production policy or dependency changed in this step.

The raw report, SQLite, full native witness capture and process census
remain owner-only outside Git under
`artifacts/p10-viewer-queue-trace-preflight/direct-full-8694/raw-first-full`.
The Git evidence includes selected actual API rows, the ordered
derived timeline, displayed PNGs and hashes. The stage owner retains
authority for authenticated dataset cleanup after this borrower release.
[Archive checksums](SHA256SUMS).
