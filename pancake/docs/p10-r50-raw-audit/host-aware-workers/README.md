# R50 RAW worker and storage diagnostics

The sealed Linux baseline was `d6359e1`, with the same CLI and shared-core
sources and lock as this audit's starting commit `2143eda`. It used two
fixed Rawler workers. The first candidate at `e8e1549` sizes one process-wide
pool from Rust's available CPU count, leaving one CPU's worth of headroom
where possible and clamping to 1–15 workers. It uses one worker when the count
cannot be determined. In the 15-CPU measurement affinity this selects 14.
The existing stack scheduler still decodes one frame at a time.
[Sealed binary check](sealed-checksums.txt) [Candidate build](workers-build.txt)

The second candidate at `b10fbec` also retains Rawler's owned developed RGB
allocation for normal or unspecified EXIF orientation. It uses the same
Rawler integer quantizer in chunks of up to 65,536 samples, holding at most
128 KiB of temporary code values, and divides each code by 65,535. Working
samples still carry the U16 precision marker. Transformed orientations
keep the original ordered RGB16 conversion and orientation path.
[Candidate build](reuse-build.txt)

## Verified Canon output

Each arm decoded the identical first and last frozen R50 Canon originals.
For all three, the 5088×3392 full developed RGB working arrays matched
bit for bit, totaling 414,203,904 bytes with SHA-256
`ed3351ff2b8f5f1f3b278c806258170d6a2b89b75da55014f5dc875535c8a0b2`.
All three CLIs completed the same CPU Hybrid job with identity transforms,
auto-crop and 16-bit export. The independently inspected unsigned RGB16
5086×3390 TIFF files also were byte-identical, SHA-256
`d85703c50ccba5ae21303f93ba8eea590d5eb41554c82accd7e50605c04993e4`.
The input hashes matched before and after use. Both originals were removed.
[Acquisition](acquisition.json) [Actual commands](canon/commands.txt)
[TIFF metadata](canon/tiff-metadata.txt) [Product checksums](canon/products.txt)
[Input cleanup](input-cleanup.txt)

## Direct decoder timing

The pinned Rust 1.100.0 releases ran serially on an AMD Ryzen 9 7950X3D
under the same affinity, CPUs 0–14. Each timed sample was a fresh process
reading both originals in the same order after their byte-parity runs.
Four rounds rotated arm order. Reported decode time is the median
sum of the two production decoder durations. Peak RSS is the median
process high-water counter from GNU time.

| Release | Two-frame decode | Relative to baseline | Peak RSS |
|---|---:|---:|---:|
| Sealed fixed workers | 1.853 s | 1.000× | 630,312 KiB |
| Host-aware workers | 1.029 s | 0.555× | 639,476 KiB |
| Workers and allocation reuse | 0.782 s | 0.422× | 639,534 KiB |

## Boundary and regression verification

The worker-budget test failed against the original fixed-two policy.
The allocation test failed against the original converter after verifying
its working samples. The final 20 library tests passed, including
the allocation test's all-code bit comparison and the eight explicit
EXIF orientations. All 24 shared CPU image I/O integration tests
passed, including malformed and oversized RAW cases. Exact locked Clippy
passed with the reviewed dependency configuration.
[Worker red](worker-test-red.log.gz) [Allocation red](packing-test-red.log.gz)
[Final tests](test.log.gz) [Clippy](clippy.log.gz)

The frozen generated Bayer DNG and its valid horizontal-mirror variant
also matched decoded working values and 160×112 unsigned RGB16 export
bytes in all three builds. The Linux process trace saw exactly
1, 1, 7, 14 and 15 named RAW workers under 1, 2, 8, 15 and 32 allowed
CPUs with `RAYON_NUM_THREADS=128`. The repeated DNG decodes returned the
same samples under all five masks.
[DNG checksums](dng/output-sha256.txt) [DNG TIFF metadata](dng/tiff-metadata.txt)
[Observed worker counts](affinity/summary.tsv) [Trace command](affinity/replay.sh.txt)

## Repeated decode memory

A separate untimed process decoded the two reauthenticated originals eight
times each. It released each full working frame before decoding the next.
All sixteen complete working-sample fingerprints matched the corresponding
sealed baseline byte stream. Process RSS came from Linux `smaps_rollup`
immediately after decode and drop. The process peak was 678,124 KiB.
Post-drop RSS reached 163,000 KiB on the eighth decode and stayed within
44 KiB of that reading through the sixteenth. Host build and diagnostic
activity could overlap this canary. The readings describe memory in the
tested process. Both originals were rehashed and removed again.
[Exact probe](raw_repeat_probe.rs.txt) [Memory and fingerprints](repeat/repeat-observations.tsv)
[Baselines](repeat/baseline-fingerprints.txt) [Reacquisition cleanup](repeat/repeat-cleanup.txt)

## Frozen P2 replay

The exact candidate `b10fbec440069602ed16e6055c6bb5f1c4212757` ran
the unchanged P2 roster from the [previous verified replay](../f2f1bb3/README.md).
All 68 CPU-only IDs and all 16 GPU-bearing IDs passed. The latter
included sixteen completed physical NVIDIA RTX 3090 Vulkan jobs and
sixteen completed CPU comparisons. The CPU-only lane had 27 completed
CPU jobs on the Ryzen host. Every completed call and backend matched
the earlier verified roster. No cases were failed, skipped, unsupported,
or unrun. Actual GPU events identify PCI address `0000:01:00.0`,
`wgpu-vulkan` and zero fallbacks.
[CPU accounting](p2/p2-cpu-summary.json) [GPU accounting](p2/p2-gpu-summary.json)
[CPU provenance](p2/p2-cpu-provenance.json) [GPU provenance](p2/p2-gpu-provenance.json)

The tested combined CLI SHA-256 is
`0c14b3344742408a20e10258f66cfce7cb698303d32a794dbe0050f8ac711122`.
Its ELF build ID is `a4f0b3d04f0677d527cc986d5f2d6987786a6be1`.
Both experimental CLIs were retained with independent checksums.
[Worker-only build](workers-build.txt) [Combined build](reuse-build.txt)
