# Direct exact-GUI Vulkan API trace

The directly profiled `287828a` Debian GUI produced actual host Vulkan
API events on 2026-11-18, 20:03:12–20:03:30 SGT. The observer verified
the private window's PID 413946 against the exact executable, recorded
83,533,824 bytes of residency on the matched RTX 3090 and retained the
1440 × 1000 XWD display. SQLite identifies six queue submits, two image
acquisitions, and two queue presents from that PID. The Vulkan loader
log records two instance insertions and one device insertion of the
installed Nsight layer. This smoke used no TIFFs or worker.
[Independent verification](summary.json) and [actual display](display.png).

The external window-close command was sent but the launcher timed out
waiting 13 seconds for Nsight. Scoped cleanup stopped the processes.
The trace and SQLite were still generated, and the launcher exited 1.
The trace verifies API transport and leaves clean shutdown unproven.
The original attempt was retained. No retry ran.
[Exact command](nsys-command.json) and [launcher timeout](first-smoke.log).

Both this positive report and the earlier zero-Vulkan-row report mark
the per-process `COLLECT_VULKAN_TRACE` metadata flag false. Loader
insertion and the matching actual API rows provide the discriminating
proof. The raw profiler report, SQLite and loader log remain owner-only
outside Git under
`artifacts/p10-viewer-queue-trace-preflight/direct-smoke/raw-first-smoke`,
because the profiler retains process environment metadata.
[Checksums](SHA256SUMS).
