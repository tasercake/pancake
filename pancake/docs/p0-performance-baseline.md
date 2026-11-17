# P0 real corpus and preliminary reference baseline

Date: 2026-11-17, Asia/Singapore. Worktree: `codex/p0-performance`, based on
`8fcbc52`. Status: real D150/E9/R50 corpus acquired and pinned; two preliminary
reference CPU jobs completed. **No performance target or P0.16 pass is claimed.**

## Corpus selection

| Workload | Real data frozen | Precision and limits |
|---|---|---|
| D150-native-dorsal | First 150 source-filename-sorted frames from the 152-frame amber mite dorsal optical focal stack; 2304x1728 RGB JPEG | Original 8-bit photographs. No upscale, duplicated frame, or artificial precision promotion. Not the 24 MP RGB16 TIFF workload. |
| E9-native-balloons | All nine Balloons photographic exposure brackets from Waterloo MEF; 512x339 RGB PNG | Original 8-bit exposure images. Correct exposure-fusion domain, separate from focus stacking; nine frames exercise 4+4+1 batching. |
| T50 | Not selected | Still needs 50 real 6000x4000 genuine RGB16 ZIP TIFF sources. |
| R50-sood-first50 | First 50 consecutive Canon EOS R5 Mark II CR3 files, frame_000–049; full visible decode 5118x3412 RGB | 14-bit sensor RAW, stored as uint16. All files hash-verified and full RAW decoded with LibRaw. Below T50 resolution. |

Sources are the existing Bokehforge frozen selections:

- D03 [amber mite focal stacks](https://doi.org/10.6084/m9.figshare.14707077.v1),
  version 1; recorded CC BY 4.0. Dorsal archive SHA-256
  `311ca6aab7d3bff03c714fd2d81c741f3847de1d4d3f2f9cfa767b6eb62cf56a`.
- D28 [Waterloo MEF](https://ivc.uwaterloo.ca/database/MEF/MEF-Database.html),
  creator reuse/attribution terms recorded in config. Archive SHA-256
  `02939e5eb63d0536793e467848c9381e41c71ca1642b9f0291e2fc0843744242`.
- D05 Lytro was also acquired/processed successfully, but is not used in these
  measurements. Its config records unresolved image terms; no image is redistributed.

Total unique native acquisition was **214,404,651 bytes**, below the unchanged
2 GiB budget. D03's configured 150 MiB acquisition cap selected three archives.
Disabled D01 was not resolved or downloaded. No policy or source pin changed.
Acquisition and source conversion completed within approximately three minutes;
the six-minute bounded discovery window did not require fallback data.

## Completed-job measurements

| Job | Samples | Full process wall time | Peak process RSS | Result |
|---|---:|---:|---:|---|
| D150 Hybrid, breathing alignment, slabs 40/overlap 4 | 1 | 30.585 s | 2,897,117,184 bytes | Exit 0; 2304x1728 RGB uint16 TIFF, finite values, original input hashes unchanged. |
| E9 exposure fusion, no alignment | 1 | 0.877 s | 237,469,696 bytes | Exit 0; 512x339 RGB uint16 TIFF, finite values, original input hashes unchanged. |

Both jobs used kernel 3, pyramid levels 4, scale factor 6, and 16-bit TIFF
export. D150 requested first-reference breathing and reported cumulative scale
1.0000-1.0003 with **145 fallback steps**. That is a useful warning against
claiming real-scene registration quality from completion alone; no independent
geometry truth was evaluated here.

External wall timing includes interpreter startup, import/JIT/cache work,
per-frame decode, alignment when selected, fusion, and complete TIFF export.
The CLI's smaller numerical timers (26.62 s and 0.13 s) exclude parts of that
work and are not substituted for full-job measurements. `/usr/bin/time -l`
reported peak RSS in bytes; this does not measure GPU memory or justify adding
CPU/GPU allocations as independent physical pools on unified memory.

Approximate job intervals were 00:25:15-00:25:46 UTC. Runtime agent confirmed
no heavy CPU/GPU work immediately before launch. OS background load, frequency,
thermals, and contention were not independently sampled, so controlled-load
performance is not claimed.

## Qualification still required

These are **single fresh-process diagnostic samples**, not warm-engine medians.
A new worktree-local Numba cache was used initially; E9 followed D150 and reused
populated import/kernel caches. OS file caches were not cleared. There were no
two warmups plus ten measured warm jobs or five fresh-cache cold runs.

These jobs also differ from the frozen target profile: kernel 3/levels 4 rather
than 6/8, no auto-crop, and E9 without the target's Euclidean alignment. Exact
profile equality must be established before a future matched comparison;
these single diagnostic values cannot be promoted into qualified baselines.

The integration-owned target specification remains authoritative: at least 2x
warm completed-job speed, no more than 50% reference peak process memory at
equal quality, nonregressing cold median, viewer event-to-present P95 at most
33 ms, and pause/cancel safe-state acknowledgement at most 250 ms.
The present numbers cannot establish any of those relative targets.

Remaining work includes the other three focus methods, full sampling protocol,
actual GPU jobs/runtime identity, same-quality Pancake comparisons, full
precision/geometry validation, and viewer/lifecycle latency. T50 acquisition remains open. R50 acquisition and full RAW corpus decoding are
complete; no matched R50 job baseline has been measured. No real photograph is resized or converted from 8-bit into claimed
genuine 16-bit evidence to fill those gaps.

## Reproduction

From this worktree's root, using the unchanged 2 GiB policy:

Verification: local scaffold `scripts/check.sh all`, staged whitespace checks,
manifest/log/archive hashes, unique native input hashes, output decoding, and
local links passed. Scaffold checks still contain zero application tests; no
application coverage percentage is claimed. Remote/platform/sanitizer, GPU,
full statistical, viewer, and lifecycle qualification runs were not performed.

## R50 acquisition update

New payload: 442,098,996 bytes. Cumulative unique corpus acquisition:
656,503,647 bytes, within unchanged 2,147,483,648-byte budget. Full 999-frame
source is approximately 8.96 GB; only this explicit 50-frame subset was acquired.
Bokehforge config, full-source download policy and disabled D01 remain unchanged.
RAW files stay ignored under the retained `codex/p0-unblock` worktree,
`datasets/p0-d02/raw/`. No original image is committed.

All 50 images completed `rawpy 0.26.1`/LibRaw full-image postprocessing using
`output_bps=16`, `use_camera_wb=True`, `no_auto_bright=True` and other defaults.
Decoded shape: 3412x5118x3; dtype: uint16. Sensor white level: 16383; nonzero low
eight bits confirmed in every visible RAW plane. This is genuine higher-than-8-bit
source precision; it does not claim a 16-bit sensor ADC. Manifest records engine
versions, parameters, original/decoded dimensions, source rights and file hashes.

This resolves R50 source selection, not the Rust decoder, complete baseline or
P0.08 gate. T50 still needs 50 distinct native sources at least 6000x4000; these
files cannot fill that requirement without disallowed upscaling.
