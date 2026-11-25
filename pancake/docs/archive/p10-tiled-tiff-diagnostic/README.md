# T50 tiled TIFF import diagnostic

The bounded decoder streams each DEFLATE-encoded contiguous RGB16 tile through the pinned `tiff` codec into a reusable typed buffer. It writes normalized RGB directly into the working raster, keeps byte-exact ICC data within the existing profile budget, and reads each compressed span from the existing source handle. Other layouts and nonidentity orientations follow native raster import.

Against `2143eda2b308497caee96334357dd23d324e1e8e`, the test-first run accepted a frame whose first tile declared one compressed byte. The changed decoder rejects this input, zero-length tile spans, and spans extending beyond the file. The independent 257×259 fixtures use 64×64 tiles, horizontal prediction, a retained sRGB ICC profile, and the row-major RGB code sequence `index % 65536`. Little-endian, big-endian, and rotated big-endian cases preserve every normalized code bit, including the partial right and bottom tiles. These fixtures were generated with the frozen T50 generator's `tifffile` and `imagecodecs` versions.

The clean integrated candidate `70ee7bfe7696105c58caafe4269b594feb3619e6` passed formatting, reviewed source inventory validation, scoped strict Clippy and 39 Linux CPU image I/O, export and NumPy tests. Those tests also compare every RGB16 sample and the ICC bytes from an independent `tifffile` fixture with the historical DEFLATE storage tag `32946`. [Focused test log](focused-codec-70ee7bf.log.gz) · [Red and green receipt](receipt.json)

Three authenticated 6000×4000 T50 frames ran three times per decoder on CPU 20 of the AMD Ryzen 9 7950X3D. All 648,000,000 candidate sample bits matched the baseline. Other application and build jobs ran on the host during the measurement.

| CPU decode metric | Baseline | Streamed |
| --- | ---: | ---: |
| Median across nine runs | 1.076 s | 0.947 s |
| Separate frame 024 wall inside decode | 1.431 s | 1.077 s |
| Separate frame 024 peak process RSS | 425,392 KiB | 285,184 KiB |

This is a TIFF import diagnostic. Physical end-to-end T50 timing and frozen P10 qualification remain open.

The shared materialization verified all 50 focus frames and both truth images against the frozen encoded and decoded identities. After the consumers released the media, the generator rehashed and removed all 52 TIFFs, releasing 2,775,174,405 bytes. The cleanup receipt records zero remaining TIFFs. [Shared materialization](t50-materialization.json.gz) · [Cleanup](t50-cleanup.json)

Recreate the inputs from the [frozen T50 materialization procedure](../../../qualification/t50-generated/README.md). Build the [probe source](probe-source.tar.gz) with the same Rust 1.100.0 toolchain and pinned crate versions. The probe includes both production TIFF paths and removes the unrelated RAW, NPY and export modules from its diagnostic build.

```sh
cargo +1.100.0 build --release --locked
taskset -c 20 ./target/release/pancake-t50-tiff-probe alternating \
  "$stage/inputs/focus_000.tif" "$stage/inputs/focus_024.tif" "$stage/inputs/focus_049.tif"
for arm in native streamed; do
  /usr/bin/time -f 'elapsed_seconds %e\npeak_rss_kib %M' \
    taskset -c 20 ./target/release/pancake-t50-tiff-probe "$arm" "$stage/inputs/focus_024.tif"
done
```
