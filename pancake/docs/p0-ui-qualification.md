# Minimal P0 native UI, interop and worker qualification

## Boundaries proved

| Boundary | Implementation and oracle |
| --- | --- |
| One GPU resource domain | wgpu 30.0.2 creates a physical adapter with a minimal default device descriptor. The same safe device/queue handles enter CubeCL `try_init_device` and Slint `WGPUConfiguration::Manual`. The actual Slint rendering notifier checks device and queue identity; the native oracle requires confirmation. |
| Real image computation | A 2048 x 1536 u16 checker enters the production safe `blur_horizontal_u16` CubeCL kernel. The worker retains u16 precision; a separate fixed downsample creates a 512 x 384 RGBA8 display copy. Screenshot samples require exact black/white values before and after inversion. |
| Responsive worker and cancellation | Native Start submits finite batches of real kernel/readback work. Native Ping is observed while another batch completes. Native Cancel stops further submission at the next batch boundary; another Ping confirms submission count stays fixed. A new Start and Cancel prove reuse. There is no worker sleep, artificial barrier, fake progress or business-control hook. |
| Bounded ownership | Command capacity 1, pending-preview capacity 1, two full u16 planes, up to three RGBA staging owners, two texture-cache slots. Real RAII counters track planes/staging; cache ownership is tested through actual drops. Queue completion precedes cache eviction, and old cache ownership is released before allocating replacement. |

CPU staging is an explicit bounded bridge: this proof does not claim zero-copy
compute-to-preview or zero readback. It proves a shared GPU resource domain and
safe texture import without unsafe application buffer arguments. Native CUDA
and native Metal can use the same bounded presentation-copy boundary later.

## Results and scope

Native functional qualification passes on Linux x86-64 with NVIDIA GeForce
RTX 3090 / Vulkan / DiscreteGpu. The retained native snapshots, independent
before/after screenshots, complete driver trace and source/artifact hashes are
in [`p0-ui-evidence`](p0-ui-evidence/). Window teardown status is recorded in the
same receipt; forced cleanup is never presented as a normal close. Final native
Cancel stopped at 3/128 batches, reuse stopped at 2/128, and native Alt+F4
exited with status 0 (`forced: false`).

| Logical storage | Structural bound | Native observation |
| --- | --- | --- |
| Command / pending preview | 1 / 1 slot | 1 / 1 configured; rejected full preview drops its allocation |
| Worker full-precision planes | 2 x 6,291,456 B | Peak 2 / 12,582,912 B; terminal live 0 |
| RGBA staging | 3 x 786,432 B: producer + queued + presenter | Peak 1 / 786,432 B; terminal live 0 (structural maximum remains 3) |
| Owned texture cache | 2 x 786,432 logical B | Peak 2 / 1,572,864 logical B |
| Root blur u32 staging | 25,165,824 logical B at this size | Static bound from root input/output conversion; separate from harness counters |

These are allocation/ownership bounds, not measured whole-process RSS, GPU
allocator peak, JIT memory or driver memory. A retained image can have shared
references inside Slint; two cache slots bound harness-owned imported preview images,
not all renderer-internal allocations or references. Full 64-frame lifecycle, user-visible
latency distributions, all controls and application performance remain P7/P8/P10.

The standalone checks pass formatting, exact dependency review, strict Clippy,
private-item documentation, four meaningful Rust tests, cargo-deny advisories /
bans / sources and fail-closed unused-dependency analysis. Root library coverage,
Miri, mutation tests and sanitizer gates remain separate. No UI source coverage
percentage or all-transitive-dependency Miri soundness is claimed.

## Close-test failures and resolution

The first complete functional run passed controls, pixels, cancellation and
reuse, but the unchanged driver's fallback terminated it with SIGTERM. Its
`native-final-result.json` and `native-final-shutdown.json` retain that distinction.
A new native Alt+F4 oracle then timed out after 10 seconds; the full
`native-close-*` trace and failure tree are retained. The initial implicit
window/worker lifetime assumption was wrong.

A test-first explicit worker `Shutdown` message proves that a retained UI sender
cannot keep an idle worker alive. That change alone still timed out during native
close (`native-shutdown-*` evidence). A standard Slint `on_close_requested`
callback now requests batch-boundary cancellation and explicitly quits the
Slint event loop. After `component.run()` returns, the main thread sends Shutdown,
drops timer/component/sender ownership and joins the worker. The unchanged native
Alt+F4 oracle then passes with clean status 0. These results prove the explicit
lifecycle route; they do not diagnose a particular upstream default-close defect.
No timer delay or forced termination counts as success.

## UI generation and trust boundary

Test-first compile probe `probes/macro_forbid.rs` rejects direct generated Slint
bindings under the unmodified first-party policy:

```text
error[E0453]: allow(unsafe_code) incompatible with previous forbid
```

The error originates in `ItemTreeVTable_static`. Run
`bash qualification/p0-ui/check-macro-negative.sh` from the repository root to
reproduce it; the script requires that exact failure rather than accepting any
compilation error. The retained log records the complete diagnostic.

The safe `slint-interpreter::ComponentInstance` API avoids generated glue in the
first-party crate. It compiles only repository-owned, embedded
`include_str!("qualification.slint")` and pinned standard widgets. There is no
user-provided UI path/source, hot reload, custom import callback or source-loading
argument. This limits the parser input trust boundary; it does not prove parser
memory safety.

The interpreter invokes Slint's compiler at runtime, so Rowan is part of this
UI process. The original and narrowly layout-patched Rowan both fail the extra
strict Miri probe at `arc.rs:264`:

```text
error: Undefined Behavior: trying to retag from <139395> for SharedReadOnly permission at alloc42706[0x18], but that tag does not exist in the borrow stack for this location
```

The exact error involves `HeaderSlice` unsizing; full original/patched traces
and context are retained in [kernel qualification](p0-kernel-qualification.md#rowan-layout-boundary).
The layout patch does not alter ARC. Native UI success does not resolve this
upstream soundness concern; safe first-party calls do not certify upstream unsafe
implementations. The probe is not proof that this exact owned component triggers
native memory corruption, nor proof that it cannot. No UI Miri pass is claimed.

The interpreter therefore remains a qualified minimal proof boundary with a
known upstream limitation. A separate crate containing only generated Slint
glue could keep `unsafe_code = "forbid"` on all authored application/core/worker
code, move the parser to build time, and reduce startup/binary overhead. That
requires an explicit, narrow generated-code policy review and an A/B size/startup
measurement before selection; no blanket unsafe allowance is proposed here.
P9 must resolve or explicitly accept this UI runtime soundness boundary before
production release.

The final debug Linux binary is 1,001,110,304 bytes and includes LLVM/JIT,
interpreter and debug symbols. The final single compile/instantiate observation
is 252 ms, displayed and recorded in native state. Neither value is a release-size result, interpreter-only delta,
startup P95 or acceptance target. Prefer measured compiled-glue evaluation for
production if its narrow boundary is approved.

## Reproduction and provenance

Use the README commands with the pinned toolchain and lockfile. The runtime
qualification must use a physical GPU; absence fails rather than selecting a
software adapter. The oracle enables `org.a11y.Status.IsEnabled` only on its own
private session bus because AccessKit otherwise publishes no tree.
