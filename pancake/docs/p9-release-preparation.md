# P9 release preparation

## Linux package and case accounting

Build the final locked workspace once in a clean application checkout on Ubuntu
24.04 x86-64. Keep the Cargo invocation, version, selected features, and build
log alongside the package and case artifacts. Build both executables from the
same revision. Include the retained runtime profiles and exercise them separately
where they require vendor SDKs or drivers.

```sh
cargo build --manifest-path pancake/Cargo.toml --locked --release --workspace --bins
python3 pancake/scripts/package_linux.py \
  --release-dir pancake/target/release --output artifacts/p9-linux-package \
  --maintainer "${PANCAKE_MAINTAINER}"
```

The package contains the exact CLI and GUI executable bytes and a desktop
launcher forwarding input paths. The packager derives direct ELF dependencies
from `dpkg-shlibdeps` and declares desktop and Vulkan loader dependencies that
may be loaded dynamically. It records the ELF dynamic tables, symbol versions,
package SHA-256, source revision, and lock SHA-256, then confirms extracted
binaries match the build. Keep the archive and receipt. Extract it in a
task-owned Ubuntu environment and exercise the actual packaged CLI and GUI using
their matched dependencies. Retain `ldd` output and startup, export, and
clean-shutdown evidence. CPU LLVM is statically bundled by
`tracel-llvm-bundler`; the package host still resolves native system libraries.
Vulkan ICDs, NVIDIA's driver and NVRTC, and optional ROCm/HIP dependencies remain
platform provisioning inputs. Validate available profiles with actual runtime
completions on the required hardware.
The package also archives matching source notices from the locked Linux graph
and preserves an informational inventory. [Package notice evidence](p9-package-notices.md).

Keep each bundle intact. The verifier checks the 90-file reviewed contract, 376
live catalog records against the roadmap, runner provenance, source revision,
Git tree, locks, binaries, runner reports, and package receipt. It rejects
partial or contradictory results even when the runner returns zero. Retain the
original failed or interrupted bundles for investigation.

## Apple and retained builds

On a clean native Apple Silicon worktree with Python 3.11 or newer, the
[private package recipe](../scripts/package_macos.py) builds the locked
ARM64 release with CPU, wgpu Metal, and the explicit `cubecl/metal-native`
dependency feature. Supply a stable private bundle identifier and a build
number from 1 through 9999. The display version comes from the release crates.

```sh
python3 pancake/scripts/package_macos.py \
  --output artifacts/p9-macos-package \
  --bundle-identifier "$PANCAKE_BUNDLE_IDENTIFIER" \
  --build-number "$PANCAKE_BUILD_NUMBER"
```

The output contains `pancake-cli`, `Pancake.app/Contents/MacOS/pancake-gui`,
`Info.plist`, the build log, and the package receipt. The recipe checks the
plist with `plutil`, verifies each executable's ARM64 slice with `lipo`,
and records direct loader dependencies with `otool -L`. It accepts macOS
system library and framework install names. The recipe ad-hoc signs only the
packaged copies and checks both identities with strict `codesign` verification.
The receipt records the exact source and lock, build command, selected
developer directory and SDK, compiler and SDK linker identities, explicit
SDK, deployment and linker overrides, and executable hashes before and
after signing.

The [portable tests](../scripts/test_package_macos.py) replay the staging
contract with Apple-tool fixtures. They cover the plist, command and byte
receipts, failure cleanup, and existing and concurrently created outputs.
Run them with
`python3 -m unittest discover -s pancake/scripts -p test_package_macos.py`.
Six portable tests passed on Linux x86-64. The locked Apple ARM64 feature
tree resolved `cubecl-metal` and wgpu's Metal backend on that host.
Actual native building, signing, and application identity checks need Apple
Silicon. Developer ID signing, a distribution package and its notice review,
and any notarization decision remain with the release owner. If hardened
signing is selected, verify the CubeCL CPU JIT with the final signing
options and review any required entitlement narrowly.

On an Apple Silicon host use the configured dedicated Tart guest for the
34 compatible interface cases. The suite reconstructs its guest target
checkout at each sync, so stage the matching package outside the suite-owned
guest checkout and point the guest GUI command to the packaged executable.
Record guest binary, package, lock and descriptor hashes before and after
execution, plus the native driver logs and host parity results. Keep a separate
physical Apple CPU/wgpu Metal/native Metal execution receipt. The available
Linux machine provides actual NVIDIA hardware and private native desktop
transport. Apple application execution and macOS package verification need an
Apple Silicon host and access to its isolated desktop or guest.

Keep Linux HIP build/link and Windows MSVC build/link checks under the same
source and lock. Retain distinct build receipts for each profile, compiler,
SDK, feature set, revision and command. AMD/Intel GPU and Windows execution
are best effort under the [milestone plan](implementation-plan.md).

## Slint interpreter and Rowan boundary

The release decision retains the safe interpreter for the embedded,
repository-owned Slint source. The `12f2151` compiler call, built-in source URI,
single toolkit import and locked Slint and Rowan dependencies match the
`5216ba4` packaged runtime witness. The native window opened with a
colocated widget file present, and the file-open trace recorded zero reads
of that file.
The applicable Rowan strict-Miri ARC retag finding remains open. The P0
representation patch leaves that ARC implementation untouched. Keep the
first-party unsafe prohibition and nightly Miri gates and re-review the
boundary if the application adds external Slint source or imports.
[Interpreter decision](p9-slint-boundary.md).
[Integrated source comparison](p9-linux-package-evidence/12f2151/slint-source-boundary.log.gz).
[Rowan probe and patch evidence](p0-kernel-qualification.md#rowan-layout-boundary).

## GPU AddressSanitizer controls

The standalone P0 Vulkan C probe ran on Ubuntu x86-64, Ryzen 9 7950X3D and
RTX 3090 with driver 580.178.04. It created a native Vulkan device and exited
zero. The same probe built with GCC 13.3 AddressSanitizer also created the
device and then exited 1. LeakSanitizer reported 2,274 bytes across three
allocations. The stacks include D-Bus allocation and `vkCreateInstance`.
Private D-Bus and a scrubbed display environment yielded 2,522 bytes across
five allocations. Disabling implicit layers in that diagnostic yielded the same
2,522 bytes across five allocations. The native setup works and the sanitizer
leaks reproduce outside Rust, including with implicit layers disabled.

The integrated replay at `fcf395b` reproduced both outcomes. Minimal wgpu and
standalone C created the RTX 3090 Vulkan device under ASan and each reported
2,522 leaked bytes across five allocations, with matching D-Bus allocation
frames. The direct CubeCL request in the same Rust executable and the unchanged
application witness returned `Parent device is lost`. The full GPU ASan gate
failed at its first test. Isolating the failing descriptor field remains open.
CPU ASan/TSan and physical GPU execution retain their separate gates.
[Integrated replay](p9-gpu-asan-evidence/fcf395b/README.md).
[Earlier C control](p9-gpu-asan-evidence/receipt.json).
[P0 Rust diagnostic](p0-sanitizer-investigation.md).

## Preparation tests

The packaging tests compile temporary native ELF fixtures, exercise Debian
assembly and extraction, and verify executable hashes and the desktop launcher.
Ledger tests use the unchanged runner report serializer to exercise disjoint
lane accounting, strict rejection, raw event checks, and package matching.
The application and package execution receipts are in the
[Linux package evidence](p9-linux-package-evidence/README.md).
