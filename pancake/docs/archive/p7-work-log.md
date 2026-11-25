# P7 native UI development

The first safe Slint application checkpoint is
`ea487ef3bbbce3e04212895aa758f479aca79be3`. Its isolated Linux native run
passed these real application cases, using the unchanged contract and a
descriptor that pointed at the debug GUI binary.

The checkpoint used Rust 1.100.0, Ubuntu 24.04 x86_64 kernel
6.8.0-147-generic and a Ryzen 9 7950X3D host with a GeForce RTX 3090 and NVIDIA
580.178.04. These six cases exercise visual controls, preferences and native
input. They supply no physical GPU computation evidence.

Run `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo build --locked
--jobs 4 -p pancake-gui` from `pancake/`. The tested executable SHA-256 was
`f7e8bc7cc759895f3b6eb49e027719dc883b5332625a967e4c4bcef72aae5ee5`.
Cargo.lock SHA-256 was
`6b2cac8f004397ed97a4fb45cab802acccc86bf3e33e67de33074f8ab75864e3`.
The focused GUI Rust tests and strict Clippy checks with the
policy-generated configuration passed. The marked checkpoint has no
integrated GPU job or release build result.

### Isolated desktop setup

Source import, render, stack jobs, export, lifecycle and the remaining GUI
cases require further implementation and independent verification.

### Shared job and desktop workflow integration

The decoded source/viewer slice passed its native startup, layout, zoom and
fit checks. The unchanged weighted-average case subsequently selected actual
frames in the app, completed the shared worker and exported genuine TIFF16
pixels with CLI parity. The native RAW import/merge/export case passed on the
same staged host engine.

The software-presentation debug checkpoint `8f1da27b65c` passed 9 of 11
selected GUI cases with native input on Linux. The real 64-frame lifecycle
paused, resumed, cancelled, settled and successfully reused the worker.
The native export matrix, retained results, signed pixel crop, viewport
snapshot, compare/navigation, non-BMP paths and recovery cases passed.
The two failures occurred when users reopened an in-process GTK picker
immediately after cancelling it. The [raw report](p7-ui-evidence/workflow-debug-a.json)
and [provenance](p7-ui-evidence/workflow-debug-a-provenance.json) retain
every selected status. This development run used the debug binaries from
that checkpoint while strict-lint follow-ups were being edited.

The follow-up uses a private GDK display round-trip and restores focus to
the Slint owner after native panel closure. It also loads repository-owned
Slint source under a builtin URI, so source resolution stays in the pinned
widget bundle, and introduces a four-entry display-only preview cache.

### X11 pointer handoff

After the asynchronous GTK conversion, actual Openbox focus returned to the
Slint workspace before an immediate second click. The root X11 observer
recorded both physical clicks at the Open button. The Winit event observer
recorded the second press and release after a final coordinate at the
dismissed GTK dialog, with the motion back to Open absent from the Winit
stream. The
[X11 trace](p7-ui-evidence/native-pointer-x11.jsonl.gz),
[Winit trace](p7-ui-evidence/native-pointer-winit-events.log), and
[failed case](p7-ui-evidence/native-pointer-red.json) preserve that sequence.

On the first delivered press following native GTK dismissal, the Linux
application compares the Winit coordinates with a safe X11 pointer query
relative to the actual workspace window. A stale pair is delivered once to
Slint with the queried position and its original button and event order.
Other events follow their normal Winit path. GTK file windows also name
the actual Slint owner as their native X11 transient parent. The observer
build was used only to localize the fault and its logging code was removed.

The private Xvfb launcher adds `-noreset` to preserve its display socket
across accessibility service activation before Openbox starts. The
unchanged runner still owns startup, teardown, and physical input.

### Matched Linux release

The sealed `5216ba4` release passed all 34 isolated native GUI cases and
the separate physical RTX 3090 GUI case. The unchanged file panel case
passed the immediate Cancel and reopen sequence, and the original native
64-frame workload passed pause, resume, cancel, and worker reuse. The
joint package verifier matched these 35 cases with all 341 host cases.
[Release and native input receipts](p7-ui-evidence/5216ba4/README.md).
