# T50 TIFF row-worker diagnostic

Eligible DEFLATE RGB16 TIFFs of at least 1,048,576 pixels can decode in up to four row bands. The caller handles one band. A process-wide lease allows up to three extra scoped workers, with a smaller count when the host offers fewer CPUs or useful tile rows. Every lane reads from the original opened file using a separate logical cursor. Only seek and compressed input reads share a lock. Each decoder verifies its directory against the original band plan. Decoded tile scratch is bounded at 24 MiB per parallel import, in addition to the shared working raster.

The first matched Linux CPU diagnostic used candidate `3529ea4` and baseline `287828a` in the same optimized probe on four physical Ryzen 9 7950X3D cores. It authenticated the three generated T50 inputs before and after use and compared all 648 million candidate working samples bit for bit. Other processes ran on the host outside the selected cores.

| Decode-only metric | Serial | Row workers |
| --- | ---: | ---: |
| Median of nine authentic T50 frame decodes | 0.471289 s | 0.140305 s |
| Frame 024 peak process RSS | 285,440 KiB | 287,664 KiB |
| One-core frame 024 median of three decodes | 0.521138 s | 0.520237 s |

The four-core candidate reached 3.36× serial throughput. Linux observed three extra TIFF worker threads. There were zero extra threads on one core, and another 216 million candidate samples matched the baseline. [Receipt](receipt.json) · [Paired samples](alternating.tsv) · [One-core samples](one-core-parity.tsv) · [Live workers](threads-rows.json) · [Source archive](probe-source.tar.gz)

The test-first witness failed when the seek was omitted inside the shared input lock, returning the wrong tile bytes. The follow-up production revision `1a88701` retains this test on an in-memory cursor and exercises the real filesystem TIFF path in the native integration lane. The independently encoded 1025×1027 fixture preserves every normalized code, partial edge pixels and its ICC profile. A one-byte compressed length in the last tile row reports the source path and tile 288. The native fixture is little-endian contiguous RGB16 with 64×64 tiles, horizontal predictor and the row-major sample code sequence `index % 65536`. [Exact pure red log](cursor-pure-red.log.gz) · [Pure tests](miri-boundary-unit.log.gz) · [Native regressions](miri-boundary-integration.log.gz) · [Strict Clippy](clippy-miri-boundary.log.gz)

Independent review at `a5cd3ce57516a0b80aa10fa26f521a1240539dfa` checked the worker source and passed both exact copied pure tests with pinned Linux Miri isolation. Its command, logs and source hashes are under `pancake/docs/p10-tiff-row-workers-review`. The directory checks anchor each worker to its validated band. An external in-place write can still change bytes after that check, so both diagnostics verified source hashes before and after the read-only use.

## Locked CPU and RTX P2 replay

## Matched physical T50 pair

The sealed CLI from the `287828a` Debian package and the exact `e228c68` release used one Rust dependency lock, one frozen Hybrid profile and the same resident-input, registration, fusion, and export source. Each processed the same 50 ordered 6000×4000 RGB16 inputs on the physical RTX 3090 with driver `580.178.04`, 32 allowed logical CPUs and a 28 GiB no-swap host scope. Both completed with zero fallback and with all 50 exact identity transforms. The 3998×5998×3 uint16 exports had the same encoded SHA-256 `6160722086e2153b4dd3d8e3810dd22e156669fcafaa15fc72a3e88968b39302` and decoded hash `f54746784bcc278da2df734e9f034053a3b54105ddb2a30046baf6dab07a80a3`. All 52 authenticated source and truth files matched again after the pair. [Source lineage](physical/source-lineage.json) · [Summary](physical/summary.json) · [Raw report and commands](physical/report.json)

| Diagnostic measurement | Serial | Row workers |
| --- | ---: | ---: |
| Wall through export and exit | 55.539127 s | 38.517180 s |
| Observed process-tree peak RSS | 1,549,021,184 B | 1,549,971,456 B |
| Maximum RSS sample gap | 10.672 ms | 9.876 ms |
| Observed attributable VRAM peak | 2,518,323,200 B | 2,518,323,200 B |
| Maximum GPU sample gap | 93.134 ms | 116.456 ms |

Wall time fell by 30.65% in this sequential pair. Each arm had a fresh application cache. Initial GPU state differed, with P8 at 56 °C before the baseline and P5 at 58 °C before the candidate. The separate compiler-cache cleanup log closes at 19:13:54.890793 SGT. The combined measured jobs and wrapper end timestamp put the latest possible serial launch at 19:13:52.944 SGT, confirming overlap between the cleanup window and the serial arm. The timing bound gives no estimate of the cleanup's effect on wall time. The cleanup preceded the worker arm. Both GPU sampling gaps exceed the frozen 20 ms limit, so formal VRAM and P10 performance qualification remain open. [Cleanup cross-receipt](physical/cleanup-overlap.json) · [Host census](physical/full-t50-census.txt) · [Serial samples](physical/serial/sample.json) · [Worker samples](physical/rows/sample.json) · [Evidence checksums](physical/SHA256SUMS)

The source archive captures the exact two TIFF decoders and build lock used in the decode-only diagnostic. The reproduction script records the materialization receipt hash and selected affinity. [Probe commands](matched.sh.txt) · [Hardware and background processes](match-environment.txt) · [Input hashes](input-sha256.txt)
