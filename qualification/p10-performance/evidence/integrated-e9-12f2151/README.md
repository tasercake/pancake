# Integrated E9 on the locked Linux release

The coordinated measurement window was 2026-11-18 13:17:14–13:19:48 SGT on a
Ryzen 9 7950X3D with 32 logical CPUs. The physical GPU run compared
`wgpu-vulkan` against OpenCL index 0 on the same RTX 3090, PCI `0000:01:00.0`,
UUID `GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver 580.178.04. The
ordinary desktop remained active, initially reporting 2% GPU utilization
and 689 MiB used. The separate diagnostic completions confirmed explicit CPU
or GPU execution. Each output hash matched that target’s ten warm samples.

Each lane ran 17 completed primary jobs per application, including two
warmups, ten warm observations and five fresh-cache cold observations. The
runner alternated each pair and launched new processes. Four additional
diagnostic jobs verified backend selection. The table uses observed program
wall time and process-tree peaks under the frozen numeric targets.

| --- | ---: | ---: | ---: | --- |
| CPU warm median, ≤0.50 | 0.424805 s | 0.922777 s | 0.46036 | Pass |
| CPU cold median, ≤1.00 | 0.413716 s | 6.585616 s | 0.06282 | Pass |
| CPU RSS, ≤0.50 | 112,029,696 B | 351,375,360 B | 0.31883 | Pass |
| Vulkan/OpenCL warm median, ≤0.50 | 0.559721 s | 0.793928 s | 0.70500 | Fail |
| Vulkan/OpenCL cold median, ≤1.00 | 0.679516 s | 7.136465 s | 0.09522 | Pass |
| Vulkan/OpenCL RSS, ≤0.50 | 396,156,928 B | 550,313,984 B | 0.71987 | Fail |
| Vulkan/OpenCL GPU memory, ≤0.50 | Pending | Pending | Unavailable | Pending |

All 68 RSS traces met the 20 ms cap. The four per-target maxima were
10.411 ms and 9.059 ms on CPU, and 8.831 ms and 9.385 ms on Vulkan/OpenCL.
GPU sampling exceeded the cap on 17 of 17 Vulkan and one of 17 OpenCL jobs.
The largest Vulkan gap was 33.885 ms in warm job 6, including a 26.809 ms
individual NVML acquisition at context teardown. The longest OpenCL gap was
21.033 ms in warmup 1. The attributable sampled lower bounds were
204,443,648 and 320,864,256 bytes respectively. The required GPU peak ratio
remains pending.

Every export independently decoded as genuine 337 × 510 RGB16. Per-target
pixel hashes were stable through all warmup, warm, and cold runs. Full-image
pixel MAE between the applications was 2,823.78 on CPU and 2,825.18 on GPU
out of 65,535. Independent first-warm review covered three corresponding
48 × 48 regions. CPU MAE across the regions was 3,706.15, 4,118.22, and
3,452.26. GPU MAE was 3,706.53, 4,118.10, and 3,451.18. The review files
record full decoded hashes, channel values and clipping fractions. Equal
output quality and full release acceptance remain open.

The benchmark runner’s only local qualification change made the explicit
review option retain both verified first-warm outputs after numeric misses.
The retention patch and runner hashes are separate from the unchanged
Pancake product identity. Review and authenticated TIFF removal ran after
the paired timings. All four review exports and the case-owned caches were
removed. The paired archive contains the commands, product and reference
locks, sample traces, independent review, and cleanup records.
