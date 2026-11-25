# P0 upstream dependency patch review

Date: 2026-11-17. Worktree: `p0-patches`, base `19b7f9e`.

## Decision

Replace two retired helper dependencies using the narrow source patches in
[`../vendor/README.md`](../../vendor/README.md). Keep Slint 1.19.0, CubeCL 0.11.4 and
wgpu 30. Preserve Wayland window titles, desktop settings and accessibility.
Do not claim full P0 passage from these dependency fixes.

| Notice | Disposition | Reason and remaining exposure |
| --- | --- | --- |
| `RUSTSEC-2024-0436`, `paste` 1.0.15 | Replace production edges with `pastey =0.2.3` | Same macro import name through Cargo's dependency rename. Five exact CubeCL manifests patched. No replacement runtime code or cross-major crate masquerade. |
| `RUSTSEC-2026-0441`, `pin-utils` 0.1.1 | Remove from patched `spin_on` | Rust's stack pinning macro provides the required pin guarantee. No allocation or new unsafe block. |
| `RUSTSEC-2026-0192`, `ttf-parser` 0.25.1 | Recommend exact temporary reviewed exception | Runtime Linux Wayland title-font parsing remains; do not describe it as compile-only or as a photograph decoder. See exposure below. |
| `RUSTSEC-2026-0370`, `RUSTSEC-2026-0371`, `RUSTSEC-2026-0372` | Recommend three exact temporary reviewed exceptions | `zvariant` 5, `zbus_names` 4 and `zvariant_derive` 5 are required by the present Slint/AT-SPI graph. Preserve runtime D-Bus settings/accessibility; migration is not a safe manifest-only change. |

All listed remaining notices are unmaintained-package notices, not evidence of
an identified exploitable defect. Exceptions must name exact IDs, versions,
owners, expiry and replacement conditions in the policy agent's ledger. This
report does not itself exempt any package or edit dependency policy.

## Linux title-font parser

Verified local source path:

```text
i-slint-backend-winit 1.19.0 (wayland)
  -> winit 0.30.13 (wayland-csd-adwaita)
  -> sctk-adwaita 0.10.1 (ab_glyph)
  -> ab_glyph -> owned_ttf_parser 0.25.1 -> ttf-parser 0.25.1
```

`sctk-adwaita/src/title/ab_glyph_renderer.rs` reads title-font preferences, runs
`fc-match` to locate a system font, memory-maps that file, and calls
`FontRef::try_from_slice`. If no usable system font exists, it parses bundled
`Cantarell-Regular.ttf`. It also parses variable-font weight and renders title
glyphs at runtime. User-installed fonts and font configuration therefore reach
this parser. Opening a photograph does not provide arbitrary font bytes through
this path; title text is input to glyph lookup, not to font-file parsing.

`winit` offers `wayland-csd-adwaita-crossfont` and
`wayland-csd-adwaita-notitle`. The latter removes title rendering. The former
changes renderer and introduces native font-library/build requirements; it needs
real Linux Wayland title, font, scale and packaging checks. Neither is a verified
drop-in fix here. The advisory recommends `skrifa`, which requires a real font
integration change rather than aliasing crate names or version numbers.

## Linux D-Bus migration

Verified manifests:

- `i-slint-backend-winit` 1.19.0 requests `zbus` 5.14 with `async-io` for Linux
  desktop settings. Its `xdg_desktop_settings.rs` reads session-bus appearance,
  font, cursor and double-click settings, and consumes change notifications.
- `accesskit_unix` 0.24.1 requests `zbus` 5.19 and `atspi` 0.29.
- `atspi` 0.29, `atspi-common` 0.13 and `atspi-proxies` 0.13 request `zbus` 5.5;
  `atspi-common` directly requests `zbus_names` 4.1.1 and `zvariant` 5.2.

Changing only Slint's direct zbus edge to 6 cannot remove the older family from
accessibility dependencies. A complete migration needs compatible proxy macros,
serialized D-Bus types, AT-SPI interfaces and runtime session-bus tests across
these packages. No zbus 5-to-6 API compatibility claim or untested port is made.
Disabling accessibility or Wayland would change required behavior and is not
used to clear the audit.

## Validation record

| Check | Result |
| --- | --- |
| Pre-patch `python3 -m unittest discover -s pancake/vendor/tests -v` | Expected red: five CubeCL macro-package identity failures and one `pin-utils` dependency failure. Provenance identity check passed. |
| Post-patch contract tests | Four tests passed, including package identity, retired dependency removal, original package provenance and reviewed changed-file hashes. |
| Upstream `spin_on` unit/doc tests plus immovable-future regression | Passed on Rust 1.100.0: two upstream unit tests, one original doctest, one added address-stability/drop/waker regression. Same four passed under Miri with strict provenance and symbolic alignment checks. |
| Existing upstream CubeCL unit tests | Passed on Rust 1.100.0: core 12, C++ generator 20, standard library 50; SPIR-V crate built with zero unit tests. These are code-generation tests, not physical CUDA/Metal/AMD execution. HIP build/device execution was not tested. |
| Existing upstream CubeCL macro integration tests | Both trybuild groups passed: two rejected programs and three accepted programs. No expected-error snapshots changed. |
| Independent validation lock | `pastey` 0.2.3 present; no package named `paste` or `pin-utils`. Lock SHA-256: `a638b910684a73a08a3a32d130a79d0a6a213f42cf211f893ce12f61a8740399`. This is not the integrated application lock. |
| Exact archive/source comparison | All 390 upstream files checked against SHA-pinned original archives. Only seven files differ: five CubeCL manifests, `spin_on` manifest and `spin_on/src/lib.rs`. Added tests/provenance are separate. |
| `bash scripts/check.sh all` | Exit 0 on dependency-free scaffold, but **not a clean integrated pass**: cargo-machete reported six vendor metadata/workspace errors and returned success. Other scaffold gates passed. Kernel agent owns root workspace exclusion; policy agent owns fail-closed dependency checking. |

To keep root manifests under the kernel agent's ownership, exact copies of the
six vendored packages were placed in ignored
`artifacts/p0-upstream-validation/`, outside the application workspace. A test-only
workspace lists those packages as members and applies the same path overrides.
No copied package source or manifest was modified for this harness. Command:

```sh
cargo +1.100.0 test --locked --manifest-path artifacts/p0-upstream-validation/Cargo.toml -p spin_on
cargo +1.100.0 test --locked --manifest-path artifacts/p0-upstream-validation/Cargo.toml -p cubecl-core -p cubecl-std -p cubecl-cpp -p cubecl-spirv --lib -j 2
cargo +1.100.0 test --locked --manifest-path artifacts/p0-upstream-validation/Cargo.toml -p cubecl-core --test mod -j 2
CARGO_BUILD_JOBS=2 MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check" cargo +nightly-2026-11-15 miri test --locked --manifest-path artifacts/p0-upstream-validation/Cargo.toml -p spin_on
```

Saved local logs: `artifacts/p0-upstream-validation/core-macro-regressions.log`
and `artifacts/p0-upstream-validation/spin-on-miri.log`. The application manifest
and lockfile were not edited by this workstream. Their actual resolved stack,
first-party gates and device proofs must be checked after integration.

The original upstream README files and one C++ backend Rust string retain seven
trailing-whitespace lines from their exact published archives. The patch does
not reformat unrelated third-party source. This explains `git diff --check`
findings for newly vendored files; changed helper lines and first-party additions
have no such errors.

Native Linux/Windows, real Wayland titles, AT-SPI/session-bus integration, CPU JIT,
GPU kernel execution and performance qualification are separate checks. They
have not been executed by this patch workstream.
