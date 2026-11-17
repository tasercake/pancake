# P0 runtime, safe-launch, and assurance audit

Date: 2026-11-17. Worktree: `codex/p0-runtime`, based on `8fcbc52`.
Execution host: macOS 26.2 / 25C56, ARM64. Scope: P0.03-P0.06.

## Result

**Three bounded dependency candidates fail current dependency policy.** The
minimal compatible-patch candidate still has **30 duplicate groups and six
unmaintained advisories**. This conclusion is not based only on the original
overbroad viewer dependency graph.
No application dependency, unsafe statement, advisory ignore, duplicate-version
skip, coverage exception, or runtime code was added to Pancake. No custom safe
kernel ran, so P0.04 and P0.06 are not passed. The successful empty-scaffold
checks below are not runtime or kernel qualification.

The dependency-only resolution fixture has no executable behavior. It is under
ignored `pancake/target/p0-runtime-audit/`; it does not bypass a failing gate to
produce a runtime pass. The tracked [evidence](p0-runtime-evidence.json) records
its exact manifest, lock reconstruction, dependency paths, diagnostics, and
artifact hashes.

| Gate | Status | Evidence |
|---|---|---|
| P0.03 | **Blocked for tested candidates** | All three `cargo-deny` runs exit 7. Minimal/current compatible patch graph retains 30 duplicate groups and six unmaintained advisories, separate from expected license review. |
| P0.04 | Not run; API route identified | Safe `TensorHandle::empty` / `into_arg()` and checked `#[cube(launch)]` exist in installed source. No compilation or completed custom-kernel execution was claimed. |
| P0.05 | Blocked / unqualified | Existing all-features Miri tests cannot be assumed to execute native LLVM JIT/GPU integration. A real verification split has not been designed or proven. Empty-scaffold results do not satisfy this gate. |
| P0.06 | Not run | No native CPU/wgpu or other-runtime kernel battery was executed under a policy-compliant adopted dependency graph. |

## Candidate and exact resolution

The starting lock is the viewer experiment at commit `0c83870`, path
`experiments/slint-cubecl-viewer/Cargo.lock`. CPU and stdlib are added to CubeCL
features to investigate safe tensor-backed CPU/wgpu launches. The only lock
change is adding `cubecl-std` to CubeCL's dependency list; package versions are
unchanged. Apply [the lock patch](p0-runtime-lock.patch) to reconstruct it.

### Bounded follow-up: minimal features and compatible released patches

| Attempt | Required features and resolution | Result |
|---|---|---|
| A | Original viewer-derived manifest below, adding CubeCL CPU/stdlib. | 41 duplicate groups; six unmaintained notices; one missing-license error; 653 allowlist rejection diagnostics. |
| B | Only two direct dependencies, defaults disabled: Slint 1.19.0 with `std`, `backend-winit`, `renderer-femtovg-wgpu`, `renderer-software`, `unstable-wgpu-30`, `compat-1-18`, `accessibility`; CubeCL 0.11.3 with `std`, `stdlib`, `cpu`, `wgpu`. | 30 duplicate groups; six unmaintained notices; missing-license error removed; 561 allowlist rejection diagnostics. |
| C | B's required features, refreshed by Cargo/Rust 1.100.0 within `slint ~1.19.0` and `cubecl ~0.11.0`. Resolves Slint 1.19.0, CubeCL/cubecl-wgpu 0.11.4, one wgpu 30.0.2. | Same 30 duplicate groups and six notices. No prerelease, fork, missing accessibility, or platform-feature deletion. |

B removes avoidable direct codec/utility and direct cubecl-wgpu default-feature
activation. It retains native CPU, stdlib safe tensor APIs, all winit platform
paths, accessibility, portable GPU rendering, and software presentation. Thus it
does not obtain a better result by dropping required capabilities. Fresh C
resolution confirms that compatible released patches do not remove the decisive
conflicts. Cargo metadata for B and the patch refresh/metadata for C were verified
under Rust 1.100.0. Metadata success is not compilation or kernel execution.

Exact manifests, hashes, diagnostic counts, and all remaining duplicate groups
are in `candidate_attempts` in the evidence JSON. Reconstruct B/C from the source
lock using [B's patch](p0-runtime-minimal-lock.patch) or
[C's patch](p0-runtime-refreshed-lock.patch). Later tables retain A's initial
diagnostics for history; C is the final retained result.

| Package | Resolved version | Relevant declaration |
|---|---|---|
| `slint` | 1.19.0 | winit, FemtoVG-wgpu, `unstable-wgpu-30`, accessibility; default features disabled. |
| `cubecl` | 0.11.3 | `std`, `stdlib`, `cpu`, `wgpu`; default features disabled. |
| `cubecl-core`, `cubecl-std`, `cubecl-cpu` | 0.11.4 | Resolved through CubeCL's compatible dependency ranges. |
| `cubecl-wgpu` | 0.11.3 | Same version used by the experiment. |
| `wgpu` | 30.0.2 | **One version**, shared by Slint and CubeCL. wgpu-major duplication is not the observed problem. |
| `cubecl-llvm` / `llvm-sys` | 0.11.4 / 231.0.2 | Native CPU JIT compilation. |
| `tracel-llvm-bundler` | 23.1.0-4 | Downloads/verifies a prebuilt LLVM bundle and links it. |

Cargo metadata contains 792 packages before platform filtering. That is not the
count of packages compiled on this Mac. `cargo-deny` used Pancake's unchanged
four-target graph policy: ARM64/x86-64 macOS, x86-64 Linux, x86-64 Windows.
The fixture retains the viewer's file-codec and small utility dependencies, so
this is an audited candidate graph, not a proof that every minimal feature set
has exactly these counts.

## Independent policy blockers

| Kind | Observed result | Consequence |
|---|---|---|
| Duplicate versions | 41 rejected crate-name groups | Cannot pass `multiple-versions = deny` with empty skip lists. Upstream semver requirements need alignment or a different reviewed graph. |
| Unmaintained dependencies | Six rejected advisories below | Cannot pass `unmaintained = all` with empty advisory ignores. Updating the allowlist cannot solve these. |
| Missing license metadata | `cfg_block` 0.1.1 has no manifest license expression | Needs actual source/license clarification or replacement, not a blanket unknown-license allowance. |
| License review | 653 rejection diagnostics under GPL-only allowlist | Expected during adoption of new dependencies. Reviewed concrete SPDX additions are permitted by `deny.toml`; this is not the sole blocker or a claim of legal incompatibility. |

The six advisory paths include:

| Advisory | Dependency | Observed path |
|---|---|---|
| RUSTSEC-2024-0436 | `paste` 1.0.15 | CubeCL -> cubecl-core -> paste. |
| RUSTSEC-2026-0441 | `pin-utils` 0.1.1 | Slint -> slint-macros -> spin_on -> pin-utils. |
| RUSTSEC-2026-0192 | `ttf-parser` 0.25.1 | Slint/winit -> sctk-adwaita -> ab_glyph -> owned_ttf_parser. |
| RUSTSEC-2026-0371 | `zbus_names` 4.3.5 | Slint winit backend -> zbus -> zbus_names. |
| RUSTSEC-2026-0370 | `zvariant` 5.15.1 | Slint winit backend -> zbus -> zvariant. |
| RUSTSEC-2026-0372 | `zvariant_derive` 5.15.1 | Slint winit backend -> zbus -> zvariant -> derive. |

The advisory database was fetched/read by the actual `cargo-deny` check; these
are its current diagnostics, not remembered vulnerability claims. They are
unmaintained notices, not assertions that each package has an exploitable bug.

Representative incompatible dependency requirements are visible in installed
source. `i-slint-renderer-femtovg` 1.19.0 requires `glow` 0.18, while wgpu-hal
30.0.2's GLES path requires `glow` 0.17. Slint's wgpu dependency enables default
features, so adding `default-features = false` to a separate application wgpu
dependency cannot subtract those features. Other duplicate groups include
multiple `hashbrown`, `getrandom`, `objc2`, `syn`, and `windows-sys` versions.
The complete rejected groups are in the evidence JSON. Two concrete retained
constraints explain why ordinary application pins cannot eliminate every
duplicate: Slint/FemtoVG requires `glow ^0.18`, while wgpu-hal requires
`glow ^0.17`; on Apple, winit requires `objc2 ^0.5.2`, while wgpu-hal requires
`objc2 ^0.6.3`. Both pairs have disjoint compatibility ranges.

Unmaintained notices also persist beyond removable codecs/default features.
CubeCL-core 0.11.4 directly requires `paste ^1`. Slint's Linux winit backend
requires `zbus ^5.14.0`; zbus 5.19.0 requires `zvariant ^5.14.0` and
`zbus_names ^4.3.4`. A migration to the maintained zbus 6 API family cannot be
selected within those declared major ranges. It requires upstream source/API
alignment or a different compatible upstream release, not an advisory ignore.

The current direct declarations include `MIT OR Apache-2.0` for CubeCL/wgpu and
their runtime crates, and Slint's choice of `GPL-3.0-only` or its alternative
license references. These actual declarations need package-specific review
when adopting a viable graph. No allowlist addition was committed because no
application dependency was adopted and the independent duplicate/advisory gates
already reject this graph. Disabling accessibility or a required platform merely
to hide a dependency failure would not prove the required native UI contract.

## Safe-launch source audit

The installed sources expose a possible application-unsafe-free route:

1. `cubecl-std-0.11.4/src/tensor/handle.rs:71`: safe
   `TensorHandle::empty(client, shape, storage)` allocates a tensor.
2. The same file at line 114 exposes safe `into_arg()`, which routes through
   the dependency's own tensor binding boundary.
3. `cubecl-macros-0.11.4/src/parse/kernel.rs:43` distinguishes checked launches
   as safe and unchecked launches as unsafe.

This does **not** prove that arbitrary metadata passed through safe constructors
is correct. Pancake still needs checked shape products, capacity/dtype/stride
invariants, aliasing rules, lifetimes, tail invocations, and resource ownership.
It must compile real generated kernel code under existing lints. In particular,
the macro generates some `allow` attributes internally; whether emitted spans
interact with Pancake's Clippy policy must be tested, not inferred.

`TensorHandle::binding()` internally calls an unsafe upstream constructor, and
`TensorHandle::zeros()` uses an upstream unchecked launch. Dependency-internal
unsafe code is not the same as adding unsafe application code. No policy change
to `unsafe_code = forbid` is proposed here, and the experiment's application-side
`BufferArg::from_raw_parts` statements remain inadmissible.

## Native CPU and all-feature requirements

Installed `cubecl-cpu` 0.11.4 README describes real in-process LLVM JIT and a
worker pool with SIMD. Native CPU is not wgpu's software adapter. Its default
worker stack is 64 MiB, floored at 16 MiB when configured; these are stack
reservations, not a measured resident-memory allocation per worker. Include
this in resource qualification rather than treating CPU fallback as free.

`cubecl-llvm` 0.11.4 build script links LLVM through `tracel-llvm-bundler`.
Bundler 23.1.0-4 source names six platform/architecture artifacts: Linux,
macOS, Windows, each x86-64 and ARM64. It caches downloads under a user cache
directory and verifies/extracts bundles. Source support is not evidence that a
clean end-user package builds, loads, or executes on those six combinations.

Native feature names in CubeCL 0.11.3 are significant: `cpu`, `cuda`, `hip`,
`metal-native`, and `wgpu`; `metal` selects `wgpu-msl`, not the separate native
Metal runtime. Qualify exact feature names rather than interpreting product
labels as runtime identity.

The existing `check.sh nightly` runs Miri over workspace/all-feature tests. A
test that launches a native LLVM JIT/GPU cannot be assumed to work under Miri.
Separate checked host-model tests from native execution evidence, but do not
add ignored tests, broad exclusions, or change all-feature/Miri commands merely
to make them pass. The final coverage denominator must still include all
application-owned launch/error/resource handling. This design remains unresolved.

## Reproduction and verification

Reconstruct the dependency-only fixture from `candidate_manifest` in the JSON
and a documentation-only `src/lib.rs`. Restore the source lock from commit
`0c83870`, then apply the checked-in patch. Do not treat this fixture as adopted
application code or a working kernel implementation.

```sh
cargo metadata --manifest-path pancake/target/p0-runtime-audit/Cargo.toml \
  --format-version 1 --locked > pancake/target/p0-runtime-audit/metadata.json

pancake/target/tools/bin/cargo-deny \
  --manifest-path pancake/target/p0-runtime-audit/Cargo.toml \
  --config pancake/deny.toml --format json check --deny warnings \
  > pancake/target/p0-runtime-audit/deny.jsonl \
  2> pancake/target/p0-runtime-audit/deny.log
```

Expected audit outcome: nonzero dependency-policy failure, not a runtime pass.
Raw metadata/diagnostics are ignored artifacts in this worktree. Their hashes
and compact diagnostic records are retained in the JSON. No suite file outside
adapters was changed. No runtime graph was installed in Pancake's Cargo.toml.

`bash scripts/check.sh all` passed locally on the unchanged documentation-only
scaffold, including guardrail rejection fixtures. Application test count remains
zero; source/branch coverage are not applicable to this exact empty scaffold.
Native Linux/Windows builds, Linux sanitizers, custom kernel compilation, and
CPU/GPU runtime execution were not run. These limits are separate from the
candidate dependency-policy failure.

## Existing remote hardware access addendum

A short follow-up used the previously authorized SSH hostname `krishna-desktop`
and independently verified current access. One BatchMode connection with an
eight-second connection timeout returned exit 0:

```text
krishna-desktop
Linux 6.8.0-147-generic x86_64
NVIDIA GeForce RTX 3090, 580.178.04, 24576 MiB
```

Next decision: choose a dependency version/feature/upstream-fix strategy that
passes the unchanged duplicate/advisory policies, and review concrete licenses.
Then execute meaningful test-first safe CPU/wgpu kernels and complete the real
assurance design. No direct merge of a tested candidate graph is recommended.
No custom kernel was compiled or executed in any of the three attempts. Under
the bounded no-fork/no-ignore/no-feature-removal constraint, continuation into
adopted runtime code would leave a known failing dependency gate unresolved.
