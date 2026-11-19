# Native depth-map box-normalization mutation replay

The archived `0a9ae85` shard 9-of-16 missed the mutation at
`src/fusion/focus.rs:428:19` that inverts the predicate guarding box-filter
normalization. The applied patch matches the shard's saved diff and JSON
entry byte for byte. [Original outcome](archived-outcome.json) · [Exact patch](archived-mutant.diff.gz)

The test pairs 13-pixel gray images. One has a centered `0.375` impulse on
a `0.125` background. The other keeps that background at the center and
has the same impulse at indices 2 and 10. This gives each isolated impulse
one unit of modified-Laplacian energy. At index 6, the centered impulse is
inside all three box windows (5, 3 and 11), while the two outer impulses
contribute energy only to the 11-pixel window. Normalized scores are
`1/5 + 1/3 + 1/11` and `2/11`. With division disabled, the one-pixel axis
repeats the sum once per tap, and the scores are `5 + 3 + 11 = 19` and
`2 × 11 = 22`. This reverses the selected source. Both the `13 × 1` row and
its `1 × 13` transpose independently exercise the public CPU accumulator.

| Native CPU `cpu_fusion_contract` arm | Pass | Fail | Exit | Center source |
| --- | ---: | ---: | ---: | --- |
| Exact archived mutant | 3 | 1 | 101 | `0.125` in both orientations |
| Restored `0a9ae85` kernel | 4 | 0 | 0 | `0.375` in both orientations |

The final oracle checks copied pixel bits directly. An earlier version
produced the same runtime outcomes and Clippy rejected its direct
float-array comparison. Both arms ran again on the final test source.
Reviewed-policy Clippy, Rust format, Git whitespace and the source
inventory passed. This result covers the targeted native CPU test.
[Mutant output](red-full-target.log.gz) · [Restored output](green.log.gz) · [Clippy](green-clippy.log)

The replay used test revision `209207a5a74977ad3e5ff2e03248abecaa706393` over
`0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`, Rust 1.100.0, the frozen lockfile,
and two direct offline jobs on CPUs 14 and 15. The production kernel
matched the original digest again before the green arm.
[Environment](environment.txt) · [Replay](replay.sh.txt) · [Checksums](SHA256SUMS)
