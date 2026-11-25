# Integrated Linux nightly assurance

The exact clean product revision `8694b555666a1ddf4051e2839bbced4bcc625859`
passed 22 of 22 tests in the locked supplemental Linux Miri library suite.
Both rewritten TIFF cursor and lane-policy tests ran in the actual Pancake
workspace along with the RAW and resident-pixel ownership tests. The
execution used pinned `nightly-2026-11-15` and the existing strict-provenance
and symbolic-alignment flags with isolation enabled. Exit status was 0.
[Raw Miri log](miri.log.gz) [Reviewed-source check](reviewed-source.log)

The unchanged `bash scripts/check.sh layouts-cpu` gate ran both pinned
seeds, `1` and `42`. Each passed 142 tests in all 19 selected executable
groups, with zero failures, ignored tests or filters. The native large TIFF
case preserved ICC data and rejected a later truncated tile in both runs.
The gate exited 0. The shell trace identifies the unchanged Cargo
arguments and seed boundaries.
[Raw layout log](layouts.log.gz) [Accounting](receipt.json)
[Native TIFF results](tiff-native-tests.txt)

Both checks ran serially on Linux x86-64 and an AMD Ryzen 9 7950X3D, using
only CPUs 0 and 1, two Cargo build jobs and separate fresh targets.
GNU time measured 3m42.12s for the supplemental Miri command and
7m46.02s for the two-seed layout gate. The canonical Darwin Miri job
and sanitizer release lanes retain their separate requirements.
[Command](command.txt) [Environment](environment.txt)
[Miri process status](miri.time) [Layout process status](layouts.time)

The pre-cleanup native manifest records SHA-256, size and ELF build ID
for the 19 test executables used by each seed.
[Native test executable identities](native-artifacts.json)
