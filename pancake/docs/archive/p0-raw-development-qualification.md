# Adopted P0 RAW/I/O minimum

## Initial RAW policy

Read source bytes into an owned `Arc<Vec<u8>>`, construct
`RawSource::new_from_shared_vec`, and call `rawler::decode` with default
`RawDecodeParams`. Run `RawDevelop::default().develop_intermediate` and
`Intermediate::to_dynamic_image`. Require full RGB16 output; thumbnail or sensor
metadata parsing is not a substitute.

Freeze Rawler 0.8.0's defaults: black/white rescaling, PPG demosaic for ordinary
RGB Bayer, applicable Fuji normalization, decoder-recommended active/default
crop, metadata white balance and color calibration, then sRGB transfer. The
qualified output representation is full-scale, gamma-encoded sRGB RGB16. Do not
silently switch to linear RGB or apply histogram auto-brightening. ICC conversion
remains explicit: validated byte-identical profiles pass samples through;
different profiles use the already-qualified safe Little CMS path.

Do not add camera-specific crop constants. The known Canon CR3 difference is
retained: Rawler recommends 5088×3392 from the file while LibRaw's compatibility
table exposes 5118×3412. Complete camera rendering/WB/demosaic/crop equivalence
and real T50 photograph comparison belong to P2 before comparison under the
adopted plan. The full camera/sensor/compression/orientation matrix is not
qualified by this minimum.

Rawler remains third-party code with internal unsafe/SIMD. Application and probe
Rust introduce no unsafe. The earlier LibRaw binding review remains valid;
this selection avoids adding an unqualified application FFI boundary.

## Executed full-development proof

| Check | Observed result |
| --- | --- |
| Input | Unchanged `generate_fixture("bayer_dng", …)`, generator version 2. Actual uncompressed RGGB DNG; three generated inputs, each SHA-256 `99051b2e08724bdf6aeb8c27a4c8aa495143d9ffa658d826c7ab089078c66817`. No embedded thumbnail route is used. |
| Complete Rust path | Owned bytes → Rawler sensor decode → default rescale/demosaic/WB/calibration/crop/sRGB development → `DynamicImage::ImageRgb16` → TIFF export. No application-shipping RAW logic added. |
| Unchanged validator | `[112,160,3]`, `uint16`, range **65535**, genuine-16-bit checks pass. Required range is 10000. |
| Independent libtiff read | **28,206** unique u16 values; **52,723** samples are not multiples of 257. This excludes an 8-bit promoted output. |
| Independent negative control | A separate TIFF with the same shape/dtype/range but samples quantized to 8-bit promotion is rejected by the unchanged validator with `Image lost 16-bit precision`. |
| Output identity | TIFF SHA-256 `1b7afc3b05d0cde5ee61b4ce2de7c3363cce19a923bc7e68b83dcaaa10ebda6c`; little-endian pixel SHA-256 `523d4b0e2c883f6661b82d6b269c832e1340001f13040abedbac9142ac14f396`. |

The independent output test was run first and failed with
`Complete Bayer-to-RGB16 output is missing; sensor parsing is insufficient`.
It passes after the actual full-development test produces output. No precision
tolerance, fixture, validator or expected shape/range was weakened.

## Worker/process budget

One process-wide `LazyLock` creates one local Rayon pool with **two named RAW
workers**. All real CR3, generated DNG and full-development calls use that same
pool's `install`. Rayon documents that nested `join`, `scope` and parallel
iterators then execute in the selected pool. A broadcast observed exactly
`pancake-raw-0` and `pancake-raw-1`, each reporting two pool workers.

No global pool is configured, no global Rayon environment is mutated and no
pool is rebuilt per image or test. Rust tests ran sequentially with
`--test-threads=1`; Cargo compilation used two jobs. Independent Python checks
run as separate sequential oracle processes. This bounds RAW workers; it is not
a claim about every future driver/native-library service thread or whole-job
memory usage. The probe does not add a competing Rayon numerical implementation.

## Retained proofs and combined graph

All three Rust tests pass with the same strict Rust/Clippy lint tables as the
current application. Existing independent checks still pass: complete real CR3
sensor equality with LibRaw; PNG16 and ZIP TIFF16 identity; float TIFF/EXR bit
identity including `0x3f000001`; exhaustive equal-profile u16 passthrough; and a
real ICC transform matching system Little CMS exactly. These are separate from
full RAW camera-render parity.

A bounded **metadata-only** combined probe resolves 839 packages using the real
Pancake path dependency, selected codec/RAW/color dependencies and all eight
reviewed root vendor overrides. It retains CubeCL 0.11.4, Slint 1.19.0 and exactly
one wgpu family, wgpu 30.0.2. No package named `paste` or `pin-utils` appears.
The manifest and complete lock snapshot are committed next to the probe.
Production dependencies were not added; the P2 application integration must run
the enforced policy against its actual adopted graph. Combined build, actual
decode→CubeCL blur→export, and combined assurance/performance are not claimed.

Disk free was 49 GiB initially and 42 GiB after normal hooks; the
required 15 GiB reserve was preserved. The existing codec target was reused.
No photograph download, personal-photo scan or guest boot occurred. Normal
commit hooks passed app fast checks and debug/release native tests, including
small CPU/Metal blur checks. Full aggregate application assurance remains owned
by the separate assurance/integration stream; no competing instrumented cache
was created for this I/O worktree.

Reproduction: [`../../qualification/p0-codecs/README.md`](../../qualification/p0-codecs/README.md).
Machine-readable evidence: [`p0-raw-development-evidence.json`](p0-raw-development-evidence.json).
The implementation remains test-only qualification code; no coverage exclusion
or untested first-party shipping logic was introduced.
