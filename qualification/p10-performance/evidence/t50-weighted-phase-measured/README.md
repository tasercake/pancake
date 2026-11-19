# Measured stock e81079d Weighted Average CPU phases

The separately authorized `870ef45` experiment ran one stock control and
one GDB-traced job, using the same sealed `e81079d` Debian ELF. Both used
the frozen T50 Weighted Average settings and the same 50 ordered original
RGB16 source TIFFs. Worker affinity was 0–14 with 15 worker threads. GDB
used CPU15. Each app started with its own empty case cache. The scope ran
from 01:26:00 to 01:27:34 SGT on 2026-11-19. Both apps exited 0 on explicit
CPU without fallback and drained their sessions. All 52 files, the
materialization receipt and the ELF matched their hashes after execution.
[Verified timings and identity](summary.json) · [Prepared replay](../t50-weighted-phase-preflight/README.md)

The control took **43.569249 s**. The traced process scope took
**45.932933 s**, 2.363684 s or 5.43% longer. This observed elapsed
difference includes debugger and run-order effects. GDB's run-through-exit
interval was 45.569872 s, with 43.966986 s covered by disjoint top-level
phases and 1.602886 s outside them. The observer handled 1,858 stops,
including 850 filtered or unmatched stops. Callback bookkeeping summed to
0.134151 s. The interval figures remain inclusive traced host-call times.
The paired stock images reproduced the previous exact encoded and decoded
RGB16 digests at 5998×3998, and their complete event bytes matched. Both
recorded 50 keyed identity maps and 50 ordered sequential identity maps.
The original control-only validator failure remains a separate receipt.
[Prior failure](../t50-weighted-phase-attempt/README.md) ·
[Prior exact CPU truth comparison](../t50-weighted-cpu-e81079d/README.md)

| Traced phase | Count | Inclusive time |
| --- | ---: | ---: |
| Decode | 50 | 6.162 s |
| Registration thumbnail | 50 | 10.655 s |
| ↳ Original RGB staging | 50 | 5.960 s |
| Registration estimate | 49 | 1.029 s |
| Weighted accumulator add | 50 | 25.672 s |
| ↳ Original RGB frame staging | 50 | 12.961 s |
| ↳ Weighted filter submission | 50 | 0.203 s |
| ↳ ↳ Aggregate submissions | 100 | 0.086 s |
| ↳ Exact sync block-on call | 50 | 0.050 s |
| Focus final readback | 1 | 0.132 s |
| ↳ Read-image call | 1 | 0.130 s |
| Crop and finalization | 1 | 0.044 s |
| Prepare export | 1 | 0.272 s |
| Publish export | 1 | 0.001 s |

An independent reduction checked every recorded interval's duration,
expected call count and parent containment. Top-level intervals and
direct children within Weighted add were disjoint. The registration and
Weighted staging calls together accounted for 18.921 s in the traced run.
Median staging was 119.104 ms per frame for registration and 259.398 ms
for Weighted, with narrow per-frame ranges. The observed costs were
sustained across the 50 frames. The current single-pair diagnostic wall
shortfall is approximately 10.687 s at its observed reference duration.
This supports prioritizing the owned-frame staging comparison. It does not
predict savings from that change. [Exact staging callsite](../t50-weighted-phase-preflight/fixed-calls-disassembly.txt)

Weighted add still has 12.458 s outside its direct measured children.
Aggregate launch measurements cover submission. Kernel work and backend
copies can occur elsewhere in the traced add interval. The 0.050 s
block-on interval follows the sync binding and submission code. Those
facts leave reflected-index computation time unattributed. Preserve
all settings, source order, output precision, maps and device for the
owned-frame candidate's eventual matched diagnostic. Only a successful
measured candidate can justify a product change or a frozen schedule.

The archive retains exact launcher results, quiet scope, application
events, observer intervals and stdout/stderr. The benchmark stage remains
owned for any separately authorized follow-up. This experiment supplies
phase attribution and no frozen time, memory or equal-quality verdict.
[Raw phase and control receipts](raw-phase-pair.tar.gz) · [Checksums](SHA256SUMS)
