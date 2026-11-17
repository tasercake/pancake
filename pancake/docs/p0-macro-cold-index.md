# Offline macro fixture resolution

CI run 44096988927 failed before any of the three macro compile contracts ran.
The fixtures copied the root lockfile but supplied only the `cubecl-core` and
`cubecl-macros` patches. Cargo still resolves optional packages from the locked
graph, including `cubecl-hip`. A warm local registry hid the missing patch.

The fixtures now inherit every root `[patch.crates-io]` entry, resolving local
paths to absolute paths. They retain `cargo check --offline`, start each fixture
from the root lockfile, and keep all three compiler assertions. Root manifests, lockfile,
and Rust application source are unchanged.

Each test class creates a temporary Cargo home, copies the registry index without
the `cubecl-hip` entry, and shares existing registry cache/source directories by
symlink. The global index is not changed. Offline Cargo does not download packages
or perform [automatic cache garbage collection](https://doc.rust-lang.org/cargo/reference/config.html#cacheauto-clean-frequency). Temporary indexes are removed by
the test-class cleanup.

| Check | Observed result |
| --- | --- |
| Cold index with original two patches | All three tests failed: ``error: no matching package named `cubecl-hip` found``. |
| Cold index with all eight root patches | All three real compiler contracts passed. The negative contract reached the expected `E0423` compiler error. |
| Complete vendor test suite | All seven tests passed. |
| Fixture manifest inspection | All three contain exactly the root patch set with absolute local paths. |

Commands, logs, source hashes, and cleanup checks are in
[the evidence receipt](p0-macro-cold-index-evidence/receipt.json). These local
checks establish the cold-index regression; the parent integration checkout owns
the complete assurance run and subsequent CI result.
