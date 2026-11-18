# Independent oneshot receive review

The crates.io archive SHA-256 is
`cfe21416a02c693fb9f980befcb230ecc70b0b3d1cc4abf88b9675c4c1457f0c`.
All 44 file paths and executable modes match the published archive. Forty-three
files match byte for byte, including the Cargo manifests, crate-local lockfile,
VCS metadata, tests, and both license texts. The only modified file is
`src/receiver.rs`, original SHA-256
`5beb04d077c4cfd29747739f80ef9a3a58135c08db573bd9c269044f8e579188`, patched
SHA-256 `065a10346822ee15a039a68565ff22c8d14df56b1513060edf54dc4117ad6ce5`.
The root lock differs only by the origin of this exact version. Cargo resolves
one copy through CubeCL Environment with `std`. Tool pins, workspace lints,
coverage rules, dependency severities and frozen performance targets are
unchanged by the adoption.

## Synchronization and ownership

- The consuming receiver's initial acquire load synchronizes with a release
  publication of `MESSAGE` or `DISCONNECTED` before the receiver reads data
  or frees the channel. The sender's terminal fast path performs no subsequent
  channel access.
- The registration exchange releases the receiver's initialized waker to the
  sender. If the exchange instead observes a terminal send or disconnect, the
  acquire half synchronizes with that publication. The sender never saw the
  receiver registered in that branch, so the receiver owns the unused waker.
- In the parked path, the sender takes ownership of the waker from
  `UNPARKING`, publishes the final state, then wakes via its local waker. The
  receiver still acquires the final state before taking the message and
  freeing the allocation. The sender's remaining wake operation uses its
  local waker handle. Spurious wakes still leave the receiver parked until a
  terminal state is observed.
- The non-consuming, timeout, asynchronous, and endpoint drop implementations
  retain upstream code. CubeCL's active runner paths call the consuming
  `recv`. Its timeout call is in a CubeCL unit test. The unchanged deadline
  path still has separate wake and state ordering, including an acquire swap
  on timeout and a relaxed terminal observer after the sender enters
  `UNPARKING`. The narrow instrumentation change does not qualify every
  upstream API under ThreadSanitizer.

The acquire operations preserve the original synchronization needed by the
terminal branches and expose it directly to ThreadSanitizer. [Rust atomic fence semantics](https://doc.rust-lang.org/std/sync/atomic/fn.fence.html#atomic---fence) and [Rust ThreadSanitizer limitations](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html#threadsanitizer).

## Independent checks

Extracted the exact Git tree for the vendored package outside the Pancake
Cargo workspace and ran the upstream tests with their published lockfile.

| Command with that standalone crate as working directory | Result |
| --- | --- |
| `RUSTFLAGS=-Dwarnings cargo +1.100.0 test --locked --features std,async --test sync --test assert_mem --test raw` | 28 passed |
| `RUSTFLAGS='-Dwarnings --cfg oneshot_test_delay' cargo +1.100.0 test --locked --features std,async --test sync` | 23 passed |
| `MIRIFLAGS='-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check' cargo +nightly-2026-11-15 miri test --locked --features std,async --test miri` | 16 passed |
| Workspace `python3 -m unittest vendor/tests/test_patch_contracts.py -v` | 5 passed, including the exact path identity and recorded file hashes |

The delay is an upstream test configuration that exercises the registration
race. It is absent from the deployed dependency. The standalone Miri tests
include send and disconnect with consuming, borrowed, and timed receivers.
They supplement the separate integrated application sanitizer results.

Cargo-deny applies registry advisories to registry sources. The workspace
path entry therefore needs a separate exact-origin advisory observation.
A disposable manifest with `oneshot = "=0.2.1"`, `std`, and default features
disabled resolved the published package with the archive checksum shown above.
`cargo +1.100.0 deny --locked --config deny.toml check advisories --deny warnings`
passed with the project policy and RustSec DB revision
`233155b4025b854c68a4b199c0e32d7fd53291a2`.
[Cargo-deny path behavior](https://github.com/EmbarkStudios/cargo-deny/blob/0.20.2/CHANGELOG.md).

The source tree retains the original MIT and Apache-2.0 texts. The Linux
packager currently stages the binaries and desktop entry without accompanying
license notices. Include the applicable copyright and license notices in a
distributed package. [MIT license](https://opensource.org/license/mit) and [Apache License 2.0 redistribution terms](https://www.apache.org/licenses/LICENSE-2.0).

[Exact standalone logs](standalone-logs.tar.gz).
