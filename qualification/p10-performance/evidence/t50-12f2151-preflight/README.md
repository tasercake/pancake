# T50 Hybrid matched-job preflight

The initial preparation reviewed retained evidence and wrote a quiet launch
plan during the D150 campaign. After D150 ended, the sealed candidate and
reference were staged and all 52 TIFFs were generated for read-only reuse
by the viewer A/B and this benchmark. One complete diagnostic pair
established matching stationary RGB16 geometry. The candidate took 3.084×
the reference time and produced different measured detail transfer.
The full timing schedule was closed. [Diagnostic evidence](pair-001/README.md)

## Exact inputs and prior closure

The generated 6000×4000 RGB16 recipe uses seed `20261117`, 256×256
tiles, fifty distinct focus positions, a sharp RGB16 truth, and a categorical
depth truth. The frozen receipt SHA-256 is
`8881a4968bb95f5a911b040a6ebc9635a9cbdc5a759ac0443605de33f700849c`.
Its 52 file hashes and 52 independent decoded-pixel hashes matched a complete
Linux rematerialization. The CLI consumes only `focus_000.tif` through
`focus_049.tif`, in receipt order. The two independently generated TIFFs are
reserved for review. [Recipe](../../../t50-generated/README.md) ·
[Linux verification](../../../../pancake/docs/p2-corpus-evidence/t50-linux-verification.json)

The full `5e10244` Hybrid pass was on the RTX 3090 through Vulkan, with all
50 reference-to-source transforms exactly identity and the agreed one-pixel
symmetric crop. Its independent 3998×5998 RGB16 output MAE was 843.33/65535
against generated truth across the entire centered overlap. All 50 focus
regions remained visible, with vertical second-difference variance ratios
from 0.8667 to 0.8918. That one job completed within its 360-second deadline.
The receipt has no timing or resource measurement. The sealed release
`12f2151` includes newer Hybrid fusion and registration code and needs its own
result and comparison. [Previous full T50 result](../registration-recheck/README.md)

## Run the first pair when the host is quiet

Run from the staged `12f2151` checkout. The variables point at fresh owned
scratch, the sealed package, the saved P9 producer summary, this evidence
directory, and the existing frozen generator interpreter.

The first pair deliberately stops before any 34-job schedule. When both
outputs have been reviewed, the unchanged `p10.py run` command selects
`--runtime wgpu-vulkan --profile T50.hybrid.gpu`, the same `--gpu-uuid`,
both index flags set to 0, the existing build receipt and input root,
`--threads` equal to the permitted affinity width, a fresh `--output`, and
`--timeout 900`. Its schedule contains 17 jobs per target plus one backend
diagnostic per target. A different runtime or affinity after a resource abort
requires a new, explicitly identified profile. CPU-versus-CPU is separately
valid with the same 50 RGB16 TIFFs and unchanged quality settings.

Review and hash the two diagnostic TIFFs before removing them. After all
consumers finish, invoke `t50_generated.py --cleanup-from "$scratch/materialization.json"
--output "$scratch/inputs" --receipt "$scratch/cleanup.json"` using the same
generator environment. That command rechecks all 52 file hashes and
authenticates the directory before deleting the generated TIFFs. Preserve
the receipts and remove the diagnostic's owned output, cache and package
extraction after review.

## Resource estimates and open checks

| Resource | Read-only estimate and basis |
|---|---|
| Corpus space | Exact Linux payload 2,775,174,405 B. Generator preflight 3,067,213,930 B plus 16,106,127,360 B reserve. The full runner holds at most two output pairs, around 576 MB at 144 MB per uncompressed TIFF, plus package, traces and caches. Start with at least 24 GiB free. |
| Candidate memory | 288 MB per packed RGB f32 allocation. The `12f2151` Hybrid constructor budgets about 2.944 GB for levels plus scratch under a 3 GiB fusion share of the 4 GiB logical job cap. A device page still must fit a packed RGB frame. The logical cap excludes allocator reserve and measured RSS/VRAM. |
| Host RSS | Plan around several GiB for candidate and up to 12 GiB for reference until measured. On the smaller 5088×3392 R50 CPU functional diagnostic with four allowed CPUs, observed lower-bound process high-water marks were 2.51 and 8.28 GiB respectively. The full T50 OpenCL loader can have up to eight workers, a lookahead frame, and retained reference data. Those mechanisms and temporary precision conversion can raise peaks. |
| Time | Plan a few minutes for 52-image generation, verification and hashing, then up to 30 minutes for a first 900-second-per-job pair before review. The prior full Vulkan candidate completed within six minutes. A tentative 1–6 minutes per job gives 0.6–3.6 hours for the 36-job campaign plus validation. There is no matched T50 timing baseline. Refine the window using the first pair. |

The pinned TIFF reader uses OpenCV unchanged-depth decode. RGB16 enters
the reference Hybrid path as float32 values scaled by `255/65535`. It
exports RGB16 by rounding after multiplication by 257. The OpenCL input
helper decodes high-bit TIFFs twice before using the f32 path, which affects
cost. Source inspection reveals no unconditional 8-bit conversion for
T50 Hybrid. The first real reference export must independently establish
genuine precision and stationary geometry. The reference's transform
confidence policy differs from Pancake's repaired self-repeat check, so
the prior candidate's identity matrices cannot establish reference geometry.

The E9 Vulkan receipt already shows GPU sampling gaps above the frozen
20 ms limit at teardown on this same host. Preserve actual T50 sample gaps
and lower bounds. A complete pair can support a functional decision while
that resource metric remains pending. [Prior resource evidence](../integrated-e9-12f2151/README.md)
