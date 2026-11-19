# Semantic follow-up to the failing Linux mutation shard

The archived `0a9ae85` Linux CPU shard `9/16` remains failed. All
167 operations completed with 108 caught, 45 missed, one timeout and
13 compiler-unviable. The proof base is local archive commit
`6d0ebb52baa7bc6a6d02e3350c20c7eb32213a5e`. This assessment reads
all 46 adverse rows from the authoritative outcome JSON, retains each
native file/function/span/operator identity and patch digest, and checks
available focused follow-ups against those exact operations.
[Full identity and evidence map](operation-map.json).

| Follow-up disposition | Rows |
| --- | ---: |
| Focused Linux CPU catch and restored pass, integrated at frozen main `54e3a8d` | 18 |
| Focused Linux CPU catch and restored pass, qualified on staged `d5d7505` | 5 |
| Focused physical RTX 3090 catch and restored pass on staged `d5d7505` | 1 |
| Miss with no qualified exact replay located | 21 |
| Native timeout still unresolved | 1 |
| Total | 46 |

Each qualifying follow-up records an actual mutated-process exit 101
and restored-process exit 0. The replay patch matches the archived
patch bytes in 23 cases. The Hybrid fine-band replay uses a
Git-format patch. Its stock module blob matches `0a9ae85`, and the only
changed source line is the same `index < 2` to `index == 2`
replacement. The byte difference comes from patch context, headings
and the mutator's inline annotation. The evidence covers one saved
operation at a time, so each historical native disposition stands.

The integrated CPU replays cover completion fingerprint, Hybrid
fine-scale selection, sharpness row/neighbor math, box-filter
normalization, both codec limits, previous-reference geometry,
resident allocation policy, stale worker results, Breathing clamp,
rigid maps, distant peak ambiguity, radius, inverse/crop geometry,
the Pearson floor and weak spectral peak abstention. The five staged
CPU catches cover four related pyramid reconstruction operations plus
the blue-luminance sign, with exact red and restored green evidence.

The Vulkan ordinal replay ran a real blur on RTX 3090 `0000:01:00.0`.
Stock rejected the absent ordinal 1 while the mutant ran it on ordinal
0 and returned `[2, 8, 14]`. The physical test failed for that reason
and passed after source restoration. Its command wrapper exited 1 at
a separate diagnostic-string check after the native processes ended.
The retained process exits and decoded assertion establish the
physical catch. The CPU classification remains a miss unless a later
canonical CPU run supplies evidence for that lane. There are 23 rows
without focused CPU catch evidence across either audited tree, plus
five with CPU catches still staged at the main snapshot.

## Unresolved operations

Nine core survivors and the one timeout remain open. Equal-score
refinement at `estimate.rs:453:29` changes the fallback direction when
curvature is unusable. A distinguishing oracle or a reviewed
equivalence argument is still needed. The other core misses alter
the spatial correlation row lattice, two luma prefilter expressions,
the spectral window's right sample and taper, FFT vertical cell
addressing, the complex cross-power dot product, and Exposure
contrast on a one-axis edge. Their local arithmetic can be audited
independently. No native red/green pair for those exact operations
was located. At `spectral.rs:259:32` the mutant underflows the
second FFT butterfly index, built successfully and hit the actual
60-second Test deadline. The archived disposition stays Timeout.

## Runtime and final CI readiness

The exact local shard took **1h 3m 41s**, including its 169.59s Build
and 6.53s Test baseline. Two direct jobs ran on CPUs 6 and 7. Sampled
target allocation peaked at 7,339,266,048 bytes and finished at
7,331,684,352 bytes. The 168 phase logs totaled 170,442,947 bytes.
The byte-checked raw archive contains 384 files and is 29,344,241
bytes compressed. These are local measurements with a dedicated
target, disabled incremental compilation and dev debug, and an unset
compiler wrapper.

The reviewed `edc1d26` matrix remains local. The current workflow
still has the serial CPU mutator inside a 30-minute assurance job.
The draft retains the other assurance jobs and requires all 16
Ubuntu partitions in its final aggregate, with four concurrent
jobs and a 75-minute timeout per partition. The single local result
uses one of 16 partitions at an older source, leaving no passing
canonical shard. Its runtime leaves approximately 11m 19s against
the draft limit before differences in hosted build defaults,
provisioning, shard mix and artifact upload. Hosted transport,
rerun behavior and per-shard headroom remain unqualified.

A final invocation requires one integrated source and its freshly
classified CPU targets and independent ordered roster. The test tree
at `54e3a8d` already has 17 CPU integration files versus the
original 16 selected targets. Staged `d5d7505` also changes the
resident-input product path. Reusing the old 2,672-row roster, its
167-row partition, or pre-merge receipts cannot satisfy the final
checkout, lock and selector bindings. The candidate workflow needs
review and authorized integration at the final source. Fresh same-run
producers must complete every shard with successful baselines and
reconcile exactly once, with no survivors, timeouts or missing phases.
The remaining P9 and P10 release gates and the user's final-push
condition still apply. No workflow, product code, test or threshold
was changed for this assessment. No build, device access or network
operation ran.
