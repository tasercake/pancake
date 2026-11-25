# Twelve-frame Vulkan trace diagnostic

On 2026-11-18 from 19:41:31 to 19:41:58 SGT, the exact `287828a`
Debian GUI captured 12 real source zoom and pan transitions on a private
1440 × 1000 Xvfb desktop. The 12 source TIFFs matched the frozen original
prefix before and after. The RTX 3090 completed one physical
`wgpu-vulkan` Hybrid job on PCI `0000:01:00.0`. The GUI exited
cleanly. All 36 presented patches decoded independently and differed
from their exact source projections by at most one channel level.
[Detailed verification](summary.json) and [raw native observations](raw-private-witnesses.tar.gz).

All six loaded inputs had a real immediately preceding `Running · Fuse`
observation and remained in the numerical pipeline at the visible endpoint.
One source pan took 82.487 ms across Fuse with zero completed frames to
Decode with one completed frame. The largest framebuffer polling gap
during that input was 1.121 ms. These source-only observations were
instrumented and cover 12 input files, so they carry no frozen viewer
or full T50 qualification credit.

Nsight Systems 2024.2.3 completed its report and SQLite export. The
database contains the actual GUI PID 239186 and observer PID 238843.
All 24 paired CPU anchors and one warmup marker came from the observer.
The largest measured local clock envelope was 27.932 µs. The report
contains zero `VULKAN_*` data tables, so the correlation tool returned
exit 1. Its capture mode says Vulkan while its primary Python process
records `COLLECT_VULKAN_TRACE=false`. The profiler warns that Vulkan
profiling may not have started and that no Vulkan events were collected.
The data establishes the clock bridge and leaves GUI layer activation and
queue timing unverified. [Genuine NVTX rows](observer-nvtx.json) and
[exact launch output](first-diagnostic.log).

The profiler was launched around the Python recorder, which passes selected
profiler settings to the real GUI child. The report does not contain
child injection diagnostics. It cannot determine whether the Vulkan layer
was absent from the GUI child, the launcher failed to select the relevant
trace configuration, or the profiler omitted the events for another reason.
A separate direct-target empty-GUI trace can first verify that the loader
activates the layer and that the exact packaged GUI PID emits acquire,
submit and present records. It requires no TIFFs. The installed CLI
advertises those trace options. [Installed tool preflight](static-preflight.txt)
and [NVIDIA 2024.2 Vulkan guide](https://archive.docs.nvidia.com/nsight-systems/2024.2/UserGuide/index.html#vulkan-api-trace).

No competing compiler was listed in the prelaunch snapshot. A spot check
during the capture saw ordinary Rust compiler processes on CPUs 20 and 21.
Only the task-owned Xvfb/GUI/Nsight processes held the diagnostic GPU slot.
All were terminal before the offline review. No trace retry was launched.

The raw Nsight report and SQLite remain owner-readable on this machine
under `artifacts/p10-viewer-queue-trace-preflight/prefix-12/raw-first-diagnostic`.
That metadata contains the profiler process environment. The Git archive
retains the relevant decoded fields and raw integrity hashes only.
Authenticated cleanup rehashed and removed all 12 TIFFs, releasing
622,541,539 bytes. [Cleanup receipt](cleanup.json).
