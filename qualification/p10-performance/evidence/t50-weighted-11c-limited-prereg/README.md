# Limited release preflight for packaged 11c T50 Weighted CPU

## Schedule, capacity and acceptance

Both applications use the unchanged `T50.weighted_average.cpu` settings
and the same first 50 frozen generated RGB16 TIFFs. Stage6 includes
those files plus two truth/extra files, all 52 totaling 2,775,174,405 B.
Its exact manifest is `1d9be3f1cd20a01083b11c10c4d17ab75a4f3b6bb7e2ebef682824e345ba649d`.
The runner authenticates its ownership and all encoded hashes before
and after the complete schedule. [Stage6 recipe](../t50-stage6-e81079d/README.md)

| Pair | Purpose | First | Second |
| --- | --- | --- | --- |

Every job uses a new process. Each arm's warmup primes only its own private
persistent compiler/application caches, and its three warm measurements
reuse those caches. The cold pair uses a distinct empty per-arm cache and
normal OS file caches. No decoded image, result or process crosses a job
boundary. Job time covers spawn through exit after the closed RGB16 export.
The per-job cap is 120 seconds under a 600-second app-schedule deadline.
Source checking, output decoding, proof reduction and rehashes take place
outside individual job timers.

Use CPUs 0–14 and 15 workers for each arm, with the process-tree RSS
observer on outside CPU15 on the Ryzen 9 7950X3D. The verified CPU-only
collector takes raw 3 ms nominal observations, processed under the unchanged
20 ms actual requirement throughout each job, including its spawn and exit
edges. All ten jobs, including warmups, must produce an eligible RSS
peak for a valid ratio. An observed bound remains visible when a gap
invalidates strict RSS. No repeat or interpolation is automatic.
[Validated 3 ms collector and independent check](../t50-memory-calibration-a49ce69-5f2623a/README.md)

| Limited metric | Candidate / reference comparison | Maximum |
| --- | --- | ---: |
| Warm completed job | Median of three elapsed jobs per arm | 1 / 1.80 |
| Tree RSS | Highest fully eligible raw-derived peak per arm over all five jobs | 0.55 |
| Cold completed job | Single isolated fresh-cache pair | 1.00 |

Each real export must be native unsigned RGB16 at 5998×3998 with verified
CPU completion and no fallback. The reference records exact frozen settings,
its 50 ordered inputs and keyed identity maps, one pixel cropped per edge,
and the CPU alignment/warp/fusion stages. Pancake records 50 ordered keyed
identity maps and 50 ordered sequential identity matrices. Every export's
bytes and decoded pixels must reproduce its own prior independently
reviewed image. One export per arm rechecks all 50 generated sharp-truth
regions and computes the paired pixel metrics. A changed export requires
new independent quality review before it can count. [Earlier exact
outputs and oracle](../t50-memory-calibration-a49ce69-5f2623a/independent-image-receipt.json)

The prior eligible calibration took 64.701 s for the reference and
25.987 s for standalone a49. That gives approximately 453.4 s, or 7m33s,
for five serial pairs on this host. Reserve 8–9 minutes of exclusive
host time to include source and producer hashes, ten independent RGB16
decodes, and one matched truth review. The standalone comparison is
planning information. Packaged 11c remains unmeasured for this protocol.
Ten retained exports consume about 1.55 GB. Reserve another few hundred
MB for the raw traces and private caches, and maintain at least 15 GiB
free. Keep raw attestations and hashes after independent review, then
reauthenticate and remove only the ten owned derived files/caches.
The Stage6 owner retains all original TIFFs.

## Exact producers and launch

Before requesting or using the new grant, check actual logical CPU activity
with `LC_ALL=C mpstat -P ALL 1 2` and attribute work using
`LC_ALL=C pidstat -u -p ALL 1 2`. Inspect CPUs 0–15 and their physical
siblings 16–31. Wait for sustained unrelated activity, including builds.
Ordinary idle app processes are compatible with a quiet slot. The first
measurement while preparing this receipt showed the host at 46.85% idle,
worker CPUs with 0–86.93% idle, sampler CPU15 at 3.50% idle, and multiple
active Rust compiler processes. It does not support a launch.

With explicit quiet-host and read-only Stage6 grants in place, use the
offline qualification interpreter and exact worktrees. The lease must have
`exclusive_host_granted`, `stage6_read_granted`, and
`other_workloads_terminal` set to true, plus the exact profile,
`materialization_sha256`, `sample_interval_ns` and schedule from the
static preflight. The output path must be new, private and outside all
producer and shared-input trees.

The operator's default invocation prints the preregistration without
launching an app or opening TIFF bodies. Synthetic policy tests check the
exact schedule, warm/RSS/cold limits, missing or malformed measurements,
and numerical misses. [Default preflight](static-prereg.json) ·
[Reducer tests](reducer-green.txt)

## First attempt and launcher correction

The first authorized attempt stopped after a valid reference warmup
when the Debian CLI rejected a CPU value in the GPU-runtime environment
override. The corrected wrapper uses the stock CPU runtime mapping and
both descriptors retain the explicit `--cpu` argument. Source, metrics,
limits, sampler and order stay unchanged. A fresh grant is needed for
a separate ten-job attempt, with new private caches and scope.
[Original invalid launch](../t50-weighted-11c-limited-invalid-launch/README.md)
· [CPU launcher regression](cpu-launch-green.txt)
