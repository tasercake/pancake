# Narrow upstream dependency patches

These directories contain **third-party code**, not Pancake application crates.
Their original licenses, source, normalized manifests and original manifests are
retained. `upstream-sources.json` pins each published crate archive by SHA-256,
records its upstream VCS metadata, and lists every modified upstream file with
before/after hashes. The additional `spin_on` integration test is first-party
regression code and forbids unsafe Rust.

| Published package | Patch | Scope |
| --- | --- | --- |
| `cubecl-core`, `cubecl-std`, `cubecl-cpp`, `cubecl-hip`, `cubecl-spirv` 0.11.4 | Rename dependency key `paste` to package `pastey =0.2.3` | Manifest only; existing `paste::paste!` imports and exported macro stay intact. Cargo and advisory tools see the actual `pastey` identity. |
| `spin_on` 0.1.1 | Replace `pin_utils::pin_mut!` with `core::pin::pin!`; remove `pin-utils` | Same future is pinned on stack before its first poll; no allocation. Declare Rust 1.68, when the standard macro became stable. Replace deprecated atomic spin hint with its standard `core::hint::spin_loop` successor. Update dependency-count documentation. |
| `cubecl-macros` 0.11.4 | Honor the existing `expand_only` flag for functions | Opt-in removal of the nonexecuted host DSL stub. Checked launch and device IR generation remain; unflagged functions retain their original behavior. Three compile contracts check these guarantees. |
| `rowan` 0.17.0 | Give private `GreenChild` enum explicit `repr(u32)` | Preserve its asserted 16-byte layout under both randomized layout seeds. Native regression tests exercise node/token clone, drop, offsets and payload access. Existing ARC implementation is unchanged; separate upstream Miri failures are recorded. |
| `oneshot` 0.2.1 | Acquire on the blocking receive's initial state load and receive-registration exchange | Replace four standalone acquire fences in `Receiver::recv` with synchronization recognized by ThreadSanitizer. The exchange continues to release the waker. |

No crate is relabelled as another API major. `cubecl-ir` has no production
`paste` dependency and is not patched. Published backend **dev** dependencies
on `paste` do not enter Pancake's production dependency graph; upstream backend
test graphs need separate review if those packages become vendored/test targets.

Third-party CubeCL and `spin_on` sources contain internal unsafe Rust. This patch
does not remove or certify those implementations. Pancake's first-party
`unsafe_code = "forbid"` policy stays in force. In particular, `spin_on`'s
original no-op raw waker is unchanged. It uses null-free static data, does not
allocate or dereference the raw pointer, and has inert wake/drop operations.
The executor still busy-polls; this is not a new scheduler for application jobs.

The `oneshot` patch retains the archived package's existing sender, raw
pointer, and deallocation code. The terminal state read now acquires the
message publication before any data access or free. The blocked receive path
still acquires the terminal state after it wakes. The crate's remaining
methods retain upstream behavior and remain within the third-party trust
boundary. The [P9 audit](../docs/archive/p9-tsan-audit/3dc9bcf/README.md) records the
original warning and the controlled comparison.

## Integration boundary

The application workspace excludes `vendor/*` from workspace membership, so
third-party source does not inherit first-party lint/coverage rules. It must
remain in Cargo's dependency/advisory/license audit. Root manifest overrides:

```toml
[patch.crates-io]
cubecl-core = { path = "vendor/cubecl-core-0.11.4" }
cubecl-macros = { path = "vendor/cubecl-macros-0.11.4" }
cubecl-std = { path = "vendor/cubecl-std-0.11.4" }
cubecl-cpp = { path = "vendor/cubecl-cpp-0.11.4" }
cubecl-hip = { path = "vendor/cubecl-hip-0.11.4" }
cubecl-spirv = { path = "vendor/cubecl-spirv-0.11.4" }
spin_on = { path = "vendor/spin_on-0.1.1" }
rowan = { path = "vendor/rowan-0.17.0" }
oneshot = { path = "vendor/oneshot-0.2.1" }
```

Keep the resolved lockfile. Do not edit registry cache files. Remove overrides
only after upstream releases replace both dependencies and the pinned stack
passes equivalent graph, macro-expansion and executor tests.

## Regression checks

```sh
python3 -m unittest discover -s vendor/tests -v
```

The dependency-identity test was run before changes: five CubeCL dependency
subtests and the `pin-utils` removal contract failed for the intended reason.
After the patch, all four contract tests pass. Upstream Rust checks and full
application integration results are recorded in
[the upstream patch review](../docs/archive/p0-upstream-patch-review.md).
