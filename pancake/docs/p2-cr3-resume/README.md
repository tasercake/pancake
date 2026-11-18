# Canon CR3 crop and rendered parity

| Source | Bytes | SHA-256 |
| --- | ---: | --- |
| `frame_000.CR3` | 8,869,034 | `4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca` |
| `frame_049.CR3` | 8,813,738 | `98fe6e370fc2ab58ba7b45d242a8c2f2a1942404ee0e0524c5cb33048969dc36` |

## Geometry

Both files' Canon MakerNote SensorInfo, displayed image dimensions and RAW
track IAD1 recommended bounds give 5088×3392 at (272,172). The underlying
mosaic measures 5376×3574. The IAD1 active offsets start at (132,160) and
give a right offset of 5243. The recommended crop extends through column 5359,
so Rawler's `activearea_bug` path for this capture falls back to the recommended
area. Dropping only `CropDefault` retained exactly the same origin, dimensions
and measured differences. Dropping both crop operations yielded the complete
5376×3574 mosaic extent after development. [ExifTool 12.76 geometry trace](exiftool-geometry.txt)

LibRaw's 0.22.0 Canon sensor table determines the reference rectangle
5118×3412 at (258,162), from total dimensions 5376×3574 and the table's left
and top offsets. The observed file's recommended and active rectangles supply
different boundaries. A metadata-only change to Rawler cannot recover this
table-defined choice. [Version-pinned LibRaw identification](https://github.com/LibRaw/LibRaw/blob/0b56545a4f828743f28a4345cdfdd4c49f9f9a2a/src/metadata/identify.cpp#L275)
[Rawler 0.8 CR3 decoder](https://docs.rs/crate/rawler/0.8.0/source/src/decoders/cr3.rs)

## Rendering

The comparison uses the same absolute sensor coordinates with no resampling.
Errors are normalized to 0–1 RGB values. The nominal and measured probes use
the reference rectangle as an explicit diagnostic input. The reference's
piecewise power transform was reconstructed from its toe slope and exponent
and verified separately on linear LibRaw output. That reconstruction differed
from the reference by at most about two u16 codes.

| Probe | Frame 000 mean absolute error | Frame 049 mean absolute error |
| --- | ---: | ---: |
| Rawler default vs frozen reference, 5088×3392 common sensor crop | 0.075572 | 0.075289 |
| Rawler default vs LibRaw configured for sRGB, same crop | 0.019691 | 0.020268 |
| Rawler PPG, oracle crop, CMP1 nominal white and reference power transform | 0.004299 | 0.004377 |
| Rawler PPG, oracle crop, measured visible maximum and reference power transform | 0.004100 | 0.004097 |
| LibRaw PPG vs its frozen AHD pipeline, everything else identical | 0.002816 | 0.002813 |

Rawler and LibRaw identify the same black level of 2048, camera WB and D65
calibration matrix. The camera saturation tag is 14,888. LibRaw's initial
white level is 16,383 from the CMP1 14-bit precision. Its measured-maximum
policy uses the visible peaks 16,293 and 16,280 for these captures. The
measured-white Rawler probe still differs by more than one u8 code in about
35% of RGB samples. Its p99 differences are 0.022202 and 0.022171, and max
differences are 0.724498 and 0.742702. LibRaw's own PPG override moves about
25% of samples by more than one u8 code compared to its default AHD. The
remaining difference can include interpolation, white-balance operation order,
clamping and rounding. [LibRaw postprocessing](https://github.com/LibRaw/LibRaw/blob/0b56545a4f828743f28a4345cdfdd4c49f9f9a2a/src/postprocessing/dcraw_process.cpp)
[LibRaw output transform](https://github.com/LibRaw/LibRaw/blob/0b56545a4f828743f28a4345cdfdd4c49f9f9a2a/src/postprocessing/mem_image.cpp#L194)

## Reproduce

From the recorded source revision, acquire only the two manifest rows above
through their pinned R50 URLs and verify size and SHA-256 before saving the
images in a new `artifacts/resume-cr3` directory. The independent comparison
checks both again. Use the exact manifest and probe source on this page.

```sh
scratch=artifacts/resume-cr3
evidence=pancake/docs/p2-cr3-resume
mkdir -p "$scratch/probe/src"
cp "$evidence/probe-manifest.toml.txt" "$scratch/probe/Cargo.toml"
cp "$evidence/raw_probe.rs.txt" "$scratch/probe/src/main.rs"
cp pancake/Cargo.lock "$scratch/probe/Cargo.lock"
cargo +1.100.0 build --manifest-path "$scratch/probe/Cargo.toml" --offline -j 6
cargo +1.100.0 build --manifest-path "$scratch/probe/Cargo.toml" --locked --offline -j 4
uv venv --python 3.12.12 "$scratch/venv"
uv pip install --python "$scratch/venv/bin/python" rawpy==0.26.1 numpy==2.5.4
OMP_NUM_THREADS=2 OPENBLAS_NUM_THREADS=1 "$scratch/venv/bin/python" \
  "$evidence/compare.py.txt" "$scratch" frame_000.CR3 frame_049.CR3
```

Workspace lock SHA-256 `639c5c88b9286e5583174f05168412db19f39dc3f651b6682b685d1db801e0dd`.
Isolated lock after resolution SHA-256 `5ae51d437cb2dca00d58d7de3e62deb6b1de14715ce54dd6916d21539f51ef1e`.
Uncompressed machine receipt SHA-256 `bc0e11b5326bcde942c13dec3abf40f404374580755cf767de2d15f2162ee8f2`.
The probe removes each intermediate f32 raster after comparison. Delete the
two temporary CR3 inputs after the final diagnostic.
