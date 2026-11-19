# Separate limited T50 process-tree RSS pair

The canonical Linux sampler on CPU15 uses 2 ms nominal acquisition
through exit and retains the actual process/session roster and timing.
An independent reduction requires every actual gap, including spawn
and reap edges, to be at most 20 ms. The maximum of the fresh eligible
candidate readings divided by the fresh eligible reference peak must
be at most 0.55. An ineligible arm leaves this comparison pending.
The kernel per-process high-water mark remains descriptive. This pair
supplies no frozen-full credit. [Exact static roster](static-prereg.json)
· [Typed strict gate and synthetic checks](rss-pair-gate.py.txt)

Both real complete jobs must reproduce the previously reviewed
5998×3998 RGB16 encoded and decoded hashes, selected CPU/backend,
maps and crop. The paired generated-truth check repeats all 50 regions
for each output. The runner reauthenticates both producers and all 52
source records before and after the pair and holds the common lock
through postflight. Keep both derived outputs private until verified
cleanup. Each child has a 120 s watchdog. Previous exact cold job walls
totaled 90.79 s. Reserve roughly 2 minutes plus any isolated failure
postflight. Use a fresh two-second per-CPU/pidstat census before the
explicit grant and launch. No repeat is automatic.

From the clean operator checkout and existing Stage6 pinned Python:

The lease must bind the profile, order, sample interval,
`materialization_sha256` and cache state from the static roster and
set `exclusive_host_granted`, `stage6_read_granted`, and
`other_workloads_terminal` to true. The default script only prints the
preflight. This preparation launched no applications or input reads.
