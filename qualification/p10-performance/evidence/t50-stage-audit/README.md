# T50 Hybrid stage audit

## Work confirmed in the candidate

- `job::execute` decodes the first frame before selecting a runtime.
  `job::stream::process` consumes that frame and decodes the remaining
  inputs serially before alignment and accumulation. T50 TIFFs use
  `image_io::decode_raster`. Native raster decode, orientation, bounded TIFF
  ICC inspection, `into_rgb32f`, and sample validation use the host.
  DNG and CR3 go through `image_io::raw::decode` and its existing bounded
  two-thread RAW development pool. That path did not run for T50.
  [Job entry](../../../../pancake/src/job/mod.rs)
  · [Stream](../../../../pancake/src/job/stream.rs)
  · [Raster decode](../../../../pancake/src/image_io/mod.rs)
  · [RAW decode](../../../../pancake/src/image_io/raw.rs).
- With First-reference similarity, each source goes through
  `Alignment::estimate`, including the first frame's proxy. The
  `make_luma` kernel uploads that frame as 72 million packed f32
  samples, makes a 768×512 luma proxy at scale factor six, then reads
  the proxy back. Each of the 49 `estimate_pair` calls then uploads
  both proxy arrays via `Pair::new` and reads back correlation results.
  The retained first-frame proxy is host data, so the two proxy uploads
  total a further 154,140,672 logical bytes across those calls.
  [Alignment](../../../../pancake/src/job/registration.rs)
  · [Estimator](../../../../pancake/src/registration/estimate.rs)
  · [Kernels](../../../../pancake/src/registration/kernels.rs).
- Identity and matching dimensions let `Alignment::normalize` return
  the original host samples. `AdvancedAccumulator::submit_frame` then
  uploads those same samples, calculates weights, reduces each pyramid
  level, and blends. Its existing `sync_buffers` completes the frame
  before progress advances and the next lifecycle boundary admits work.
  Each full RGB upload is 288,000,000 bytes. The two call sites account
  for 28.8 GB over 50 frames, of which 14.4 GB and one host staging copy
  per frame are avoidable. These are logical byte counts. Explaining all
  55.704 s with those 14.4 GB alone would require only 0.259 GB/s of
  exposed throughput. The archived pair cannot attribute that interval
  to copying or its associated allocation and queue waits.
  [Advanced accumulator](../../../../pancake/src/fusion/advanced/mod.rs)
  · [Upload helper](../../../../pancake/src/fusion/buffers.rs).
- The identity shortcut avoids a full-size warp in this pair. After one
  final reconstruction and readback, `crop_samples` compacts the
  output on the host. Quality reporting was off, so there was no
  sharpness upload. TIFF quantization, spool writing, publication and
  exit are included in the observed wall times. The output sizes also
  differ, 143.9 MB for Pancake and 78.3 MB for the reference.
  [Stream finalization](../../../../pancake/src/job/stream.rs)
  · [TIFF export](../../../../pancake/src/image_io/export.rs).

## Attribution run

Use one opt-in trace on the exact sealed source and locked release build.
Store bounded timing and byte records in memory and write them once after
publication. Keep the existing waits, order, kernels, settings, diagnostics
and outputs. Record the first decode and runtime acquisition separately.
For every original, record these nonoverlapping host intervals and counters.

| Boundaries | Counters and observations |
| --- | --- |
| `decode_raster`, `DynamicImage::from_decoder`, RGB conversion and `RgbImage::new` | Original index, bytes read from receipt, raster/model format, decode versus conversion and validation time |
| `make_luma` upload, dispatch to existing readback, then `estimate_pair` | Full RGB bytes, proxy readback and pair-upload bytes, number of score readbacks, elapsed estimation, identity versus warp |
| `AdvancedAccumulator::add_frame` validation, upload, weight/pyramid submission, `sync_buffers` | Full RGB bytes, queue submission time and exposed drain wait by frame |
| Final reconstruction/readback, crop, export staging and commit | Readback and output bytes and elapsed time by stage |

Time the actual blocking call and record CPU submit time separately from it.
A wait can cover earlier queued work. It cannot partition DMA and kernel
execution by itself. Keep the top-level stage sum and parent wall residual
so startup, output, progress and related overhead remain visible. Compare
trace-enabled and trace-disabled runs on re-materialized, authenticated
T50 with the same GPU, affinity, settings and cache policy. Require equal
decoded outputs and transforms between those two candidate runs. These
are diagnostic results. The frozen acceptance schedule uses uninstrumented
binaries. [Frozen targets](../../../../pancake/docs/performance-targets.json).

## Follow-up experiments

The smallest transfer A/B retains the contiguous `TensorHandle` already
uploaded by `make_luma` until `Alignment::normalize` decides the geometry.
For identity at the original canvas extent, pass that same job/device-owned
handle to a private `AdvancedAccumulator` entry point and skip the second
upload. Retain the existing sample validation and `sync_buffers` in both
variants. Keep each phase's budget check and charge the live input in both
phase estimates. The 4 GiB logical job budget reserves 1 GiB for registration
and 3 GiB for fusion for this job. Reject a handle from a different job,
client, or shape. Keep the tested host route for resize, nonidentity warp,
large tiles, slabs, and other methods until their ownership is separately
qualified. Final crop still operates after reconstruction. Test identity
RGB16 output and transforms, nonidentity selection, failure cleanup, and
physical backend receipts before comparing time and observed memory.

If host decode is a substantial exposed interval, a separate A/B can admit
one next-frame host decode while the owner computes the current frame.
Account for both the held frame and the in-flight decoder's scratch before
admission. T50 holds 288 MB decoded RGB per slot and conversion alone can
also hold its 144 MB RGB16 raster. Include orientation, ICC, encoded input
and decoder overhead in the bounded admission check. The owner keeps
estimator state, GPU submits, frame ordinals, progress, error ordering and
export publication serial. Stop admitting work on pause or cancel. Drain a
scheduled decode before safe pause acknowledgment and prevent all new
decode or compute submissions after either safe acknowledgment. Retain
cancellation-to-reusable-Idle and resource drain as separate measures.
A blocking decoder can prevent a timely safe boundary. Qualify the frozen
250 ms deadline with actual 64-frame and selected real jobs before adoption.
Concurrent estimation needs its own proof, especially for sequential
references. [Lifecycle](../../../../pancake/src/job/lifecycle.rs).

The trace and two diagnostic jobs should take about 10–12 minutes once
the trace patch is ready and resources are quiet. That estimate includes
a 3–5 minute release build, the recorded 229-second input materialization,
and two approximately 82-second jobs. The separate transfer A/B can reuse
the authenticated staged inputs while their owner retains them. This audit
read source and archived receipts only.
