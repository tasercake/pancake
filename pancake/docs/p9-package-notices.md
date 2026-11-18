# P9 Debian package dependency notices

The Debian package stages original dependency notice files under
`/usr/share/doc/pancake/dependency-notices`. The inventory records every package
in Cargo's locked default-feature Linux CLI and GUI graph, including build
dependencies. The package identity and executable bytes are unchanged by the
notice payload. The package receipt records the notice inventory SHA-256 and
the rootless extraction check verifies the inventory and every archived file.

The inventory selects packages from `cargo tree --locked --offline --target
x86_64-unknown-linux-gnu --edges normal,build` for both application binaries,
then joins the selected names and versions to Cargo metadata and Cargo.lock.
Vendored and first-party files come from the named Git revision. Selected
vendor entries include the repository's upstream crate archive hash, VCS
identity and patch record where available. Registry files come from local
crate archives that match their Cargo.lock hashes. The inventory preserves
original names, byte counts and hashes for discovered license and notice
files, including declared `license-file` paths and files in `LICENSES`
directories. Dev-only and inactive optional crates stay outside the selected
package inventory.

Notice availability and declared licenses remain informational under the
existing dependency policy. Packages with missing files or unavailable
archives receive explicit inventory statuses. This records discovery within
the locked Cargo graph and leaves distribution approval with the release
owner. System libraries resolved by APT remain external packages. Native
build artifacts downloaded by build scripts need their own source and
notice receipts. [Dependency policy](dependency-policy.md).

## Verification

Run the package tests on Linux x86-64 with Cargo, Git, a C compiler and Debian
packaging tools. The package test extracts native executables and notice
files, checks unchanged executable and notice bytes, and confirms the
inventory excludes a dev-only crate. The registry test exercises the locked
archive checksum and path checks.

```sh
python3 -m unittest discover -s pancake/scripts -p test_package_linux.py
```

## Verified notice archive

The original `5216ba4` Debian archive lacked the dependency notice directory.
The failing test reproduced that omission before the packaging change. A
rootless repack used both executables extracted from the original archive,
and selected notice sources from a clean checkout of
`5216ba4227e14033483a21bb5eca1c6c168ef840` with its original Cargo.lock.
The updated package has the same version, architecture, maintainer,
dependency metadata and byte-identical CLI and GUI executables. Its
SHA-256 is
`b645bcba77d0d408d80245d78fde6a41a74a169bf412e9a07dba438a227f736d`.
The workspace retains it under
`artifacts/pancake-packages/linux-5216ba4-notices/pancake_0.1.0_amd64.deb`.

| Locked Linux graph | Packages | With notice files | Without a matching file | Files archived |
| --- | ---: | ---: | ---: | ---: |
| Original tested candidate `5216ba4` | 583 | 556 | 27 | 1,036 |
| Vendored oneshot source `a270bd5` | 583 | 556 | 27 | 1,036 |

Both inventories identify the 583 selected name/version pairs. The patched
graph selects committed `oneshot 0.2.1` source and records the pinned upstream
archive and receiver patch. Its two notice files are byte-identical to those
drawn from the registry archive in the original graph. The patched audit
staged and checked the actual files without attributing the patched source
to the older executables.
[Original archive inventory](p9-package-notice-evidence/5216ba4-inventory.json.gz).
[Patched graph inventory](p9-package-notice-evidence/a270bd5-inventory.json.gz).
[Patched oneshot witness](p9-package-notice-evidence/a270bd5-oneshot.json).

The 27 missing entries include the three first-party Pancake packages and
vendored `cubecl-std`. The receipt lists their exact versions and declarations.
The local Tracel LLVM cache archive contains a short LLVM Support notice. Its
complete notice set and exact association with the linked native LLVM bundle
remain open. For any external distribution, the release owner also needs to
decide which terms and source materials accompany the package, including the
first-party license and the entries without matching source files. The current
archive preserves discovered notices and license declarations for review.
