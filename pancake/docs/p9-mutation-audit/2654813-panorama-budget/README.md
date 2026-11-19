# Panoramic registration budget mutation regression

Test-only revision `26548131bd44d798816ebcfa7690d4eee03501eb` adds a public
`registration::thumbnail` oracle. A legal 2×2 RGB source uses an 8192×160
reference canvas. The area cap yields an expected 3686×72 luma proxy. The
independently calculated working budget is 4,246,496 bytes. The native CPU
test succeeded at that budget and rejected one byte less.

The original division-to-multiplication row survived at `287828a`. Pinned
cargo-mutants 27.1.0 independently listed the same row byte for byte with
the new test. All 143 unmutated tests passed across 19 targets, with the
unchanged 16 classified CPU integration targets plus libraries and
binaries. The exact mutant compiled, then the panorama test failed with
`kernel working-data budget exceeded` at the expected accepted budget.
Cargo-mutants caught the variant and exited zero.

The replay used Rust 1.100.0 and two jobs on CPUs 20 and 21 of the Linux
Ryzen 9 7950X3D host. It took 5m 19.20s, including the cold baseline.
Production source and the Cargo lock were unchanged by the regression.
The earlier shard retains its three misses and one timeout as observed.
The complete P9 mutation gate remains open.

[Receipt](receipt.json) · [Raw outcomes and logs](raw-native.tar.gz) ·
[Exact replay](replay-panorama-mutant.sh.txt).
