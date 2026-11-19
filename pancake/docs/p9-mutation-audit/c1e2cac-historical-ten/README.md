# Historical ten-operation CPU mutation replay

The original accepted list came from the `287828a` subpartition `255/256`.
Every original matched one current operation by function bytes, function-relative
source span, package, replacement and deleted or inserted diff lines.
Two registration kernel positions moved down one line. Independent filtered
listing and the native producer both emitted the ten matched rows in the same
order. The first preflight's regex-only form listed 21 candidates. The
combined file and exact-name selection emitted the verified ten. The current
full roster has 2,642 rows. The other 2,632 rows await this native replay.
[Operation matching receipt](match-receipt.json) ·
[Original native result](../287828a-255-256/README.md).

The four earlier unresolved operations now produced concrete native failures.

| Earlier outcome | Native catch at `c1e2cac` |
| --- | --- |
| Panorama cap arithmetic survived | The public panorama test rejected the mutant's working budget expansion. |
| Forced empty registration read timed out | Bounded warp processing returned a shape diagnostic in the registration geometry and partial-tile tests. |
| Desktop reveal accepted a nonexistent source | The missing-source reveal test rejected the mutant's reported success. |
| Native GTK builder returned default widgets | The isolated no-display child failed when the mutant attempted GTK widget initialization without a display. |

Rust 1.100.0 and cargo-mutants 27.1.0 used the same full-workspace,
all-features build, unchanged 16 classified CPU integration targets,
library and binary suites, and original per-variant deadlines of 180
seconds for build and 60 seconds for tests. The local replay used two jobs
on CPUs 6 and 7 of the Linux AMD Ryzen 9 7950X3D host. It took 10m 21.57s,
including a 395.75-second cold baseline build and 6.63-second baseline test
run. The working target occupied 7,331,131,392 allocated bytes after the
run. The full P9 mutation gate remains open.
[Replay command](run-historical-ten.sh.txt) · [Native reconciliation](reconcile-native.py.txt).

[Next complete CPU shard projection](full-shard-projection.md).
