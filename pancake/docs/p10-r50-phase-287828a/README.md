# R50 CPU stage diagnosis for the packaged 287828a CLI

Both forced-CPU Hybrid jobs retained the frozen R50 flags, including
first-frame similarity, kernel 6, eight pyramid levels, alignment scale 6,
auto crop, no slabs and true RGB16 export. They used Ryzen 9 7950X3D
CPU IDs 0–14, common thread settings of 15 and a 32 GiB process
address-space limit. The exclusive lease ran 19:04:48–19:07:21 SGT
on 2026-11-18. [Lease](quiet-lease.txt) [Host at start](window-start.txt)
[Host at finish](window-end.txt)

## Completed jobs and output

| Diagnostic arm | Process wall time | Kernel child high-water RSS |
| --- | ---: | ---: |
| Pancake, untraced | 50.791982 s | 2,764,169,216 B |
| Pancake, separately traced | 51.833229 s | See process receipt |

## External stage diagnosis

An external GDB probe recorded entry and return addresses from the
packaged ELF. The inferior stayed on CPUs 0–14 and GDB used CPU 15.
The trace retained inferior CPU ticks from Linux procfs at each
boundary. It took 1.041 s longer than the untraced candidate.
[Stage summary](stages.json) [Exact probe](external-stages.py.txt)

| Pancake stage | Complete calls | Total traced wall time | Mean busy CPU cores |
| --- | ---: | ---: | ---: |
| Full RAW decode | 50 | 18.087 s | 4.40 |
| Registration thumbnail | 50 | 17.206 s | 1.40 |
| Registration transform estimate | 49 | 0.420 s | 9.26 |
| Hybrid frame fusion | 50 | 14.340 s | 13.90 |
| Final fusion and readback | 1 | 0.294 s | 7.88 |
| TIFF staging | 1 | 0.226 s | 0.93 |

GDB emitted later return callbacks at reused call sites because a
temporary breakpoint callback returning false stayed enabled. The
initial reducer reported overlapping intervals and made the wrapper
exit 1 after every job had exited successfully. Offline reduction
used the first chronological return for each exact stage entry,
preserved all 4,954 raw callbacks, and verified 201 expected
nonoverlapping visits. [Reducer](summarize-stages.py.txt)

Thumbnail time includes native RGB staging, luma sampling, and
completed thumbnail readback. The measured stage alone does not
identify the time of each operation. The unrelated transform
estimator uses 0.420 s across all 49 comparisons. The isolated
`07816a2` candidate changes only the registration upload to one
aligned owned buffer at the same thumbnail, warp and scoring
callsites. A same-bytes R50 differential can establish its CPU
effect. [Registration upload and thumbnail source](../../src/registration/kernels.rs)

## Replay

Use the packaged ELF, exact clean source, pinned reference runtime and
verified originals in a fresh task-owned directory. Copy the four
retained scripts without their final `.txt` suffix into that directory,
with the build receipt adjusted for the verified executable path.
Acquire the exclusive CPU slot before issuing the diagnostic command.

```sh
bash "$scratch/run-diagnostic.sh" "$scratch"
```

The separate owned-upload comparison reused the authenticated originals
and then cleaned them and every TIFF. Retained evidence contains receipts
and digests for all exports. [Owned-upload comparison](owned-upload/README.md)
