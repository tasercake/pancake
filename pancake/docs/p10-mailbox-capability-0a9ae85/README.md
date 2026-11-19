# Vulkan presentation mode on the current private desktop

The exact Vulkan surface advertised **FIFO and Immediate**. All four successful
configurations resolved to FIFO, including startup and a real native resize.
The capability result closed the Mailbox experiment before a candidate release
build. The result applies to this physical NVIDIA Xvfb surface.
[Recorded configurations](mode-contract.json) and [capability decision](control-result.json).

| Configuration | Window pixels | Requested | Resolved | Desired frame latency |
| --- | ---: | --- | --- | ---: |
| Startup | 1170 × 730 | Fifo | Fifo | 2 |
| Startup resize | 1170 × 730 | AutoVsync | Fifo | 2 |
| Startup resize | 1170 × 730 | AutoVsync | Fifo | 2 |
| Real X11 resize | 1106 × 666 | AutoVsync | Fifo | 2 |

All rows came from the same successful live surface. The private X11 driver
observed the application title and viewport resize and kept the original
native close path. The GUI had 83,533,824 process-attributed bytes on the
RTX 3090 at PCI `0000:01:00.0`, driver 580.178.04, and exited cleanly. The
host observed the functional run from 01:27:56–01:27:58 SGT on 19 November
2026. This was an empty-window startup and resize contract. The frozen
loaded viewer latency gate remains open. [Prior timed diagnostic](../p10-viewer-pipeline-trace/README.md).

The configure-only witness reported the capabilities already supplied to
the stock wgpu configuration, and the resolved mode after successful HAL
configure. Slint's stock resize path requests AutoVsync. Pinned wgpu-core
resolves it to FifoRelaxed or Fifo based on the advertised capabilities.
Immediate is the other mode on this surface and can tear. The measurement
keeps the cause of the previous driver-facing present intervals open.

The control release was built from product `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`
with same-version Slint and wgpu diagnostic overlays at `330c587612d792edc08be03789afc414caa5045a`.
The exact ELF SHA-256 is `7edfac5009e12672c8b847de2db2820fd5d85cd61fe6d5ca20d1bdb9a3d8cb75`.
The same-mode control uses the stock selection. A CPU test first failed on
the unimplemented advertised-Mailbox selector, then passed the candidate
selection and stock fallback cases with the same wgpu mode types. The
actual control release built with `taskset -c 2,3 cargo build --offline
--locked --release -j 2 -p pancake-gui --bin pancake-gui`. A Python contract
check also covered capability mask interpretation, mode contrast and PID
binding. [Build identity](control-build.json) and [overlay provenance](overlay-provenance.json).

The first operator attempt compared a normalized PCI tuple with an address
string. It exited before creating the private desktop or starting the app.
The corrected operator normalized both addresses and performed one native
contract on the same executable. The owner-only raw and operator receipts
are in `artifacts/p10-mailbox-0a9ae85/`. The private caches, session runtime and
profile were removed. [Initial setup receipt](control-invalid-setup.json)
and [raw evidence hashes](control-sealed-receipts.sha256).
