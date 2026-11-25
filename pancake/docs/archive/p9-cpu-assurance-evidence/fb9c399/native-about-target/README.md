# Native About through a normal CPU integration target

The clean product reference `fb9c399a96658331cb76b843a1e565b503e94fdd`
has the same Pancake code and lock as `c1e2cac`. Its unchanged stable
`coverage-cpu` run passed 146 tests and exited 1 on uncovered CPU lines.
The original mapped report and indexed profile are separate.
[Baseline output](standalone/coverage-cpu.log.gz) [Baseline map](standalone/standalone.json.gz)

The unchanged exporter automatically collected the new counter file.
Its named mappings, previously covered regions, files and denominators
matched the before-execution control. The added authored line hits
were 152 in session, 22 in main, 18 in pointer handling, nine in
preferences and four in the panel boundary. The strict comparator
still exited 1 on uncovered lines.

| Metric | Before About | With About | Gain |
|---|---:|---:|---:|
| Lines | 4,658 / 6,475 | 4,863 / 6,475 | 205 |
| Functions | 476 / 756 | 498 / 756 | 22 |
| Regions | 4,776 / 7,288 | 4,986 / 7,288 | 210 |

The proof adds one real CPU application witness through the normal test
and export mechanism. The original standalone gate result and the
separate optional native coverage cohort remain recorded independently.
The test-only branch is retained locally for review of runtime
provisioning before any integration into CI.
[Actual exporter commands](live/live-export.log.gz) [Live map](live/live-map.json.gz)
[Strict verdict](live/ordinary-comparator.status)
