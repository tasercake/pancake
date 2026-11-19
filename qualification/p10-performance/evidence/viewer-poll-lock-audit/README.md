# Shared-device viewer poll and present audit

Source review at `c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb` covered
`cubecl-wgpu 0.11.4` and `wgpu-core`, `wgpu-hal` and `wgpu 30.0.2` from the
locked registry. The separately owned diagnostic used the sealed GUI at
`8694b555666a1ddf4051e2839bbced4bcc625859`, with GUI ELF SHA-256
`62e77cb6305514c2a1bbd1bcf3fc4e1518cbae572d48483e4e5e2f938daacf06`, on a
GeForce RTX 3090 at `00000000:01:00.0`, driver `580.178.04`. The 80 loaded
events included 15 late samples. All 784 captured RGB patches passed with
maximum error 1. This audit ran no application, build or hardware test.

The local raw and reduced artifacts are under
`artifacts/p10-viewer-queue-trace-preflight/direct-full-8694/`. Independently
verified SHA-256 values are

- `raw-first-full/queue-utc.sqlite` (35,758,080 bytes)
  `c6210c9e64f922241aa986934af0a54424585c4fedc90433bab44652b1f9f9a8`
- `verified-full.json`
  `90a24c2f6348329ffbd808d22e28aca13bac980eb651ed041ae08153436aeeb7`

## Trace and worker attribution

The raw `ThreadNames` and `VULKAN_API` tables independently identify these
roles. Durations are maximum recorded waits for each thread.

| TID | OS thread name | `vkQueueSubmit` | `vkAllocateCommandBuffers` | `vkWaitSemaphores` | Max wait |
| --- | --- | ---: | ---: | ---: | ---: |
| 585748 | `pancake-gui` | 5348 | 9 | 2677 | 12.112 µs |
| 585832 | `DSD-0-8192` | 133 | 2 | 6 | 1.783 µs |
| 586031 | `DSD-0-8192` | 0 | 0 | 132 | 232.036 ms |

The long-wait thread's other Vulkan APIs were 3244 `vkFreeDescriptorSets`,
110 `vkUnmapMemory` and 110 `vkFreeMemory`. This is consistent with the
unnamed `WgpuPoll` child spawned on stream construction from the scheduler
path (`cubecl-wgpu` `compute/schedule.rs:110`, `compute/stream.rs:212`,
`compute/poll.rs:18`). Rust's unnamed thread start leaves the OS thread name
in place. The CubeCL wgpu server uses `SchedulerMultiStream` (`compute/server.rs:92`),
so the generic event-stream collector is outside this path. Other native
CubeCL waits occur inline during allocator relocation (`compute/copies.rs`),
queued-write flush and submission throttling (`compute/stream.rs`), or GPU
timestamp timing collection (`compute/stream.rs`, `compute/timings.rs`).
Pinned wgpu also waits while reconfiguring a surface. The displayed thread
name alone provides no call-site attribution.

For four steps with a unique native render present, the predecessor wait and
free are on TID 586031. Values below are milliseconds. The last column uses
the reported dispatch-relative present-return interval and is rounded.

| Step | Wait | Free memory | Free return to native present entry | Native present | Present return to visible |
| --- | ---: | ---: | ---: | ---: | ---: |
| 508 | 133.783 | 3.936 | 0.052 | 18.721 | 0.983 |
| 434 | 95.619 | 3.378 | 0.086 | 18.858 | 1.447 |
| 554 | 95.693 | 3.149 | 0.055 | 17.714 | 0.466 |
| 518 | 95.820 | 3.658 | 0.055 | 18.170 | 0.926 |

The latest 148.177 ms sample, step 480, has two possible presents and cannot
support this same assignment.

## Lock mechanism and discriminating check

`wgpu-core` `device/resource.rs:793–805` takes the device's snatch read
guard before entering `maintain`. That call waits on the native fence and
triages completed submissions under the guard (`:835–940`). The Vulkan HAL
wait uses `vkWaitSemaphores` on the timeline fence. `present.rs:303–356`
prepares the surface texture and then takes the same device's snatch write
guard to extract it. It releases that guard before the separate command
indices lock and native present. The queue-submit wrapper performs host
maintenance after its native submit (`device/queue.rs:1538–1565`). Native
submission return therefore leaves open when the render thread reached
Rust present preparation, snatch acquisition or command indices acquisition.
The trace also leaves open the exact scope of the recorded driver free.

Capture the poller identity and timed acquire/release on the same snatch
lock instance, with the render prepare/submit/present boundaries and frame
ownership. A render write attempt inside the poll read interval that exits
just after release supports the lock-contention hypothesis. Arrival after
release or a stall at the separate command indices lock distinguishes other
causes. Preserve Vulkan and visible XWD boundaries so a fall in pre-present
host delay can be separated from time inside the native present call.

## Bounded shared-device experiment

A finite nonzero wait in the existing native `WgpuPoll` loop is a narrow
intervention. Keep ordinary CLI polling unbounded. Route the renderer's
setup through one explicit shared-device registration. `try_init_setup`
already registers the selected device. Calling `try_init_device_with_api`
on its result would register a second server. The current Pancake setup
cache is keyed by selector, so verify that a cached ordinary setup cannot
silently select the ordinary policy for a shared setup. Preserve the GUI's
current `job::choose_runtime` eligibility, including its handling of CPU,
indexed GPU, and explicit runtime selection.

Treat only the expected timeout as a normal retry. `Device::maintain`
still collects completed callbacks and cleanup on a timeout. Retain the
read/map and submitted-work callbacks, loss/error handling, active handle
checks and idle park/wake behavior. The wait slice is a HAL fence timeout.
Resource maintenance, OS scheduling, earlier commands on the same queue,
and native present all remain part of visible latency. An instrumented
probe can check attribution and synchronization before a separate frozen
viewer comparison. This source audit confers no performance credit.

Thread counts can be reproduced locally with SQLite opened read-only and
immutable against the verified raw path.

```sql
SELECT globalTid & 16777215 AS tid, value
FROM ThreadNames JOIN StringIds ON StringIds.id = ThreadNames.nameId
WHERE tid IN (585748, 585832, 586031);

SELECT globalTid & 16777215 AS tid, value, COUNT(*), MAX(end - start)
FROM VULKAN_API JOIN StringIds ON StringIds.id = VULKAN_API.nameId
WHERE tid IN (585748, 585832, 586031)
GROUP BY tid, value ORDER BY tid, COUNT(*) DESC, value;
```
