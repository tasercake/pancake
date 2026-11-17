# Dependency review and enforcement

The user authorized removal of license gating and narrowly reviewed exceptions
for the required Slint/CubeCL/wgpu graph. Global duplicate, advisory and source
checks remain strict. This changes dependency policy; it does not qualify runtime
correctness, platform support, image quality or performance.

## Required entry points

Run `bash scripts/check.sh fast` for the first-party compiler/lint checks and
`bash scripts/check.sh deps` for the complete dependency audit. Both generate
policy from locked, all-feature Cargo metadata and the reviewed
[`dependency-policy.json`](../dependency-policy.json) ledger. The existing native
CI platform jobs run `fast`; the existing assurance job runs `deps`. Python 3.11
or newer is required; missing Python or tools fail the command.

Generated files stay under `target/dependency-policy/`:

| File | Purpose |
|---|---|
| `metadata.json` | Actual locked graph, including optional/platform/build paths. |
| `deny-effective.toml` | Strict base policy plus active, exact-version duplicate skips and reviewed advisory IDs. |
| `clippy/clippy.toml` | Existing Clippy settings plus reviewed duplicate names, after exact-version and parent checks. |
| `licenses.json` | Informational package name/version/license declarations, including missing declarations. No allowlist or legal approval assertion. |

The dependency command explicitly runs `cargo deny check advisories bans sources
--deny warnings`, with pinned cargo-deny 0.20.2. Its `licenses` check is not run.
Deleting `[licenses]` alone would leave its default license gate enabled, so the
explicit check selection is required. License inventory does not reject unknown,
missing or differently licensed packages.

## Review limits

Each duplicate review names one exact package version, concrete reason, immediate
resolved parents and their exact versions/declared requirements, review date and
expiry. The maximum lifetime is 90 days. A review fails after expiry even when its
package is no longer active, so obsolete entries must be removed deliberately.
New parent versions, new parent requirements, and added parent paths fail before
Clippy or cargo-deny can consume the generated exception. Dependency aliases and
library target names are resolved when matching parent requirements.

Cargo-deny skips apply only to the reviewed older versions diagnosed in its
supported desktop target union. Absent versions do not generate unused skips.
No package tree is skipped. The global `multiple-versions = "deny"`, wildcard
check, unmaintained/unsound checks, and unknown-registry/Git checks remain enabled.

Clippy's `multiple_crate_versions` also reports packages present only in dormant
optional, platform or build paths. Its `allowed-duplicate-crates` setting accepts
names rather than exact versions. Therefore the helper checks **every active
version and parent path** for each allowed name before generating Clippy config.
Unreviewed versions fail; global lint severity remains unchanged. The source
Clippy config cannot contain hand-added duplicate allowances.

`wgpu`, `wgpu-*`, `naga`, `raw-window-handle`, `cubecl` and `cubecl-*` cannot receive
duplicate exceptions. Multiple versions in any protected family fail even when
not present in cargo-deny's target-filtered graph. This preserves one version per
shared GPU/window crate; native runtime/interop tests are still required.

## Temporary maintenance notices

Only the four reviewed **unmaintained** IDs below may receive exceptions. Other
advisories, including any new security issue for these crates, remain enforced.
Cargo-deny ignores advisory IDs globally, so the helper additionally binds each ID
to exact active package versions and their reviewed immediate parent paths.
These exceptions expire on **2026-12-17**; duplicate reviews expire on
**2027-02-15**. Every exception has a removal condition or concrete dependency
reason in the ledger.

| Advisory and package | Actual exposure and reason for temporary exception |
|---|---|
| RUSTSEC-2026-0192, `ttf-parser` 0.25.1 | Wayland titlebar text loads/parses system fonts selected by `fc-match`, with embedded Cantarell fallback. Runtime font bytes remain an input exposure. Replacing this titlebar stack needs Linux font/fallback tests and native dependency review. |
| RUSTSEC-2026-0370, `zvariant` 5.15.1 | Runtime session D-Bus variant decoding for Slint settings and AT-SPI accessibility. Migration requires coordinated zbus/Slint/AccessKit/AT-SPI API changes. |
| RUSTSEC-2026-0371, `zbus_names` 4.3.5 | Runtime session-bus name parsing for settings/accessibility; same coordinated migration is required. |
| RUSTSEC-2026-0372, `zvariant_derive` 5.15.1 | Host macro generates wire serialization code paired with zvariant 5. Runtime generated code handles session-bus inputs; upgrade with the full stack. |

The [upstream patch review](p0-upstream-patch-review.md) documents exposure and
removal alternatives. No exception exists for `paste` or `pin-utils`; reviewed
source patches remove those dependencies. No titlebar/accessibility feature is
disabled to remove a notice.

## Unused dependencies

`check_unused_dependencies.py` passes each actual first-party workspace manifest
as an exact file to pinned cargo-machete 0.9.2. This checks all first-party members
without recursively treating published vendor sources as application crates.
Vendored dependencies remain in cargo-deny's audited graph.

Guardrail test crates are generated from the real workspace lint rules and
profiles. They have no production dependencies, patches, feature graph or custom
targets. Their own lockfile is generated offline. This keeps negative compiler,
coverage and mutation checks independent of vendor path availability while
preserving the policy each fixture is meant to test.

Cargo-machete 0.9.2 can print `error when handling ...` for a metadata error and
still return success. The wrapper treats that diagnostic as failure. Real-tool
regression tests cover malformed first-party metadata and a malformed nested
vendor manifest; metadata errors cannot count as a clean gate pass. See the
[pinned upstream implementation](https://github.com/bnjbvr/cargo-machete/blob/v0.9.2/src/main.rs#L164-L187).

## Recorded local validation

On 2026-11-17 the helper reviewed the final P0 kernel metadata recorded by SHA-256
in the ledger: 787 packages, 58 duplicate names and 123 exact package-version
reviews for Clippy; 32 exact older-version cargo-deny skips across 29 diagnosed
desktop duplicate groups. Four maintenance exceptions are active. All protected
families are single-version. Direct cargo-deny 0.20.2 evaluation of this metadata
and generated config passed advisories, bans and sources with warnings denied.
The 19 focused Python tests passed. `bash scripts/check.sh all` passed on the
dependency-free policy worktree, including ten real-tool guardrail outcomes.
That worktree remains the exact empty application scaffold; its Rust coverage
exemption does not demonstrate coverage of the forthcoming kernel implementation.
