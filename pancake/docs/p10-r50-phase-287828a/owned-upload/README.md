# R50 CPU comparison for the owned registration upload

The exact `287828a` Debian CLI and independently qualified `07816a2`
headless CLI ran the same 50 original Canon CR3s and frozen Hybrid CPU
settings on Ryzen 9 7950X3D CPUs 0–14 with common thread settings and a
32 GiB address-space cap. The package used a locked workspace release
build. The owned-upload binary used a locked headless release and passed
73 P5/P6 device checks. Its only production source change is the shared
registration upload. Source and executable bytes were reverified after
the jobs. [Package](../build.json) [Candidate](owned-binary-provenance.json)
[Acquisition](../acquisition.json.gz)

Both programs completed on CPU. All 50 source-order alignment and
sequential matrices matched in f32 bits. The TIFFs matched encoded bytes
and all 51,724,620 decoded RGB16 samples at 5086×3390, repeating the
previous candidate output exactly. This comparison covers the exposed
transforms and TIFF. [Output digests](paired-summary.json)
[Transform digests](owned-transform-parity.json)

| Diagnostic | Packaged 287828a | Owned upload 07816a2 |
| --- | ---: | ---: |
| Completed wall time | 51.520891 s | 42.072283 s |
| First complete frame | 1.556089 s | 1.549576 s |
| Frames 2–50 | 49.303701 s | 39.865557 s |
| Kernel child high-water RSS | 2,763,558,912 B | 2,465,210,368 B |

The owned-upload arm used 18.34% less wall time and 10.80% less kernel
child RSS in this pair. The two launches used the existing job
completion event. Frozen timing medians and process-tree RSS remain
unqualified. The quiet window ran 19:19:28–19:22:24 SGT on 2026-11-18.
[Lease](owned-quiet-lease.txt) [Terminal state](owned-window-end.txt)

| Traced stage | Packaged 287828a | Owned upload 07816a2 |
| --- | ---: | ---: |
| RAW decode | 18.087 s | 17.808 s |
| Registration thumbnail | 17.206 s | 7.944 s |
| Transform estimate | 0.420 s | 0.401 s |
| Hybrid frame fusion | 14.340 s | 14.219 s |

Use the parent receipt to stage the exact baseline, pinned reference
environment and originals. Copy the retained scripts without their
final `.txt` suffix, including the parent GDB script. Set
`owned_receipt` to the verified candidate binary provenance. With a
quiet lease, run the CPU pair and the separate external observation.

All staged originals, six TIFFs, and run caches were removed after
verifying their recorded hashes. [Cleanup](cleanup.json)
