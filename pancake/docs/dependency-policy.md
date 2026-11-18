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

Only the five reviewed **unmaintained** IDs below may receive exceptions when
the configured target graph requires them. Other
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
| RUSTSEC-2026-0543, `chrono` 0.4.45 | Slint runtime date/calendar parsing and formatting plus local clock/timezone access. Full metadata also has exact dormant Turso SQL timestamp and VFS clock paths. Replace through a reviewed compatible date/time migration. |

The [upstream patch review](p0-upstream-patch-review.md) documents exposure and
removal alternatives. No exception exists for `paste` or `pin-utils`; reviewed
source patches remove those dependencies. No titlebar/accessibility feature is
disabled to remove a notice.

## Chrono maintenance review, 2026-11-17

RUSTSEC-2026-0543 is dated 2026-09-14 and explicitly classified
`informational = "unmaintained"`, with no patched version listed. The
[upstream maintainer's wind-down notice](https://github.com/chronotope/chrono/issues/1768)
supports this maintenance classification. This review does not downgrade a
security vulnerability or accept any other advisory ID for Chrono.

The exact review covers `chrono` 0.4.45 and three immediate metadata parents:
`i-slint-core` 1.19.0 (`^0.4`), `turso_core` 0.8.3 (`^0.4.42`) and `turso_ext`
0.8.3 (`^0.4.42`). Current normal-feature `cargo tree --invert chrono` reaches it
through Slint only. The helper also checks dormant full-metadata parent paths,
so those Turso paths were reviewed explicitly instead of omitted from the ledger.

In Slint's `date_time.rs`, `NaiveDate` handles UI date/format strings, calendar
month lengths and weekday offsets. `Local::now` reads the system clock/timezone
when `std` is enabled. Dormant Turso code uses Chrono for SQL timestamp/RFC3339
parsing and arithmetic, local-time conversions, clock/pragma date values and VFS
current-time formatting. These are runtime text/database/OS-time exposures; no
compile-only or image/RAW parsing claim is made. Static source review does not
claim that the current application exercised these APIs.

The ledger records the advisory hash and seven reviewed source-file hashes.
The review expires on 2026-12-17 and remains bound to exact package versions,
parent versions and declared requirements. No date/time dependency or upstream
source was replaced here. A maintained replacement still needs compatible
parsing/formatting, invalid-input, calendar and local-time behavior checks.

### P1 headless dependency graph

The workspace resolves the core and CLI without Slint or its window toolkits.
The isolated P0 UI qualification preserves exact
Slint and Slint Interpreter pins for the separate GUI consumer. The core retains the exact CubeCL and wgpu versions and selected backend features,
including wgpu-hal's dynamic EGL/Wayland loader support. The graph regression
checks both headless packages across supported targets. It accepts the low-level
`wayland-sys` loader and rejects the Slint, winit, X11 and Wayland client graphs.

Removing Slint makes the Chrono maintenance ignore and five cargo-deny skips
unneeded for the headless target union. The affected skips are `getrandom`
0.3.4, `glow` 0.17.0, `linux-raw-sys` 0.4.15, `rustix` 0.38.44 and
`windows-sys` 0.52.0. The audit still checks dormant, Clippy-relevant version
and parent edges. The previous Chrono review remains in the P0 ledger at
`34dd609`; reuse requires an active target graph with unchanged reviewed
versions and relationships before the original expiry.

Bounded image diagnostics activate a new Chrono consumer in Rawler. The exact
maintenance review records Rawler 0.8.0's date and metadata calls and retains the
original expiry. This does not restore the Slint paths to the headless graph.
The codec graph also adds reviewed container, parser and compression generations
to the exact duplicate ledger. The pinned Cargo-deny target union finishes with
advisories, bans and sources checked with warnings denied.

Clap 4.6.8 adds a reviewed compile-time parent to the existing `syn` 3.0.10
duplicate entry. Clap's derive proc macro parses Rust input and generates the
CLI parser. All `syn` ASTs stay within their own proc-macro expansions. The
duplicate exception keeps its original expiry.

## P7 native desktop graph

The locked GUI crate adds Slint 1.19 with its safe interpreter, GTK 3.24 file
panels and ATK accessibility on Linux, and rfd 0.18 native panels on macOS
and Windows. The source
patches for Rowan and spin_on remain active in the GUI graph. Detecting both
toolkits in the same target or reaching either from the core or CLI fails
the resolved-graph regression. Linux hosted builds provision `libgtk-3-dev`
and check the GTK 3.24 and ATK 2.34 pkg-config minimums before compiling.

The desktop target union adds exact parent paths for existing duplicate
versions, including GLib/Cairo bitflags, GTK/GLib proc-macro syntax, and rfd
AppKit/Windows bindings. Two exact Uniform Type Identifiers versions are
reviewed for Clippy's all-target inventory. The UIKit version appears only
in dormant iOS metadata. rfd uses the AppKit version for local filename
filters. The cargo-deny target union does not skip that pair. Existing
Slint-induced duplicate skips are restored only where the desktop target union
reports the exact older version again.

The locked GTK3 graph produces no additional advisory IDs in pinned Cargo-deny.
[RustSec's GTK3 advisory](https://rustsec.org/advisories/RUSTSEC-2024-0415.html)
records its withdrawal after upstream development resumed. The five existing
maintenance reviews retain their original expiry. The Chrono review binds
both the retained Rawler and reactivated Slint paths to their original
reviewed versions.

The Linux-only dialog handoff adds exact GDK X11 bindings 0.19.0,
their matching sys crate, and X11 2.21.0. The GUI also declares x11rb 0.13.2
directly, using the version already resolved for winit. GDK X11 associates
the GTK panel with its actual X11 owner window. x11rb reads the current
window-relative pointer position when a real button event arrives after a
panel closes. The development package already supplies the native GDK X11
and X11 edges. The graph regression and Linux pkg-config probe check both
explicitly. The configured Cargo-deny audit adds no duplicate or advisory
exception for these packages.

Canonical Miri still runs every first-party library test on Darwin. GTK,
rfd and Slint are isolated in the GUI binary, whose native checks stay in
the workspace CPU, coverage, layout and sanitizer selectors. The reviewed
source inventory now pins the embedded Slint declaration with the Rust and
Cargo inputs. Mutation generation and the associated test suite now span
the workspace. The locked draft listed 2,261 mutations with this selection,
including six CLI mutations and 135 GUI mutations. That is an inventory,
with actual outcomes recorded by the applicable mutation gate.

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
desktop duplicate groups. That initial audit had four maintenance exceptions. All protected
families are single-version. Direct cargo-deny 0.20.2 evaluation of this metadata
and generated config passed advisories, bans and sources with warnings denied.
The 19 focused Python tests passed. `bash scripts/check.sh all` passed on the
dependency-free policy worktree, including ten real-tool guardrail outcomes.
That worktree remains the exact empty application scaffold; its Rust coverage
exemption does not demonstrate coverage of the forthcoming kernel implementation.

For the separate Chrono review, the focused regression was observed failing
before the finite approved-ID set changed. It then passed, including rejection of
an additional unreviewed Chrono version. All 20 policy/unused-dependency/fixture
Python tests passed with the pinned tool directory on `PATH`. `bash scripts/check.sh deps` then passed: 19 policy/unused-dependency tests,
seven vendor contracts, advisories/bans/sources with warnings denied, and the
first-party unused-dependency check. No advisory other than the five reviewed
maintenance IDs is ignored. Full graph build and `check.sh all`
remain the responsibility of the already-compiled integration worktree after
this maintenance commit is cherry-picked; they are not claimed for this worktree.
