# Exact stock and typed RAW R50 CPU diagnostic

One serial stock/candidate pair ran on the Ryzen 9 7950X3D in a
dedicated CPU window on 2026-11-19 SGT. The common affinity selected
15 different physical cores at logical CPUs 0–14, with one core
outside the RAW worker pool. Both processes used fresh app caches and
the same pre-read input file cache, with a 32 GiB child address-space
cap. The runner measured spawn through actual exit including final TIFF
export and retained Linux `wait4` direct-child peak RSS. Both completed
50 ordered frame steps and genuine CPU execution with no fallback.
[Quiet window census](quiet-lease.txt.gz) [Exact command](command)
[Terminal status](full-after.txt) [Stock raw run](stock/process.json.gz)
[Candidate raw run](candidate/process.json.gz)

| Arm | Wall time | Direct-child RSS peak |
| --- | ---: | ---: |
| Stock `e81079d` | 35.642573 s | 2,236,637,184 B |
| Typed RAW `c80d59a` | 34.944663 s | 2,255,990,784 B |

The corpus was exactly 50 unmodified, filename-ordered, pinned Canon
CR3 originals. The existing two verified frames were moved into their
manifest positions without duplication. The other 48 original
objects contributed 424,416,224 B for 442,098,996 B total. All lengths
and hashes were verified both before and after this pair, and both
stock and candidate ELF digests remained unchanged. The capture and
CC BY 4.0 rights are bound to the frozen acquisition record.
[Full source and ordered input receipts](full-acquisition.json)
