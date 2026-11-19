# Hybrid split and registration lattice source review

The revised Hybrid split clears the imported-device binding defect found in
`80da401`. The registration lattice keeps the previous scalar route at tight
budgets and device page limits. The exact lattice branch still needs its
reviewed Miri source inventory refreshed before integration. Both changes
need same-device physical output comparisons because their f32 intermediate
boundaries change. This review used Git objects and pinned dependency source.
It did not compile or open a device.

## Hybrid split

Reviewed source `3d2782e8e30a66cef1b5248d62ff6f8042c30736` and evidence
`903836ca831ba1e92b7d4f006836320dd8cbe402` against
`b820da05d3982a7912cb7c7c9d8aa926ece14a9c`. The evidence commit adds docs
only. All seven accompanying SHA-256 records, the updated
`src/fusion/advanced/levels.rs` inventory hash, and all 22 reported CPU tests
were verified against the recorded artifacts. The suite and Cargo lock are
unchanged. The isolated Hybrid branch retains the owner's receipt at
`903836c:pancake/docs/p10-r50-hybrid-band/README.md`.

Hybrid passes fine and lower levels to the existing `add_weighted` body.
Its selector receives fine, lower, weight, selected coefficients and winner
scores. Both dispatches use five data tensors. Default CubeCL WGSL adds its
usual read-only storage metadata at index 5. These requirements match the
previous peak and hold even for imported devices with independent storage
and bind-group limits. No capability branch remains.

The weighted pass and winner pass write distinct persistent output pairs.
The producer and both consumers are issued on the same client. Their cloned
arguments retain the fine and coarse allocations through launch. At the
highest level both inputs may refer to the same read-only fine allocation;
zero lower width bypasses expansion just as in the existing weighted path.
The first winner initializes scores and all three channels without reading
old scores. Strict `>` preserves the earlier winner on an exact tie. The
existing pixel-bounds guard, ceil-half pyramid and clamped `expanded` helper
also preserve narrow and odd extents. `AdvancedAccumulator` retains the
same per-frame drain of all four state handles. Exposure and Laplacian take
their original routes.

The real CPU red test issued 50 launches for two four-reduction frames. The
candidate removes four band dispatches per frame and fits the new 42-launch
bound. Retained CPU differential output is 235,962 f32 channels, equal bit
for bit. Removing the global band tensor makes both consumers lower the same
subtraction independently. GPU compiler contraction can differ even when
CPU bits agree. Retain same-device raw f32 A/B against the old source on
portable Vulkan and native CUDA, including signed bands, positive ties,
zero-weight frames and odd boundaries. Keep proof of matching runtime and
zero fallback and the relevant frozen P4/P6 completed-stack regressions.

## Registration luma lattice

Reviewed `7e29d6e5685f07c46a140e1185f9ec165fba269b` and its evidence child
`4daae4dd21532feee8044c5caa0c6f1843b6c472`. A destination `w × h` uses
`(w+4) × (h+4)` device samples. The last tap from the last output reaches
exactly the last allocated cell. Host checked arithmetic enforces u32
indexing and one device page. Normal thumbnails also retain the existing
thumbnail-size bounds. The shifted half-center coordinates match each
previous direct tap before scaling: all u16 coordinates and the four halo
steps retain exactly representable half-integers in f32. Both routes use the
same clamped bilinear and luminance helper and the same row-major 25 terms.
Each of the two new dispatches has two data tensors plus metadata, the same
limit as the scalar sampler.

The base admission check runs before device initialization. If the caller
cannot budget an additional `4 × (w+4) × (h+4)` logical bytes, or that array
exceeds `max_page_size`, the unchanged scalar kernel is selected. The first
pass produces, and the second consumes, one temporary on the same client.
The existing output readback waits through both. The optional current-frame
resident input is still a clone of the original checked RGB allocation.
CPU fixtures cover exact-budget fallback, overflow checks, constrained
buffer size, unequal fractional scales and one-pixel edges. Physical
same-device f32 readback and registration transforms remain necessary.
The extra allocation may also affect actual process/device memory even
when the logical allocation fits the caller's budget.

On both exact lattice revisions, `assurance-platforms.json` still declares
`src/registration/kernels.rs` as
`d2026ee3cbd1e4ef0f66af7a7d8e043d36df334b1c1fe8a03cef77f4c2694630`.
Git bytes hash to
`6198b601fcdfc97097b23f21ae7cc056a29da6327a59983a0dab6ef7e30eb4dc`.
The new `src/registration/kernels/tests.rs` is missing from the inventory;
its SHA-256 is
`28cea02802b1103e260bc628448d9ba62c40d689f42a3c4ef9aa5846c065bd11`.
The pinned Miri platform policy validates the source inventory and hashes.
Refresh the reviewed entries under that policy and validate the resulting
revision before integration. Focused CPU success is recorded separately.
The isolated lattice branch retains the owner's receipt at
`4daae4d:pancake/docs/p10-registration-luma-lattice/README.md`.
