# Linux package evidence

## Integrated Linux package candidate `12f2151`

The locked workspace release built both executables on Ubuntu 24.04.5 x86-64
from `12f2151c8a198b5209c8f4d506dbd17fd9aaa74f` using Rust 1.100.0. The
candidate passed `check.sh fast`, `check.sh deps`, and four package-tool
regressions. [Build and verification summary](12f2151/summary.json).

The local package is at
`artifacts/pancake-packages/linux-12f2151/pancake_0.1.0_amd64.deb`.
It is 69,404,690 bytes with SHA-256
`0d5a428309fedb6de8ed741c9ba79bb75463ac219972b87c931bbac702b02123`.
APT simulated the local installation. Rootless extraction verified the
binaries, native dynamic libraries, desktop entry, CLI help and 1,036
dependency notices, including both licenses from the pinned vendored
`oneshot`. The inventory identifies 27 of 583 selected packages whose
locked sources supplied no recognized notice. The packaging record is
informational for redistribution review.
[Package verification](12f2151/package-verification.json).
[Dependency-notice inventory](12f2151/notice-inventory.json.gz).

The embedded Slint compiler call, its single toolkit import, locked Slint
and Rowan records, and pinned Rowan tree match the earlier packaged runtime
witness at `5216ba4`. The exact Linux CLI production graph remains
independent of the UI graph. [Slint source and dependency comparison](12f2151/slint-source-boundary.log.gz).

The package remains provisional while required Apple execution, retained
platform build and release decisions, complete assurance and the unchanged
performance targets remain open. See the [implementation plan](../implementation-plan.md).

## Integrated Linux package candidate `5216ba4`

The Linux x86-64 Debian package for
`5216ba4227e14033483a21bb5eca1c6c168ef840` is retained under
`artifacts/pancake-packages/linux-5216ba4/pancake_0.1.0_amd64.deb`. It is
69,262,768 bytes with SHA-256
`f3caa23753ff1a670d895b33bff58d26ab6d48aa58f961e5bab731d985371ed1`.
The producer compiled at `bc19261` and reran the locked release build after
sealing `5216ba4`. The sealed build reused the same executable bytes. The
application sources, manifests and Cargo.lock are identical across those
revisions. The recorded differences concern assurance, tooling or
documentation. [Build source witness](5216ba4/source-stage.json).
[Sealed release build](5216ba4/sealed-release-build.log).

## P7 release checkpoint `0859deb`

Pancake's locked Linux x86-64 release CLI and GUI produced a 69,265,760 byte
Debian package on Ubuntu 24.04. The archive SHA-256 is
`d5cfb3d07f0452b56d690e633cb77af47841b8326ed4b4ea4eccec595b1d7df4`.
The workspace retains the archive under
`artifacts/pancake-packages/linux-0859deb/pancake_0.1.0_amd64.deb`. Both extracted
binaries match the release build hashes, their linked libraries resolve, and
the desktop entry validates. APT simulated installation with the installed
dependency set. The native installation test used rootless extraction into a
temporary directory and executed the resulting binaries. A separate private
desktop run launched the packaged desktop entry through `gio`, observed its
Pancake window and checked the live process arguments. Both file paths arrived
as individual arguments, including a path with spaces and non-BMP Unicode.

This checkpoint predates subsequent GUI and registration changes. Its receipts
apply to the exact `0859deb` binary, lock and descriptor. Final platform and
full-case acceptance need a combined revision and fresh matching runs.

- [Package and smoke receipt](0859deb/receipt.json)
- [Packaged desktop entry launch and argv](0859deb/desktop-entry-receipt.json)
- [Native launch and Slint source-path check](0859deb/slint-receipt.json)
- [APT simulation](0859deb/apt-simulation.txt)
