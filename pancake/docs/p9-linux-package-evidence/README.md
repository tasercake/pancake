# Linux package evidence

## P7 release checkpoint `0859deb`

Pancake's locked Linux x86-64 release CLI and GUI produced a 69,265,760 byte
Debian package on Ubuntu 24.04. The archive SHA-256 is
`d5cfb3d07f0452b56d690e633cb77af47841b8326ed4b4ea4eccec595b1d7df4`.
The workspace retains the archive under
`artifacts/pancake-packages/linux-0859deb/pancake_0.1.0_amd64.deb`. Both extracted
binaries match the release build hashes, their linked libraries resolve, and
the desktop entry validates. APT simulated installation with the installed
dependency set. The native installation test used rootless extraction into a
temporary directory and executed the resulting binaries.

This checkpoint predates subsequent GUI and registration changes. Its receipts
apply to the exact `0859deb` binary, lock and descriptor. Final platform and
full-case acceptance need a combined revision and fresh matching runs.

- [Package and smoke receipt](0859deb/receipt.json)
- [Native launch and Slint source-path check](0859deb/slint-receipt.json)
- [APT simulation](0859deb/apt-simulation.txt)
