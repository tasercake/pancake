# Exact Linux mutation scope and delivery blocker

## Prior executions and current schedule

| Retained run | Caught | Compiler-unviable | Missed | Timed out | Elapsed |
|---|---:|---:|---:|---:|---:|
| P0 closure `bc67159` | 79 | 0 | 0 | 0 | 7m 11s |
| P1 CLI/config `f69a639` | 85 | 3 | 0 | 0 | 4m 25s |
| P6 lifecycle `2375efa` | 17 | 0 | 0 | 1 | 3m 58s |
| P6 resume follow-up `5056563` | 1 | 0 | 0 | 0 | 42.5s |

The latest list contains 79, 85 and 18 candidates matching the first three
runs' full function bytes and relative mutation spans. Matches preserve
historical behavior evidence with its original test scope and source.
The resume mutant timed out in the 18-case run. The follow-up caught that
same source mutation with an assertion that also wakes the worker for
cleanup. Compiler-unviable cases have distinct retained Rust build logs.
[P0 closure](../../p0-close-evidence/closure.json) ·
[P1 CLI/config](../../p1-assurance-evidence/cli-config/receipt.json) ·
[P6 lifecycle and timeout follow-up](../../p6-core-resume-evidence/README.md).

The hosted assurance job has a 30-minute limit shared with provisioning,
coverage, dependencies, layout, branch coverage and guardrails. Even if
mutation consumed the whole job it could spend only 0.719 s per candidate,
including builds, baseline and test execution. Pinned cargo-mutants rejected
`--in-place --jobs 2` in listing mode. Each gate invocation therefore uses
one mutant source stream, with Cargo build parallelism inside that stream.
The retained P1 to P6 scopes averaged roughly 2.5 to 9.5 s per mutation
without their baselines. Linear scenarios at those rates span roughly
1.8–6.6 hours for 2,504 candidates. The actual full gate may take longer;
these scopes ran lighter test selections and smaller earlier sources.
The current hosted time allocation cannot credibly qualify the full gate.

## Exact partition and later CI design

The pinned tool accepted zero-based `--shard k/16 --sharding round-robin`
for `k=0…15` and rejected `16/16`. The first eight partitions held 157
candidates each and the second eight held 156. Each actual shard JSON
matched the full listing's `k::16` sequence. The complete canonical
JSON-object multiset matched all 2,504 candidates exactly, with zero
missing, duplicate, or unexpected entries. A separate rerun of shard 1
matched byte for byte. [Partition result](partition.json) ·
[Raw listing checksums](raw-listing-sha256.txt).

An auditable CI design would use a distinct mutation matrix on clean
checkouts with sufficient time for a baseline and one serial shard each.
Select shard count and job time from a timed scoped pilot and cap concurrency
to available runners. The 16-way partition is a listing proof. Every job
would capture identical source, lock, pinned tool, compiler and CPU target
identities and retain its expected shard list, `outcomes.json` and raw logs.
A required aggregate step would bind the actual baseline and mutant outcomes
to the canonical full roster, require each expected shard exactly once,
and reject missing/duplicate/unexpected entries, survivors, timeouts and
failed jobs. Persist compiler-unviable outcomes and their build diagnostics
as a separate class for review. Keep the same source scope, test selectors,
baseline and per-mutant deadlines. The current workflow has no such artifact
aggregate and did not run this matrix.

The opt-in [local verifier](../../../scripts/verify_mutation_shards.py) checks
a separately generated full `--list --json` artifact and directories with
`list.json` and `outcomes.json`, in zero-based shard order. It reconciles the
ordered expected partitions and exact outcome identities, checks successful
baselines, rejects survivors and timeouts, and reports compiler-unviable
outcomes separately. Retain the pinned build/source provenance, per-job exit
codes and raw compiler diagnostics for the release review.

```bash
python3 scripts/verify_mutation_shards.py "$records/full.json" "$records"/shard-{00..15}
```

Reproduce a listing from `pancake` with the pinned tool and the locked local
registry. Using an empty `shard` array lists the full roster.

```bash
cargo metadata --locked --no-deps --format-version 1 > "$records/metadata.json"
python3 scripts/assurance_targets.py "$records/metadata.json" cpu > "$records/cpu-targets.txt"
selectors=(--lib --bins)
while IFS= read -r target; do selectors+=(--test "$target"); done < "$records/cpu-targets.txt"
shard=(--shard 1/16 --sharding round-robin)
cargo mutants --workspace --test-workspace true --cap-lints true --all-features \
  --in-place --timeout 60 --build-timeout 180 --list --json "${shard[@]}" \
  -- --locked "${selectors[@]}" -- --include-ignored
```

## Scoped pilot after the quiet window

Start with the 17 `src/image_io/raw.rs` candidates once the RAW agent has
released the area. Existing tests independently exercise encoded size
rejection, missing and invalid DNG sensor metadata, all orientation outputs,
true RGB16, and RAW channel quantization. A survivor here would expose a
concrete decoder or resource-boundary invariant to test. In particular,
separate the initial metadata size guard from the post-read byte-count guard
and inspect exact boundary behavior before judging equality mutants.
[Current RAW parity and focused tests](../../p10-r50-raw-audit/f2f1bb3/README.md).

The second pilot can repeat the known 18 mutants across `JobControl::claim`,
`complete_frame`, `commit`, `pause` and `resume` using current tests. This
exercises publication and worker-release invariants and supplies a real
comparison against the earlier timeout and its cleanup fix. Both are scoped
diagnostics. Full mutation acceptance still requires the complete gate.
