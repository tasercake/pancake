# macOS observer path fixture

The hosted macOS ARM64 job at `d6359e1` passed all 18 Miri tests and
later failed the CPU job fixture with `Illegal byte sequence (os error 92)`.
The original test propagated its attempted invalid-byte rename error before
running observer logic. Its function bytes match the pre-fix source at
`ae0b438`. [Hosted job](https://github.com/tasercake/focus-stacking/actions/runs/44277150143/job/134708072584),
[exact failure excerpt](hosted-failure-excerpt.txt), [original job log](hosted-job.log.gz)
and [test source hashes](test-source-shas.txt).

The test-only fix at `6d25836` runs a real CPU job with a Unicode-named
source, verifies the exported samples and checks both exact source keys and
identity transforms in the completion JSON. The test derives expected keys
using native filesystem canonicalization, matching production input
resolution and using the actual path spelling on the host. It then attempts
the invalid-byte rename. On filesystems that allow it, the test retains the
real byte-path run and checks that it creates an export and issues the
observer warning without inventing completion geometry. On macOS,
only the observed EILSEQ 92 permits the already verified Unicode oracle
to complete, after comparing the original source bytes. Any other rename
error fails the test.

The exact clean locked Linux checkout passed the targeted test, including
the real Unicode and invalid-byte paths. The scoped Clippy and formatting
checks passed. The macOS EILSEQ fallback still requires an Apple-host run.
No hosted rerun was requested for this check. [Local test log](local-test.log.gz),
[local identity](local-provenance.txt), [Clippy log](clippy.log.gz)
and [receipt](receipt.json).

From `pancake/`, reproduce the targeted test with the pinned Rust toolchain.

```sh
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
unset DISPLAY WAYLAND_DISPLAY DBUS_SESSION_BUS_ADDRESS AT_SPI_BUS_ADDRESS
cargo test --locked -p pancake --test cpu_job \
  observer_handles_non_unicode_paths_without_fabricating_geometry -- --exact
```

The capture used the existing temporary registry compiler cache and nice 5.
Clippy used `cargo clippy --locked -p pancake --test cpu_job -- -D warnings`
with the existing dependency-policy-generated configuration.
[Artifact checksums](SHA256SUMS).
