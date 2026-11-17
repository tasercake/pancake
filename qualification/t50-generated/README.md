# GENERATED T50 focal-scale workload

The user authorized generation after bounded real-source discovery. This
explicitly replaces the former real-photo condition for T50 alone. No camera
images are promoted from 8 bits, resized, or read by this generator. No source
payload is downloaded. Generator code uses GPL-3.0-or-later, matching Pancake's
package license; generated pixels are offered under CC0-1.0.

## Recipe

| Piece | Exact definition |
| --- | --- |
| Native geometry | Width 6000, height 4000, RGB unsigned 16-bit; 50 focus positions 0–49. Seed 20261117. |
| Depth | At each native pixel, `depth = floor(x * 50 / 6000)`. Each of 50 vertical scene regions has a different known focus position. |
| Defocus | Integer radius `abs(depth - focus)`. Radius zero gives sharp detail in that region; different focus positions change spatial contrast, not global intensity. |
| Spatial detail | Separable square-wave bands with periods 4, 14 and 62, amplitudes 12000, 7000 and 3000; per-channel phases are SHA-256-derived from seed, channel and band. Each band is averaged over its exact integer square neighborhood. |
| Native low bits | RGB biases 32768/30000/35000 plus `((73*x + 137*y + seed + 431*channel) % 4096) - 2048`. Integer floor division and clipping to 0–65535 are explicit. This is direct high-bit synthesis, not 8-bit expansion. |
| Truth | `sharp_truth.tif` uses radius zero everywhere. `depth_truth.tif` stores each layer index in one uint16 channel. Depth labels need only 50 values and are not claimed as photographic high-bit data. |
| Encoding | Little-endian TIFF, contiguous RGB, 256×256 tiles, ADOBE_DEFLATE tag 8, compression level 6, horizontal predictor 2. One page. No ICC transform or conversion. |
| Hash order | TIFF tile-row-major; trim each tile to native bounds, then hash C-order little-endian uint16 samples. This explicitly differs from whole-image row-major hashing. |

`render_tile` calculates pixels directly at native coordinates, without an
intermediate full image. An independent scalar oracle sums every square-wave
sample in the integer neighborhood; it does not call the vector renderer's
prefix-sum algorithm.

## Qualification and resource ownership

For each image, the generator writes tiles, hashes original native pixels,
streams every tile through tifffile/imagecodecs, and hashes decoded pixels.
A second full pass uses Python's independent stdlib zlib DEFLATE decoder and
explicit uint16 horizontal-predictor reversal. All three pixel hashes must
match. Scalar reference samples include corners, center and each depth region.
TIFF metadata, tile order/count, distinct focal hashes, value counts and genuine
low-bit data are checked. File-byte and pixel-byte SHA-256 digests are retained
for every image; no conversion occurs, so the converted-pixel digest equals the
decoded digest.

Only one new image exists at a time. The generator verifies its hash and
ownership marker, deletes that exact image, then writes the next. It requires
a new output directory, rejects symlinks/path escapes or changed files, and
checks every candidate before cleanup deletes any file. It never recursively
removes an arbitrary directory. The output ownership marker remains as evidence.
The receipt is stored outside the image directory.

The default 15 GiB free-space guard runs before each image and each generated
tile. A separate 1,490,980,001-byte cap bounds **retained temporary image bytes**,
using a conservative per-image raw-size bound before writing. Total generated
and deleted bytes are recorded separately; they are not unique downloaded
source bytes. Decode concurrency is one worker with an 8 MiB input buffer;
rendering and predictor reversal use tile buffers. No full-frame buffer or
50-frame RAM stack is allocated.

This command always deletes verified images. Future completed-job benchmarks
must materialize the same recipe under a separately reviewed lifecycle that
preflights the full retained dataset plus the disk reserve. This qualification
does not time either application or satisfy speed/memory targets.

## Reproduce

Run from the repository root in a worktree. Use new output and receipt paths:

```sh
uv venv --python 3.12.12 artifacts/t50-venv
uv pip install --python artifacts/t50-venv/bin/python -r qualification/t50-generated/requirements.txt
artifacts/t50-venv/bin/python -m unittest discover -s qualification/t50-generated -v
artifacts/t50-venv/bin/python qualification/t50-generated/t50_generated.py \
  --output artifacts/t50-new-owned-run \
  --receipt qualification/t50-generated/evidence/new-native-receipt.json
```

The frozen qualification used the already installed
`artifacts/p0-close-io/fixture-venv/bin/python`; it made no package installs or
image downloads. Unit tests use a zero reserve only inside disposable tiny
fixtures; the native run keeps the default 15 GiB reserve.

Evidence and final native results are recorded in [evidence/](evidence/).
