# Linux CPU mutation shard preflight at 0a9ae85

| Artifact | SHA-256 |
| --- | --- |
| Unfiltered full list | `244e27acf58ec257e6a652186f8dabcc2c1d7ec3ff4498c126e1c21879e0f617` |
| Independent native shard list | `b5bdf8ee8bf796db18ce5cdcadb443bc230eb2e6116789310b846cf3f77d4492` |
| Ordered shard identities | `53afec24096a00badb893884b7a2736737d50026e73f8e2397b117777de5ce07` |
| Full set of 16 CPU integration target names | `d1180f38b292c12c077049872a7c8ce68a7517a643fcaecd71b07acdc7878f9e` |

The 2026-11-19 local preflight used Linux 6.8.0-147-generic x86_64, an
AMD Ryzen 9 7950X3D, separate physical cores exposed as CPUs 4 and 5,
Rust 1.100.0 and cargo-mutants 27.1.0. Both lists were offline,
read-only and capped at two jobs without a compiler wrapper. The
unfiltered and selected producers took 0.37s and 0.15s respectively.
The archive contains their argv, raw lists, timing, source status,
locked Cargo target classification, tool digests and checked selection.
The source remained clean. The preflight created zero native outcomes
and zero build targets. No device, native test, build or network ran.

Reweighting the measured historical variant times by this actual
partition, with the earlier 402.38s baseline, projects **68m 38s**
using means and **73m 41s** using sampled maxima. The prior ten-row
attempt used 7.33 GB of allocated target storage at completion.
Extrapolating its uncompressed raw variant logs and retaining the
baseline log projects about **213 MB** of logs for this partition.
Peak target storage and unwrapped full-shard runtime remain unknown.
Earlier timings used a registry-only compiler wrapper. Allow for longer
provisioning or compilation when reserving the native slot, and measure
actual disk use. [Historical timing and storage observations](../c1e2cac-historical-ten/full-shard-projection.md).

The clean execution branch `codex/pancake-native-mutation-0a9ae85` stays
at the selected source. Once the authorized CPU slot is available,
use a new attempt output path and target. Rerun the same two list
commands and compare both digests and selectors before the full
physical native run. Use the final command in the reviewed
`7d20af1` recipe at this source and its verified `9/16` partition,
with the original `180s` build and `60s` test settings. Preserve
terminal source status, actual producer and log-writer exit codes, and
the authoritative native outcome and phase logs. The later native
attempt earns credit only for this one canonical partition when every
candidate reconciles. All 16 native partitions are still required for
the complete mutation gate.
