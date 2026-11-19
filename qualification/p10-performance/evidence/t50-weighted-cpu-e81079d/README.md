# Exact e81079d T50 Weighted Average CPU diagnostic

| --- | ---: | ---: |
| Process launch through closed export and exit | 65.857370 s | 43.615742 s |
| Observed peak process-tree RSS | 3,461,345,280 B | 2,119,315,456 B |
| Maximum RSS observation gap | 25.190 ms | 12.607 ms |
| Combined waited user and system CPU time | 183.403 s | 168.427 s |
| Whole-output MAE versus generated sharp truth | 1083.931 u16 codes | 1080.736 u16 codes |
| Vertical second-difference variance ratio in 50 strips | 0.840468–0.882494 | 0.840504–0.882531 |

The completed-job ratio was 0.662276. At the observed reference duration,
a 0.50 ratio would require 32.928685 s, 10.687057 s less than the measured
candidate or 24.50% of its wall time. This single diagnostic ratio screens
the next engineering work. Warm and cold medians require the unchanged
34-job schedule. The independently reduced traces reproduce the two
sampled RSS peaks and cadence. The descriptive 0.612281 RSS ratio uses
observed values. The reference gap exceeded 20 ms, so the required paired
memory ratio remains pending. [Frozen targets and schedule](../../../../pancake/docs/performance-targets.json)

Both apps exited 0 on explicit CPU without fallback. The reference completion
also reported CPU for alignment estimation, warping and fusion. Both jobs
reported all 50 identity transforms and exported genuine 5998×3998 RGB16.
The reference printed a one-pixel crop on each side, matching the candidate's
symmetric crop from identity maps. Independent decoder and truth review
covered all 50 strips. Full-output inter-app MAE was 11.099 u16 codes,
RMS 20.702 and maximum 227. The evidence supports close output detail for
this generated case and leaves frozen equal-quality acceptance open.
The sealed outputs are the fused weighted RGB readbacks; the stock app
exposes no intermediate weight map in this configuration.

## Next attribution diagnostic

Use the existing [external stage boundary approach](../../../../pancake/docs/p10-r50-phase-287828a/README.md#external-stage-diagnosis)
against the sealed ELF in a separately authorized scope. The [exact ELF symbol
inspection](elf-phase-symbols.txt) confirms entry points for source decode,
registration thumbnail and pair estimation, the basic accumulator's add and
finish, stream finalization, export preparation and publication. Keep app
affinity fixed. Collect one completed trace with the same ordered inputs
and require 50 decode, 50 thumbnail, 49 estimate, 50 accumulation and one
finish/crop/export sequence. Require matched maps, encoded/decoded hashes
and completion. Use the existing chronological boundary reducer with nested
intervals kept separate and report debugger overhead and unaccounted wall.

Split the largest measured parent interval in a later trace. Exact source
routes every decoded image through similarity estimation at its existing
anchors and then through the basic focus accumulator. The weighted path
materializes a full RGB buffer for registration proxy preparation and stages
the full RGB again at the accumulator. At 50 native frames each call site
addresses 14.4 GB of logical f32 RGB input. This accounting describes data
submitted to the call sites. Measure the two upload boundaries inside the
top-level intervals. Include per-frame finite validation, CubeCL CPU command
execution, Gaussian passes, weighted update and the existing sync separately
only where the underlying boundary is observable. An asynchronous backend
copy may fall within synchronization. Host upload-call time alone cannot
identify it.
[Streaming dispatch](../../../../pancake/src/job/stream.rs) ·
[Registration normalization](../../../../pancake/src/job/registration.rs) ·
[Registration staging](../../../../pancake/src/registration/kernels.rs) ·
[Basic accumulation and synchronization](../../../../pancake/src/fusion/focus.rs) ·
[Prior opt-in T50 phase trace](../../../../pancake/docs/p10-t50-phase-trace/README.md)

Identity maps bypass full-size warp in this exact run after estimation.
Preserve registration, normalized source pixels, the focus weighting rule,
frame order and actual export for a later measured change. Report the
reference program's printed 52.73 s separately from the external 65.857 s
whole-job time. The difference warrants a startup-to-first-frame boundary
and does not identify import, JIT, I/O or shutdown cost by itself. The
prior T50 and R50 phase times used different methods, runtimes or sources.
Retain this single pair as diagnostic evidence until a follow-up attributes
the required wall reduction and resolves the independent RSS cadence.
