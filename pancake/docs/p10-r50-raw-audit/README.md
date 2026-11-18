# R50 RAW decode source audit

## Where the implementations differ

| Stage | Pancake | Corrected reference |
| --- | --- | --- |
| Source ingress | Reads each original once into an owned byte buffer, lends the selected MDAT region to Rawler, and performs both decoder selection and CR3 sensor preflight. The two BMFF parses inspect structure rather than repeating decompression. | Checks the file brand and bounds, then opens and unpacks through RawPy's LibRaw path. The adapter calls full RGB16 postprocessing with metadata crop, camera white balance, saturation, sRGB and orientation. |
| Work distribution | With no slabs and a first-frame reference, one job loop sequences decode, alignment and fusion. It reuses the first full frame and stores a compact alignment reference. Rawler's explicitly sized two-thread pool runs its nested Rayon work regardless of the harness's Rayon environment. | CPU Hybrid chooses up to six frame workers based on `os.cpu_count()` and available RAM, with a lookahead of workers plus one. Load and alignment overlap ordered accumulation, and per-frame pyramid preparation can also run in workers. OpenCV sets its thread count from `os.cpu_count()`. The common job affinity still constrains both programs. |
| Sensor and development | CRX decodes tiles and planes with Rayon, produces each plane's lines serially and retains the decoded `i32` line vectors. It then assembles the CFA serially, allocating four `u16` line vectors per reconstruction row. Development clones the decoded RAW, expands and normalizes the mosaic, and creates a Bayer PPG RGB raster in the active area. White balance is already fused into the parallel calibration pass. Default crop can copy to a new raster, followed by parallel sRGB transfer. | The adapter leaves sensor decode, full demosaic and calibrated RGB16 development in LibRaw. It slices the camera rectangle and applies orientation through NumPy views. Converting the cropped BGR view to float32 uses a cast and scale. LibRaw's actual inner thread occupancy was not measured. |
| Final ingest representation | Rawler flattens the developed array of RGB float triples into one additional full-size float vector, quantizes to `ImageRgb16`, then Pancake applies orientation and converts that image to normalized working float32. | The NumPy conversion retains the developed RGB16 codes and scales the BGR working samples to the reference's 0–255 float range. |

## Narrow copy removal

Candidate `9e70ccf` borrows the RGB channel slice using `as_flattened()`
and sends it to the identical public Rawler float-to-u16 quantizer.
It constructs the same `ImageRgb16`, keeps the existing orientation and
u16-to-float path, and delegates other intermediate types to Rawler.
At the camera's 5088×3392 developed extent this eliminates a temporary
207,101,952-byte allocation and one logical read and write of that
extent per frame. Across 50 inputs the logical copy traffic is about
19.3 GiB. This is a buffer-size calculation. Actual traffic, peak RSS
and savings depend on allocation, cache and pipeline behavior. The
other full-image work and the ordered two-worker ingress remain.

The candidate tests pin truncation, saturation and component order,
including nonfinite inputs, and compare every EXIF orientation and
working f32 sample bit against the locked original path. Formatting
and the source-review manifest validate. Compilation and real-image
parity require a separate execution outside the official timing window.

## Offline qualification

Use two already verified original Canon inputs from the existing R50
corpus. Build and pin both clean release revisions after the timing
window. Compare the unchanged public decode geometry, encoding and
f32 sample bits frame by frame. Compare the RGB16 pixels and byte hash
of each complete export at the same settings. Keep the camera and
stack crops, white balance, color calibration, sRGB transfer and u16
quantization throughout.

Use a separate diagnostic build to collect monotonic per-frame
durations for bounded read, decoder selection and sensor preflight,
CRX decompression, Rawler development, RGB16 packing, orientation and
working normalization. Collect stage totals after the complete
process exits. If packing is material, compare baseline and
candidate on the same two originals, cache policy, CPU mask and thread
settings. This isolates the copy removal while preserving the official
whole-job evidence. If decode stages dominate, compare controlled
RAW pool widths separately with the same pixel checks and peak RSS.
A one-frame decode-ahead queue would be a distinct scheduling change
with explicit working-memory and lifecycle bounds.
