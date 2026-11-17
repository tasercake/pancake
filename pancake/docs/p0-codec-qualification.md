# P0.08 RAW, codec and color qualification

## Concrete dependency selection

| Piece | Exact candidate and role | Why this choice; limits |
| --- | --- | --- |
| Standard images | `image =0.25.10`, selected `jpeg/png/tiff/exr` features | Typed Rust-facing buffers and codec dispatch. Avoid default codecs that the application does not require. PNG16 and float EXR interpretation were exercised here; this is not a claim that every required format/orientation/error has passed. |
| TIFF | `tiff =0.11.3` directly | Explicit `RGB16`, `RGB32Float`, sample types and ZIP compression. Avoid conversion through 8-bit preview buffers. Direct APIs also support future chunked I/O; this probe did not qualify bounded chunking. |
| EXR | `exr =1.74.3`, default features disabled | Safe Rust-facing full-float channels with lossless storage. Disabling its optional Rayon feature avoids an extra default worker pool. The image integration still uses this package. |
| RAW fallback | `rawler =0.8.0`, owned-buffer API | Full sensor decode proved against LibRaw, including actual CR3 compression and Bayer DNG. No new application FFI/unsafe. **Provisional:** camera-specific geometry and rendered color/demosaic parity remain unresolved. Rawler itself uses internal unsafe/SIMD and has a mandatory Rayon dependency; bound its decode pool explicitly. |
| ICC color | `lcms2 =6.2.0`, `static`; resolved `lcms2-sys 4.0.7`, bundled LCMS 2.19 | Typed `[u16; 3]` buffers use the binding's existing checked pixel-layout contract. No custom unsafe pixel type. Static build needs a C compiler; Linux/Windows packaging and combined application policy graph remain untested. |

The exact probe lockfile lives in
[`../../qualification/p0-codecs/Cargo.lock`](../../qualification/p0-codecs/Cargo.lock).
These packages are not silently added to the production graph. The integrated
Slint/CubeCL/codec graph must pass the application audit after selection.
Primary package APIs: [image](https://docs.rs/image/0.25.10/image/),
[tiff](https://docs.rs/tiff/0.11.3/tiff/),
[exr](https://docs.rs/exr/1.74.3/exr/),
[Rawler](https://docs.rs/rawler/0.8.0/rawler/),
[lcms2](https://docs.rs/lcms2/6.2.0/lcms2/).

## Why no LibRaw binding was adopted

LibRaw remains the preferred compatibility engine. The reviewed crates did not
clear the safety/API/native-build boundary:

| Candidate | Observed source limitation |
| --- | --- |
| `libraw-rs 0.0.4` | Requests old `libraw-rs-sys 0.0.4` / LibRaw 0.20.1. `Processor::new` does not reject a null native allocation; `RawImage::deref` constructs a u16 slice from `raw_image` without a format/null guard. Public processing API lacks the required explicit WB/brightness/color controls. Not qualified for arbitrary RAW inputs. |
| `rsraw 0.1.1` | `RawImage::open(&[u8])` retains the native buffer stream without encoding its borrow lifetime before a later `unpack()`. Safe `AsMut<libraw_data_t>` exposes native owning-pointer state; mutable state can invalidate later safe operations. Const bit depth is checked only by a debug assertion. No memory-unsafe demonstration was executed. |
| `rawlib 0.7.1` | Better explicit 16-bit/AHD/WB/brightness/gamma options and allocation checks. Build script falls back to bundled GNU/Linux archives and links `stdc++` on macOS; local `libraw_r` is absent. It uses handwritten native declarations. Neither cross-platform native build nor ABI boundary was qualified; no compile success is claimed. |
| `libraw 0.1.1` | Older wrapper requires native reentrant `libraw_r` through pkg-config. Not a qualified current engine/build route here. |

These are local source-review findings, not new RustSec advisories or proof that
every path in those libraries is unsafe. No binding was relabelled, broadly
exempted, or patched by adding first-party unsafe. Primary published source:
[libraw-rs](https://docs.rs/crate/libraw-rs/0.0.4/source/),
[rsraw](https://docs.rs/crate/rsraw/0.1.1/source/),
[rawlib](https://docs.rs/crate/rawlib/0.7.1/source/),
[libraw](https://docs.rs/crate/libraw/0.1.1/source/).

## Executed evidence

Real input: D02 `frame_000.CR3`, Canon EOS R5m2, 8,869,034 bytes;
SHA-256 `4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca`.
Source is the pinned CC BY 4.0 Sood dataset revision already recorded in the
adapter's `performance-raw-corpus.json`. Original photograph was read only.
The independent oracle uses rawpy 0.28.0 / LibRaw 0.22.2. Corpus acquisition used
rawpy 0.26.1 / LibRaw 0.22.0; these versions are deliberately distinguished.

| Check | Actual result |
| --- | --- |
| Full real sensor decode | Rawler safe public owned-buffer API produced 5376×3574 u16 samples. All 19,213,824 values match LibRaw bit for bit. Mosaic byte SHA-256: `81cff63b4c1b0cb71ddc0bca8551721c362b378b6b525f3fb09ea12f1965ca32`. |
| Sensor precision | Source CRX `n_bits=14`, white level 16383, nonzero low eight bits. Rawler's camera `bps` field says 16 for storage; this does not imply a 16-bit sensor. |
| Bayer DNG | Actual generated 32×24 CFA DNG decoded by both Rawler and LibRaw; all 768 sensor samples match independent expected bytes. This is a diagnostic generated fixture, not a real focus corpus. |
| PNG16 and TIFF16 | Twelve RGB samples span 0–65535 and include 1, 255, 256, 257, 258 and 32769. Rust round trips and independent OpenCV/tifffile reads preserve every value. TIFF tag reports `ADOBE_DEFLATE`. |
| Float TIFF and EXR | All 12 f32 channel representations match independent tifffile/OpenEXR reads. Samples include negative values, values above 1, 65504 and `0x3f000001`, which cannot survive an f16 intermediate. Tags/channels are float32. |
| ICC no-op | Validated byte-identical profiles use explicit passthrough. Every u16 value is covered across 65,536 RGB pixels, with exact preservation. |
| Actual ICC transform | sRGB to a separate linear sRGB ICC profile through typed safe Rust API. Independent system LCMS 2.18 C API reads the same profile bytes: zero u16 error across 12 samples; acceptance tolerance fixed at one u16 before measurement. |
| Basic invalid input | Malformed image bytes and malformed ICC profile rejected. This does not qualify resource limits or every malformed format. |

The unchanged scaffold's required `bash scripts/check.sh all` was run again.
It returned 0, but still emitted six vendored-crate cargo-machete metadata
errors, as already recorded in the upstream patch review. This is not a clean
integrated gate pass; the policy/kernel workstream owns that integration fix.

An ICC serialize/reload conversion initially changed low-bit values by +8/-1.
Serialization can quantize a profile, so semantic similarity is not a guarantee
of bit identity. Final policy bypasses conversion only when validated profile
bytes and typed sample encoding are identical. Distinct profiles take the real
transform path. This is probe policy, not yet application integration.

The independent float oracle was also strengthened before its final run: it
rejected a one-ULP mismatch at `0x3f000001`, then passed after the Rust fixture
supplied that complete f32 representation. No tolerance was widened.

## RAW crop compatibility blocker

| Geometry | Origin in full sensor | Size |
| --- | --- | --- |
| Full decoded sensor | 0, 0 | 5376×3574 |
| LibRaw visible image / current reference output | 258, 162 | 5118×3412 |
| CR3 SensorInfo and IAD1 recommended crop | 272, 172 | 5088×3392 |
| Rawler active/crop result for this input | 272, 172 | 5088×3392 |

LibRaw 0.22.2's Canon sensor-size table explicitly selects margins 258/162 for
5376×3574 R5mk2 APS input. Its default crop still matches the file's 272/172
recommended crop. Rawler reads that file crop and its existing `activearea_bug`
camera hint falls back to it. The difference is not missing sensor pixels:
the complete decoded mosaics are identical. See
[LibRaw's pinned table and selection](https://github.com/LibRaw/LibRaw/blob/0.22.2/src/metadata/identify.cpp),
[dimension semantics](https://www.libraw.org/docs/API-datastruct.html#libraw_image_sizes_t),
and [Rawler CR3 decoder source](https://docs.rs/crate/rawler/0.8.0/source/src/decoders/cr3.rs).

A generic metadata-only crop cannot reproduce this particular LibRaw
compatibility-table choice. No camera hardcode, silent crop change, or fabricated
dimension parity was introduced. Closing RAW parity needs a qualified safe
LibRaw-facing boundary, or an explicitly reviewed general compatibility design
plus rendered-image parity tests. Demosaic, WB, tone/color defaults, all supported
cameras and all 50 benchmark frames were not compared here.

## Reproduction and remaining gate

Follow [`../../qualification/p0-codecs/README.md`](../../qualification/p0-codecs/README.md).
Generated files and local logs stay ignored; committed
[`p0-codec-evidence.json`](p0-codec-evidence.json) records output hashes and
measured facts. No new download of source photographs is required.

P0.08 remains open for RAW rendered parity; grayscale/alpha and all orientations;
mixed dimensions and Unicode/non-BMP paths; oversized input/resource limits and
atomic failed-export retention; complete transfer/profile behavior and native
platform packaging. These are required conditions, not deferred passes. The
standalone test probe has no application coverage/mutation qualification and
does not change first-party assurance policy.
