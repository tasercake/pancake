# P9 Linux package license and native LLVM provenance

The Pancake library, CLI and GUI Cargo manifests declare
`GPL-3.0-or-later`. The Debian notice inventory binds all three first-party rows
to one physical `pancake-0.1.0/LICENSE`. The committed text is the 35,149-byte
GPL-3 text from Ubuntu `base-files 13ubuntu10.5` at
`/usr/share/common-licenses/GPL-3`. It matches the installed package's recorded
MD5 `1ebbd3e34237af26da5dc08a4e440464` and has SHA-256
`3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986`.

## Native LLVM and staged notices

Linux x86-64 uses the locked `tracel-llvm-bundler 23.1.0-4` Cargo wrapper and
its separate `tracel-llvm-23.1.0-4-linux-x64.tar.xz` native bundle. The publisher
sidecar matches the pinned archive and installed content SHA-256. The bundle's
`include/llvm/Support/VCSRevision.h` identifies upstream LLVM commit
`ea7d852a70e8bdfaf601d6626a760f9771b2c4b4`. The staging manifest enumerates
and hashes the original Support pointer notice, full LLVM project license from
that revision, and the BLAKE3 license. The actual selected `libLLVMSupport.a`
contains eight BLAKE3 member names. [Bundle source](https://github.com/tracel-ai/tracel-llvm).
[Notice manifest](../../notices/llvm/manifest.json).
[LLVM project license](https://github.com/llvm/llvm-project/blob/ea7d852a70e8bdfaf601d6626a760f9771b2c4b4/llvm/LICENSE.TXT).
[BLAKE3 license](https://github.com/llvm/llvm-project/blob/ea7d852a70e8bdfaf601d6626a760f9771b2c4b4/llvm/lib/Support/BLAKE3/LICENSE).

The sealed `e81079dee4078d7a1149da0f5950aab8c89f8d44` Ubuntu 24.04 release
Cargo log and build artifacts identified 112 static LLVM archive directives.
Every selected member was present byte-for-byte in the existing CubeCL LLVM
release rlib, with 2,182 object digests and matching multiplicity. The exact
rlib SHA-256 is
`361beb7d27ee4496a5541f00cd1a873d6f11626124a8e961cb118c89f80a4a77`.
The native receipt matches the sealed Debian source commit, Cargo.lock,
and both package executable identities. The tested Debian archive SHA-256 is
`8ed6e70d7552140c0257520a07ac2b9fc5f97ff82a5fb3126864df821df4507e`.
[Bound native build receipt](p9-package-notice-evidence/e81079d-native-llvm.json).

This evidence covers the selected archive members embedded in the CubeCL rlib.
The final linker can drop unreferenced objects. The recorded texts cover the
pinned LLVM license, BLAKE3 and the bundle's Support pointer. Separate
wrapper-crate source notices, any remaining third-party attribution, source
materials and external distribution terms require release-owner review.

## Reproduce from the existing locked target

Keep the exact target and Cargo build-script outputs until the certificate is
created. Pass the `run/stdout` from the CubeCL LLVM build script and the rlib
from that target. Run the verifier against the clean application source checkout
that produced the binaries. The verifier checks the publisher sidecar, cached
compressed archive, installed tree, selected archive path and rlib object
bytes. It writes report fields with hashes and sizes, archive basenames and
pinned product identifiers. Historical sealed archives remain usable with
their original case ledgers.

```sh
python3 -B pancake/scripts/package_native_llvm.py \
  --source "$app_source/pancake" --release "$release_dir" \
  --build-output "$cubecl_build_stdout" --rlib "$cubecl_release_rlib" \
  --archive "$llvm_bundle.tar.xz" --sidecar "$llvm_bundle.checksums.json" \
  --installed "$llvm_install" > "$native_llvm_receipt"
```

For a new local Debian candidate built from the packaging source revision,
pass `--native-llvm-receipt "$native_llvm_receipt"` to
`pancake/scripts/package_linux.py`. It rejects stale source, lock or binaries,
then stages the receipt and notices under `/usr/share/doc/pancake/native-llvm`
and verifies all five file identities after rootless package extraction. The
package receipt includes those paths and hashes. The sealed `e81079d` Debian
archive predates these added notices. The unchanged application build supplied
the native-member proof. The nine package tests passed, including rootless
build, extraction and packaged CLI launch with synthetic executables and
notice fixtures. A new product package needs its own receipt and archive
digest.

```sh
python3 -B -m unittest discover -s pancake/scripts -p test_package_linux.py
python3 -B -m unittest discover -s pancake/scripts -p test_package_native_llvm.py
```
