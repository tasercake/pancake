# Verified R50 RGB16 packing candidate

The locked release CLI at `f2f1bb3bdbdffa2ee360eb9fd3194aa26e87ec34`
has SHA-256
`231b263a9ce0a9d00bffdd609ffd998489020e640573aad12bebf6a80b7174d5`
and ELF build ID `a15841e5496764993f18f852236dccfa42f9fe75`.
The comparison CLI was the sealed `12f2151` Debian payload.
The baseline public decoder probe used `51298f3`, whose CLI and core
sources, manifests and lock are byte-equivalent to that payload's source.
The two probe builds used pinned Rust 1.100.0 and the same release profile.
The verification ran after the official R50 window. [Builds](baseline-build.txt)
[Candidate build](candidate-build.txt) [Sealed release](sealed-release-provenance.json)

## Byte equality

The first and last real Canon originals matched their frozen hashes
before and after the investigation. Both actual CPU Hybrid commands
produced byte-identical 5086×3390 unsigned RGB16 TIFFs with SHA-256
`d85703c50ccba5ae21303f93ba8eea590d5eb41554c82accd7e50605c04993e4`.
Both public-decoder probe outputs also matched with `cmp` over all
414,203,904 bytes of the two full 5088×3392 working float32 frames.
The combined sample digest was
`ed3351ff2b8f5f1f3b278c806258170d6a2b89b75da55014f5dc875535c8a0b2`.
[Input custody](ownership.json) [Commands](parity/commands.txt)
[TIFF metadata](parity/tiff-metadata.txt)

The existing generated Bayer DNG and its valid metadata-derived
horizontal-mirror variant produced identical full working float32
and weighted-average RGB16 TIFF bytes between the releases.
ExifTool read 160×112 unsigned RGB16 for both TIFF files. The existing
image I/O integration test target passed 24 of 24. Both new packed RGB
unit tests passed, including all eight EXIF orientations, truncation
and saturation. Stable core Clippy passed with `-D warnings`, the
reviewed effective dependency policy, and two build jobs.
[DNG hashes](parity/output-sha256.txt) [DNG TIFF metadata](parity/dng-tiff-metadata.txt) [Image I/O tests](cpu-image-io.log.gz)
[Packing tests](packing-tests.log.gz) [Stable lint](stable-clippy.log.gz)

## Bounded RAW diagnostics

Four alternating two-file pairs on the common CPU mask 0–14 measured
the direct public decoder. The median sum of its two frame durations
was 2.0493 s for the original packing path and 1.8849 s for the
candidate, an 8.0% reduction. Median process high-water RSS across
those runs was 656,216 KiB and 614,030 KiB respectively. These
measurements describe only the two-file decoder diagnostic.
[All copy-removal samples](parity/copy-results.json)

The separate temporary test measured the exact candidate's
`raw::develop` on the same inputs after staging the encoded bytes.
The measured affinity allowed 15 CPUs. A dedicated two-worker pool
and a dedicated 15-worker pool both returned bytes identical to the
production public decode. Across four alternating pairs, their
median two-frame stage times were 1.8589 s and 1.0437 s. The production
RAW pool is still two workers. The wider-pool timing supplies
diagnostic evidence for future work. [Pool samples](pool/summary.json)
[Per-frame hashes](pool/output-sha256.txt)
[Exact diagnostic source](thread-probe.rs.txt)

## Frozen P2 regression
