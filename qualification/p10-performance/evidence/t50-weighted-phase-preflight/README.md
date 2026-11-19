# Prepared exact e81079d Weighted Average CPU phase trace

## Exact scope

Use the clean `e81079d` checkout and extracted Debian CLI SHA-256
`9630572be5e637c38c604f2fcd6d7a44e31e171a5ccf17dadb1f59ba345c24f0`.
The live stage6 seal is
`1d9be3f1cd20a01083b11c10c4d17ab75a4f3b6bb7e2ebef682824e345ba649d`.
The launcher authenticates all 52 owned staged TIFF identities before
and after both jobs. The same 50 ordered originals feed each real job,
with the complete frozen Weighted Average CPU settings. The two truth
images participate in ownership checks. The actual paired pixels and truth
regions already appear in the initial diagnostic. The trace must reproduce
that pair's Pancake output in both encoded bytes and decoded RGB16 pixels.
It must also preserve the true 5998×3998 geometry, all 50 identity maps,
explicit CPU completion and a clean exit. [Stage6 provenance](../t50-stage6-e81079d/README.md)

Run one untraced stock control followed by one traced job, both with
15 worker threads, CPUs 0–14 and isolated empty case caches. GDB runs on
CPU15 and its inferior launches through the same worker affinity mask.
Reserve the sampler's usual SMT sibling CPU31 as idle. This wall comparison
has no resource poller. The first diagnostic's independent process-tree
RSS remains pending. Both jobs in the new comparison use the same
completion observer. The outer control clock measures launch through final
export and process reap. The traced outer includes GDB startup and teardown.
The separate inferior interval and callback bookkeeping time are retained.
The direct outer difference includes instrumentation and run-order effects
and should remain an observed overhead estimate. Do not subtract it from the
production wall time or treat this pair as warm or cold medians.

Expect roughly 3–4 minutes of exclusive host time with prior stock wall
as the only planning anchor. Reserve up to two 180 s child deadlines plus
validation and pre/post hashing. Grant the slot after other compiles,
archives, true timings and the scheduled stage6 viewer reader have finished.
If the viewer will use stage6 later instead, preserve the same ownership
and serialize its reader. Keep the replay and outputs in a task-owned
ignored directory. No rerun, reduced stack or fallback follows a failure.

## Measured boundaries when run

| Inclusive CPU host call | Expected visits |
| --- | ---: |
| Decode | 50 |
| Registration thumbnail, then separate transform estimate | 50 and 49 |
| Weighted accumulator add | 50 |
| Focus readback, stream crop/finalize, export prepare, export publish | One each |
| Nested original RGB staging call in thumbnail | 50 |
| Nested original RGB staging call within Weighted add | 50 |
| Weighted filter submit and its two nested aggregation submits | 50 and 100 |
| Weighted sync wait call within add | 50 |

Ordinary function returns pair by program counter, thread and caller stack
pointer using the x86-64 ABI. A tail call can close two nested intervals at
one return. Retired breakpoints are removed outside their callback. The
Weighted full-frame staging and wait use the two exact callsite pairs
inside `FocusAccumulator::add_frame`. The wait call targets `block_on` after
binding both accumulator handles. Measuring that call excludes the earlier
submission and binding work. The filter and aggregate intervals cover
submission calls; the submitted kernel work can execute asynchronously.
The synchronizing wait can include a backend staging copy or earlier
CPU work, alongside the weighted kernels. An observed wait is insufficient
to attribute the time to the reflected indexing. The summary reports
disjoint top-level totals and nested attribution alongside uncovered wall.
[Exact breakpoint resolution](elf-preflight.json) ·
[Callsite disassembly](fixed-calls-disassembly.txt) ·
[Weighted fusion source](../../../../pancake/src/fusion/focus.rs)

Use the result to decide between the two proposed source investigations.
A large verified staging budget supports investigating same-frame ownership
reuse, with finite, precise data, original warp behavior, error mapping
and shape guards intact. If accumulation or sync dominates, first establish
what the CPU worker spends in backend writes versus weighted kernels.
A tap-index change then needs exact interior/border equivalence and runtime
parity. Neither choice follows from source byte counts alone. [Existing
function boundary method](../../../../pancake/docs/p10-r50-phase-287828a/README.md#external-stage-diagnosis) ·
[Exact registration and focus dispatch](../../../../pancake/src/job/stream.rs)

## Prepared command

From the clean exact package checkout, copy the two archived scripts to a
fresh ignored diagnostics directory with their final `.txt` removed. Use its
pinned reference environment only as a standalone qualification interpreter.
The executable under GDB is the stock Pancake CLI.

The launcher takes the shared performance lock, checks exact immutable
source, package-derived executable, targets and input seals, and runs the
two child scopes serially. It retains failed attempts and stops if a child
fails, the output differs or the observed roster is ambiguous. Its watchdog
drains only the launched session. Rehash the two owned TIFF exports after all
readers finish, preserve their reports and hashes, then remove their copies
and private caches. Stage6 cleanup requires its own final borrower release.

Preparation verified the ELF digest and all 16 static breakpoint locations
with GDB 15.1 and an unstarted inferior. Four pure-Python tests check
return matching, tail nesting, nonoverlap and invalid time or counts.
The stock application, the reference application, and the image generator
all remain unlaunched in this preparation.
[Ledger tests](ledger-green.txt) · [No-inferior GDB log](elf-preflight.txt)

## Consumer schema follow-up

The first control completed successfully, then this preparation's original
validator treated the ordered sequential maps as a mapping, so it stopped
before GDB. The separate receipt preserves the failed attempt and valid
control output evidence. The current launcher consumes the real emitted
list. A new quiet-window experiment may use this corrected launcher and a
fresh output directory. [Attempt and tested correction](../t50-weighted-phase-attempt/README.md)
