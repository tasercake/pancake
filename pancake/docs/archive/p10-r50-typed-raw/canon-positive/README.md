# Positive Canon CR3 parity

Two unmodified Johannes Sood Canon CR3 originals, `frame_000.CR3` and
`frame_049.CR3`, were downloaded as individual objects from the frozen
CC BY 4.0 source revision. All 17,682,772 bytes matched their frozen
hashes before and after the replay. Independent ExifTool MakerNotes
confirm both images have normal orientation and recommend a
5088×3392 RGB crop from the 5376×3574 sensor at `(272,172)`.
[Exact input and source rights](acquisition.json)
[Independent camera geometry](source-geometry.json)
[Source preserved](inputs-after.txt)

The two direct, offline locked Rust 1.100.0 release builds used
identical product settings. The stock CLI from exact `e81079d`
has SHA-256 `2d3ff9afaa17d73233fd2ef2ef635f999b53186b608f412dcf8ebd367d458c55`.
The candidate CLI rebuilt at exact `c80d59a` has SHA-256
`f17bc8241f776fafed9f01be42835a65293480eabb221adef6c21ae23322452d`.
Both are locked headless native builds. The first attempted candidate
build reused the stock ELF and helper in the shared private Cargo
target. Its duplicate run was discarded. Cleaning both app packages
in that target forced actual recompilation and distinct executable
hashes. [Stock provenance](stock/provenance.txt)
[Candidate provenance](candidate/provenance.txt)
[Unqualified build receipt](unqualified-build/build.log)
[Package cache correction](force-candidate-rebuild.log)
[Exact compiled source difference](compiled-delta.txt)
[Read-only CLI copies](binaries.json)

The stock image and stack checks ran first. The candidate reproduced
every one of the 103,550,976 native working f32 samples across both
sources. The two per-original source-grid 5088×3392 RGB16 TIFF files
matched byte for byte. An independent TIFF decoder validated genuine
uint16 depth and checked that every decoded RGB16 code reconstructed
its corresponding working sample bit for bit. The complete 5086×3390
Hybrid RGB16 stack TIFF also matched byte for byte. Both actual CLI
completions report the explicit CPU runtime without fallback. Source
order and every alignment and sequential f32 matrix bit matched. The
stack and ordered working-bit stream also matched archived hashes.
[Independent stock results](verify-stock.json)
[Exact parity and all output digests](verify-pair.json)
[Stock completion](stock/events.jsonl)
[Candidate completion](candidate/events.jsonl)

The builds and replay used logical CPUs 14 and 15 on Linux x86-64,
a Ryzen 9 7950X3D and a 32 GiB child address-space cap for the image
and stack checks. That affinity exercises the one-worker RAW packing
path. The earlier test-first DNG and float observer evidence is
recorded separately. These two real Canon originals establish positive
format parity on the serial path. Parallel RAW, 50-frame wall time,
frozen warm/cold and memory assurance, and independent specimen
quality remain open. No GPU or performance run was performed.
[Host](host.txt) [Test-first DNG evidence](../README.md)
[Full replay plan](../replay-plan.md)

[Build command](build.sh.txt) [Native source-grid probe](canon_native_probe.rs.txt)
[Correctness commands](run-correctness.sh.txt)
[Independent verifier](verify-canon.py.txt)

Both positive Canon probe/stack scratch trees and their build cache were
removed after checksumming the ten qualified output files. At parent
authorization the same two originals moved into the full R50 stage.
[Owned subset scratch cleanup](subset-scratch-cleanup.json)
[Subsequent complete diagnostic](../full-r50-diagnostic/README.md)
