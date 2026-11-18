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

Build and link the locked macOS ARM64 release natively with CPU, wgpu Metal and
native Metal enabled. Stage a macOS application bundle with the GUI under
`Contents/MacOS`, bundle metadata and a versioned application identity. Ship a
matching CLI and preserve the resolved `otool -L` dependencies. Verify a local
signature appropriate to the test environment and the packaged GUI's actual
Accessibility and Automation identity. Distribution signing needs a verified
Developer ID identity and the release owner's distribution credentials. If
hardened signing is selected, verify the CubeCL CPU JIT with the final signing
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

Retain the safe interpreter for the embedded, repository-owned Slint source.
Before release, audit the final compiler call sites and embedded imports so that
input images, preferences, paths and other external data cannot supply Slint
source. Review the exact Slint and Rowan lock and source, and retain the
application assurance results together with the existing Rowan strict-Miri
failure. The P0 representation patch leaves the ARC implementation untouched.
The first-party unsafe prohibition and nightly Miri gates continue to apply.

An upstream Rowan repair with the independent reproducer passing can clear that
specific finding. Moving to compiled Slint glue would require reviewed policy
for an isolated generated-code crate and evidence that the compiler and Rowan
are absent at runtime. The integrated interpreter boundary review is pending
the final GUI revision. [P0 UI review](p0-ui-qualification.md).
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

Keep this result distinct from the unresolved CubeCL device acquisition failure
under Rust ASan in P0. On the integrated build, repeat the minimal wgpu versus
CubeCL setup control, holding driver and build constant. Vary one device
descriptor condition at a time (features, limits, memory hints, experimental
features) and identify both actual device completion and ASan verdicts. Keep
CPU ASan/TSan and physical GPU execution gates. No sanitizer suppression was
introduced. [Current traces and provenance](p9-gpu-asan-evidence/receipt.json).
[P0 Rust diagnostic](p0-sanitizer-investigation.md).

## Preparation tests

The packaging tests compile temporary native ELF fixtures, exercise Debian
assembly and extraction, and verify executable hashes and the desktop launcher.
Ledger tests use the unchanged runner report serializer to exercise disjoint
lane accounting, strict rejection, raw event checks, and package matching.
These test the release tooling. The unchanged application's release acceptance
still needs real case, package, assurance, hardware, and performance results.
