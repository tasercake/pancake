# P2 image input evidence

The decoder exercise ran on the Linux x86-64 host with Rust 1.100.0. The
[receipt](p2-input-evidence/receipt.json) records source, dependency and fixture
hashes. The combined checkout's focused `cpu_image_io` target and strict
Clippy passed. The scoped Cargo dependency review passed advisories, exact
version bans and sources.

## Generated and encoded inputs

The header limit test used a 268,435,457-byte sparse DNG and ran the decoder in a
fresh process under GNU `time`. The byte-limit diagnostic appeared both times.
Maximum resident set size fell from 268,800 KiB to 6,912 KiB after checking
metadata length before allocating. The read itself also remains bounded for a
file that changes after the metadata check.

## Real Canon camera geometry and rendering

The selected R50 frame was the pinned EOS R5 Mark II
`frame_000.CR3` (SHA-256 `4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca`).
Rawler 0.8 develops the complete sensor using the file's recommended crop
(5088×3392 at 272,172). LibRaw 0.22.0 exposes the same recommended rectangle and
renders a wider table-defined 5118×3412 rectangle at 258,162 under the frozen
reference settings. On the common crop, the full-frame normalized RGB MAE was
0.075572, p99 0.094133 and maximum 0.788388. The reference was rawpy 0.26.1 with
camera white balance, no auto brightening and 16-bit output.
[Geometry and pixel receipt](p2-input-evidence/r50-raw-decode-comparison.json)
[Canon table in LibRaw 0.22.0](https://github.com/LibRaw/LibRaw/blob/0.22.0/src/metadata/identify.cpp)

The controlled one-in-eight pixel comparison reduced MAE from 0.075558 to
0.019858 by matching the sRGB transfer. Both decoders exposed the same black
level, camera WB multipliers and D65 matrix. Rawler normalizes to the camera
white tag 14,888, whereas the reference starts from the 14-bit nominal 16,383
and can adjust to the observed visible mosaic maximum 16,293. These values
account for the observed linear gain when black level 2,048 is subtracted.
A separate test-only Rawler development replaced the camera tag white level
with the nominal value derived from the actual CMP1 sensor precision. On the
common crop with linear transfer, PPG and LibRaw measured-maximum adjustment
disabled, MAE was 0.002063, p99 0.012637 and maximum 0.651426. The observed
differences persist with this explicit metadata-driven variant.
[Tone comparisons](p2-input-evidence/r50-tone-analysis.json)
[Nominal-white overlap](p2-input-evidence/r50-nominal-overlap.json)
[LibRaw gamma and maximum adjustment](https://github.com/LibRaw/LibRaw/blob/0.22.0/doc/API-datastruct.html)

As an independent reference, an unmodified local LibRaw 0.22.0
`dcraw_emu` at `0b56545a4f828743f28a4345cdfdd4c49f9f9a2a` produced
5118×3412 RGB16 TIFF on stdout with two OpenMP threads and
`-w -W -6 -q 3 -T -Z -`. Against rawpy the mean error was 1.0324 and
the maximum was two u16 codes. The tool was built locally with
`autoreconf --install`, `./configure --disable-static --enable-openmp`
and `make -j4`. The native probe and temporary raster stayed outside the
application graph.
[Native reference comparison](p2-input-evidence/libraw-native-decode-comparison.json)
