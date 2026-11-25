# Single-frame input reuse experiment

Run this as a separate test-first CPU and physical GPU worktree after the
phase study. Keep the experimental phase instrumentation available for
byte counts and held waits. The initial scope is one slab-free aligned
Hybrid job, on a single selected device and owner thread.

1. Specify a regression against the present route before editing the
   kernel entries. Run a real same-canvas identity stack with the existing
   RGB16 image and transform witnesses. Check for one registration source
   upload per original and one additional fusion source upload per original
   on the baseline. Assert the proposed one-upload route and record the
   failure. Use a translated frame and mismatched dimensions as controls
   for the existing checked normalization route. Preserve source-order
   errors and the prior export when a later source is corrupt.
2. Split the private `registration::kernels::make_luma` boundary so
   the existing packed f32 `TensorHandle` can remain owned by the
   current registration call after the existing proxy readback. Keep
   the public thumbnail behavior unchanged. Carry the retained handle
   through the internal `Alignment::estimate` and `normalize` path only
   for an original that matches the canvas and has an eligible consumer.
   Record the `Device` identity and checked extent used to create it.
   Keep the handle in a private, consuming per-frame value owned by this
   job, without adding it to the cached proxy or transform history.
3. If the accepted original map is exact identity and the extent still
   matches, consume the handle in a private `AdvancedAccumulator` entry
   point after validating against the active device and exact packed
   shape. Keep the original CPU-side sample validation. The existing
   `weights` and pyramid code take that handle as the input tensor.
   Preserve the original registration readback and per-frame
   `sync_buffers` wait before progress. Drop the retained handle before
   an ineligible warp, resize or error can allocate another full frame.
   Keep slab overlap and other methods on the existing route. Charge the
   same live input under both existing phase budgets. No device resource
   moves to a second thread or survives the current frame.
4. Require bit-identical decoded RGB16 output, matching matrices and
   real device completion, and the same held waits across both routes.
   Validate CPU lifecycle and equivalent physical completion before
   measuring. Admission remains at the existing checkpoints. Pause and
   cancellation keep their existing safe acknowledgments and the
   separate reusable-Idle drain. Also confirm peak process and device
   memory stay within the applicable bounds.

A smaller independent experiment replaces registration's borrowed
`create_from_slice` call with the existing owned `Client::create` upload
pattern from fusion. It preserves the buffer layout and measures the
extra host copy separately. Keep one independent variable per measured
comparison. Decode lookahead remains a separate experiment because it
changes admission, in-flight memory and the pause/cancel boundary.
