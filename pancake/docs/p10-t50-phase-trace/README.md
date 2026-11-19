# T50 Hybrid physical phase trace

One instrumented T50 Hybrid job completed in 73.214 seconds on the
physical RTX 3090 using strict wgpu Vulkan. The locked release binary
was built at `64fbdb6d500b185a161e33ae1d975e293cbc64cb`, based on
`d6359e1` and kept on the experimental trace branch. Rust 1.100.0 used
Cargo.lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
The binary SHA-256 was
`10155fa5ea04be416d065c777e99902a47dc3bf98e0b33f3153725955ff57978`.
This was one diagnostic with the trace enabled, outside the frozen
acceptance schedule. The prior 82.441-second candidate supplies a pixel
witness and historical wall observation. The two runs differ in their
binary and launch conditions, including starting GPU temperature.

The run authenticated all 52 generated TIFFs and selected the same 50
ordered 6000×4000 RGB16 sources, settings, GPU UUID and 32-CPU affinity
as the historical candidate. Host memory was limited to 28 GiB with no
swap. The application recorded one completed stack on physical RTX 3090,
PCI `0000:01:00.0`, driver `580.178.04`, with no fallback. All 50 maps
were exact identity. The completed 3998×5998×3 TIFF had genuine uint16
detail and matched both the prior encoded SHA-256 `6160722086e2…` and
complete decoded pixel SHA-256 `f54746784bcc…`. All input hashes were
unchanged after execution. [Historical pair](../../../qualification/p10-performance/evidence/t50-12f2151-preflight/pair-001/README.md).

## Recorded boundaries

The trace includes 1,661 records and zero drops. Its owner-thread total
was 73.069 seconds, of which 1.138 seconds were outside the selected
top-level boundaries. Indented entries below are within their parent
stage. Durations report host call intervals. In particular, upload and
wait intervals can include host copies, submission backpressure and
prior queue work.

| Stage | Total seconds | Count or logical bytes |
| --- | ---: | --- |
| Decode | 30.358 | 50 frames |
| ↳ Native raster decode | 23.314 | 50 frames |
| ↳ RGB conversion | 6.109 | 50 frames |
| ↳ RGB validation | 0.932 | 50 frames |
| Registration | 23.567 | 50 frames with no full warp or resize |
| ↳ Full RGB proxy upload | 20.462 | 14,400,000,000 B |
| ↳ Proxy readback | 2.781 | 78,643,200 B |
| ↳ Pair estimate | 0.323 | 49 pairs, including 752 score readbacks |
| Fusion accumulation | 17.109 | 50 frames |
| ↳ Full RGB fusion upload | 13.117 | 14,400,000,000 B |
| ↳ Existing per-frame `sync_buffers` wait | 2.982 | 50 waits |
| ↳ Fusion sample validation | 0.943 | 50 frames |
| ↳ Fusion submit | 0.067 | 50 frames |
| Runtime qualification | 0.399 | One |
| Fusion reconstruction and readback | 0.187 | 288,000,000 B read back |
| Final crop | 0.016 | One |
| Encode | 0.296 | 143,881,378 B export |

Decode and the two disjoint full-frame upload call intervals together
accounted for 63.937 seconds, about 87.5% of the owner total. This
observation prioritizes the copy and upload paths for an independent
experiment. The attributable VRAM observation peaked at 2,518,323,200 B
with a 68.070 ms maximum sample gap, so this run supplies recorded
physical residency and leaves the frozen GPU memory comparison open.

The pinned CubeCL `Client::create_from_slice` first copies its slice
with `slice.to_vec()`, then `do_create_from_slices` copies that vector
again inside `Bytes::from_bytes_vec(data.to_vec())`. Fusion uses the public
`Client::create(Bytes::from_bytes_vec(f32::as_bytes(samples).to_vec()))`
route with one explicit staging-vector copy. A single-copy registration
A/B can use the latter call while retaining the same contiguous shape.
The observed intervals give the comparison's scope. The time gained by
either single-copy staging or a retained device buffer needs its own
matched experiment. [Separate resident-input plan](resident-input-plan.md).

## CPU verification, reproduction and cleanup

On exact test-only `8b01dad`, the Hybrid job completed but the test
failed because the opt-in trace was absent. The first implementation
compile also rejected redundant qualifications under the existing
warning policy. The corrected `64fbdb6` passed the real Hybrid CLI
regression and all six selected CPU lifecycle tests with the locked
headless graph. The independent CPU witness produced 41 trace records,
and with tracing enabled or disabled it produced byte-identical RGB16
TIFFs, completion events, transforms, and progress. An existing trace
file stayed unchanged in the CLI regression. Existing launch, transfer,
readback, wait, checkpoint and publication call counts matched the base
source. The timing hook remains experimental.

After the later lifecycle measurement released the host, the generator
rechecked hashes and removed all 52 TIFFs. The export was rehashed before
removal. Both disposable compile targets were reclaimed.
