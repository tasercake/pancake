# Sealed T50 Hybrid diagnostic

|---|---:|---:|
| Parent wall time through completed export and exit | 26.7365 s | 82.4410 s |
| Observed process-tree RSS peak | 6,551,072,768 B | 1,563,787,264 B |
| Maximum RSS observation gap | 26.019 ms | 10.687 ms |
| Observed attributable GPU memory peak | 4,502,585,344 B | 2,518,323,200 B |
| Maximum GPU observation gap | 30.797 ms | 69.970 ms |
| Whole-output mean absolute error against sharp truth | 979.59 u16 codes | 843.33 u16 codes |
| Vertical second-difference variance ratio, 50 focus regions | 1.1663986–1.1664015 | 0.8667194–0.8918330 |

Candidate diagnostic wall time was 3.084× the reference. Source-equal
image geometry permits direct decoded comparison. Mean absolute difference
between applications was 1,792.13, RMS 2,207.63 and maximum 19,611 on the
65,535-code scale across 71,940,012 color samples. The candidate was closer
to the sharp truth in whole-image MAE. The reference amplified the measured
vertical detail relative to truth, while the candidate attenuated it.
Equal-quality acceptance remains open. In particular, the two complete
diagnostics leave all 34 frozen timing jobs unrun. The reference RSS gap and
both GPU gaps exceed 20 ms, leaving the paired RSS and GPU peak qualification
pending. [Summary and all geometric and pixel results](summary.json)

All 50 stack sources independently decoded as native 4000×6000 RGB with
16 bits per channel in the materialization receipt. The sealed Rust TIFF
path constructs native `DynamicImage` samples and records `SampleEncoding::U16`
before direct RGB f32 conversion. The pinned reference reads TIFF through
OpenCV's unchanged-depth path and scales uint16 directly to float32 BGR
by `255/65535`. Both exported TIFFs independently passed the genuine
low-bit test. The stage retains exact source hashes and both encoded and
decoded output hashes. [Native inputs and depth](../stage-001/materialization.json.gz)

## Observed pipeline differences

Both completion records identify physical RTX 3090 computation. The reference
reported GPU warping and fusion and CPU alignment estimation. Its stderr
recorded three GPU alignment-helper singular-matrix failures with the message
`using the CPU`. The candidate reported `wgpu-vulkan` and a null fallback
reason. There was no whole-job CPU fallback.

The sealed candidate decodes, registers, and fuses one frame at a time.
`job::stream::process` invokes the accumulator after serial decode and
alignment. Packed RGB f32 at 24 MP is 288,000,000 bytes. Registration's
`make_luma` uploads the source before making and reading a small proxy.
The Hybrid accumulator separately stages the same full frame, reduces
RGB and weight pyramids, selects and blends coefficients, and waits for
written buffers before accepting the next frame. Identity transforms
skip a full-size warp. This path issues about 28.8 GB of full-frame
uploads for 50 sources across the registration and fusion stages, by
source accounting.

## Retention

Both exports were hashed again before removal. The authenticated generator
cleanup rehashed and removed all 52 TIFFs after the viewer and diagnostic
released them. The case-owned caches, package extraction, and frozen reference
environment were removed. Output TIFFs can be reconstructed from the sealed
package, reference commit and lock, and the verified generated input recipe.
[Input cleanup](input-cleanup.json) · [Export and environment cleanup](output-environment-cleanup.json) ·
[Raw and reduced receipt checksums](SHA256SUMS)
