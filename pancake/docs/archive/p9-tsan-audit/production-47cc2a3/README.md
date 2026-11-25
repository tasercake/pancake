# P9 Linux CPU sanitizer replay

The Pancake CPU sanitizer gates both exited 0 at exact source revision
`47cc2a3bfe8272ff7c14228f1b1dca77c2aa5dcd`, lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Each ran 98 tests across 19 test executables with zero failures, ignored
tests, filtered tests, or sanitizer reports. The production CPU routes
include CubeCL LLVM execution in library and CLI subprocess tests. The
GUI target runs its full Rust suite, including the rendered Slint
comparison workspace.

## Source and observed results

| Replay | ThreadSanitizer | AddressSanitizer |
| --- | --- | --- |
| `a270bd5` reviewed `oneshot` synchronization | 98 tests, exit 0 | 98 test assertions passed, then exit 1 and 970,329 leaked bytes across 36,595 allocations |
| `7a95335` close the visible comparison workspace | 98 tests, exit 0 | 98 test assertions passed, then exit 1 and 529,691 leaked bytes across 11,296 allocations |
| `47cc2a3` release the offscreen platform window | 98 tests, exit 0 | 98 tests, exit 0 |

The original CPU CLI ThreadSanitizer failure and isolated standalone
before-and-after probe are preserved in the [earlier audit](../3dc9bcf/README.md).
The production graph pins the reviewed `oneshot 0.2.1` source, changing
only blocking receive synchronization in the original source distribution.
The graph guard first failed against the registry copy and passed after
Cargo resolved the reviewed local copy. Source inventory, preserving both
upstream licenses, and the separate registry RustSec observation are
recorded in the [independent review](../a270bd5-review/README.md).

The original AddressSanitizer GUI test executable
(`1cb876580ee0214ca9a92e514676544c5a63406f`) reproduced its leak in one
test, `tests::comparison_clips_source_to_left_of_split`, after that test's
assertions passed. The compiler-only `embedded_workspace_compiles` test
exited 0 when run separately in the same executable. The visible window
holds the component while shown. Closing it reduced the leak and made
the component's weak handle expire. The remaining report symbolized to
the fonts owned during the rendered workspace. The test platform also
held a strong window reference, and the window held the Slint context
that owned the platform. The fixture's weak platform reference breaks
that cycle. All pointer events, rendered pixels, image inputs, and
expected comparison behavior are preserved. The test verifies that
both the component and the adapter are released.

## Reproduction

Use exact source and the `rust-src` component of `nightly-2026-11-15`.
On Ubuntu 24.04.5 x86-64, kernel `6.8.0-147-generic`, an AMD Ryzen 9
7950X3D provided four distinct physical cores through logical CPU IDs
0, 1, 2, and 3. Use a comparable set of four physical cores on another
host. Run from the `pancake` workspace directory, with the checked-in
lock and no sanitizer option overrides.

```sh
for sanitizer in address thread; do
  export CARGO_TARGET_DIR="$PWD/target/host-sanitizer-$sanitizer"
  env -u TSAN_OPTIONS -u ASAN_OPTIONS -u LSAN_OPTIONS \
    CARGO_BUILD_JOBS=3 CARGO_INCREMENTAL=0 \
    timeout --signal=TERM --kill-after=10s 1200s \
    taskset --cpu-list 0,1,2,3 bash scripts/check.sh sanitizer "$sanitizer" || exit
done
```

The script regenerates the 16-target CPU integration inventory and runs
each package's unit tests with `-Dwarnings`, `-Zsanitizer`, and
`-Zbuild-std` on the pinned nightly. Tests use their checked-in synthetic
inputs. Capture scripts record the source and lock hashes, CPU topology,
toolchain, test counts, and the built CLI binary hashes and ELF build IDs.
They use a registry-only compiler cache and clean each generated target
tree after collecting evidence. No physical GPU execution occurred.
