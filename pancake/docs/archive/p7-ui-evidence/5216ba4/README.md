# P7/P8 native workflow evidence

The Linux release candidate is `5216ba4227e14033483a21bb5eca1c6c168ef840`.
The producer built both binaries with Cargo and Rust 1.100.0 using
`CARGO_INCREMENTAL=0 cargo build --locked --release --jobs 4 -p pancake-gui -p pancake-cli`.
[Build and binary hashes](../../p9-linux-package-evidence/5216ba4/sealed-release-build.log).

The isolated GUI lane passed all 34 selected cases with Slint software
presentation. It covered all 20 P7 and 14 P8 IDs. The unchanged native
64-frame Laplacian/breathing workload used generated 1536×1024 RGB inputs
and exercised pause, resume, cancel, and successful worker reuse.
It also validated GUI and CLI export parity, native panels, Unicode names,
retained results, compare, crop, history, context menus, and recovery.
[Exact interface accounting](interface-accounting.json).

The unchanged Python runner owned each private Xvfb display, Openbox window
manager, D-Bus session, accessibility bridge and test profile. The host ran
Ubuntu 24.04.5, Linux 6.8.0-147, and an AMD Ryzen 9 7950X3D with 62 GiB of RAM.
The staged native stack used Xvfb 21.1.12, Openbox 3.6.1, xdotool
3.20160805.1, GTK 3.24.41, and ATK 2.52.0. The Openbox shim enabled only the
isolated accessibility service. The Xvfb shim passed `-noreset` while startup
services connected. [Private desktop setup](../../p7-work-log.md#isolated-desktop-setup).

Run both captures from the clean release checkout with the staged local tools
on PATH. Prepare the two output names under a new, short local directory in
`artifacts/` so the desktop can create its private sockets.

The joint release verifier accepted 341 host, 34 isolated GUI, and one
physical GPU GUI case with matching revision, lock, binaries, and Linux
package. Raw reports, fixtures, native action records, and exact receipts
remain under `artifacts/p9-5216ba4/{h,g,p}`. The complete ledger is
`artifacts/p9-5216ba4/linux-ledger.json`. Apple execution, retained
platform builds, CPU source coverage, TSan, and the measured P10 targets
require separate evidence.
