# Next complete CPU shard at the fixed revision

At clean `c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb`, choose **11/16** as
the next full native shard. Its independent listing matched rows
`full[11::16]` from the 2,642-candidate roster. Its 165 candidates include
145 core, 19 GUI and one CLI operation. Partitions 12–15 have the same
package counts. Index 11 is the first among these five tied selections.
The projection ran listing and receipt analysis only.
[Exact listing](next-shard-11-of-16.json.gz) ·
[Cost model and all partition sizes](full-shard-projection.json).

The completed native replay gives a 402.38-second cold baseline including
tests and mean build plus test costs of 22.87 seconds for eight core
mutants and 18.02 seconds for two GUI mutants. The earlier native CLI pilot
gives 10.66 seconds across six rows, including one compiler-unviable row.
Weighting those means by the actual 11/16 package counts projects **67m
52s** end to end. Using the highest observed phase total per package
projects **72m 53s**. This second figure is a scenario based on the sampled
rows. Actual survivors, timeouts, expensive rebuilds, and runner or
provisioning differences can exceed it. The earlier matrix prototype's
75-minute job limit remains unqualified at full-shard scale. The same mean
model sums to roughly 18.14 job-hours across 16 shards. The existing
10-row run ended with 7.33 GB of allocated target storage. A fresh full
shard needs its own target and measured storage use.
[Native timings](native-receipt.json) ·
[CLI pilot](../f9af478-cli/README.md).

Use the exact fixed revision, the independently verified full-shard list,
all features, all 16 classified CPU targets and the library and binary
tests. Execute the unmutated baseline and every native mutant test with
the original 180-second build and 60-second test deadlines. Retain the
actual native roster, phases, outcome JSON, exit status and raw logs.
This progress estimate leaves the complete mutation gate open.
