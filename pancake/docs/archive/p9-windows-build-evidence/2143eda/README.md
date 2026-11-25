# Windows build route check at 2143eda

The inspected product revision was
`2143eda2b308497caee96334357dd23d324e1e8e`. The workspace and frontend Cargo
manifests, lockfile, Rust pin and Windows CI command match the earlier
`12f2151c` investigation. The lock SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Application source has advanced since that investigation.
[Prior MSVC target and LLVM linker diagnostics](../12f2151/README.md).

## Available native builders

Read-only `gh run list --workflow pancake.yml --commit 2143eda2b308497caee96334357dd23d324e1e8e`
against `tasercake/focus-stacking` returned no runs for that revision. The
repository's `actions/runners` API returned zero self-hosted runners. The
workflow still defines the native hosted `windows-2025` all-target,
all-feature MSVC build/link job with `cargo test --no-run`. The retained P0
job demonstrates that build-only lane for its older revision. [P0 Windows build receipt](../../p0-close-evidence/windows-msvc-build.json).

Local read-only libvirt inventory found an inactive Windows recovery guest.
Its inactive device record includes a writable USB storage image and no QEMU
guest-agent channel. It remains unprovisioned for this build. No guest boot,
disk contents access, or guest changes occurred.

## Windows LLVM artifact

The locked LLVM bundler's matching platform asset exists in upstream release
`v23.1.0-4` as `windows-x64.tar.xz` (59,586,340 bytes). The GitHub release
API archive digest and the original Windows checksum sidecar agree on
`6639be1a76a8e48ce1bd5b698a13882a169b914c585eaa3b11fdffe62832d915`.
The downloaded sidecar SHA-256
`5a0a50748e07f2d9de4515cde47e9bcb0f894eb38b604305c506b03d6cbb154e`
also matches the GitHub asset digest. These are upstream artifact identity
checks. [Upstream release](https://github.com/tracel-ai/tracel-llvm/releases/tag/v23.1.0-4) and [retained checksum sidecar](windows-x64.checksums.json).

The exact build scripts choose the LLVM archive, native `llvm-config` name
and C++ runtime using host compilation cfg. Selecting the MSVC Cargo target
from Linux does not change those host cfgs. The Linux loader would still
seek `llvm-config` and Linux C++ metadata even with an independently staged
Windows archive. The host also lacks a Windows Rust target and a provisioned
MSVC SDK/CRT. This verification changed no configuration and did not repeat
the failed build. [Cargo build-script cfg semantics](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

**Disposition:** the current native Windows build/link is unverified. A
dedicated authorized Windows builder or the retained hosted job can provide
it on the release source. Windows application execution remains unrun.
