# Exact positive Canon CR3 parity on the physical RTX 3090

The two exact release CLIs processed the same positive Canon CR3
originals, frozen R50 rows 0 and 49. This scoped test kept the actual
Hybrid stack flow, kernel 6, eight pyramid levels, alignment scale 6,
first-anchor similarity, no slabs, auto crop and RGB16. It used 15
distinct physical CPU cores at logical 0–14. Each original's native
51,775,488 working samples meet the existing parallel RAW threshold
under the installed 14-worker RAW pool. Both original sizes and hashes
and both exact ELF identities matched after execution.
[Positive source identity](../canon-positive/acquisition.json)
[Exact stock and candidate ELFs](../canon-positive/binaries.json)
[Exclusive device lease and host affinity](physical-lease.txt)

Both launches explicitly selected `--gpu --gpu-id 0`, `PANCAKE_RUNTIME=wgpu`
and the known NVIDIA ICD. The runtime completed on the
NVIDIA GeForce RTX 3090 at PCI `0000:01:00.0` through `wgpu-vulkan`.
NVIDIA inventory and Vulkan enumeration identified the same card,
driver 580.178.04 and UUID
`GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`. Both executions emitted the
two ordered frame progress events and a true physical GPU completion.
The fallback field was empty and no fallback or observer error event
occurred. Both processes finished and returned free VRAM to its
pre-launch value. [Before inventory](nvidia-before.stdout)
[Vulkan inventory](vulkan-inventory.stdout) [After inventory](nvidia-after.stdout)
[Stock completion](stock/events.jsonl) [Candidate completion](candidate/events.jsonl)
[Terminal receipt](phys-canon-end.txt)

The complete 5086×3390 RGB16 TIFFs had identical encoded SHA-256
`b5d2a3c27f8170cbf013f8600752c1f249cd5f8ba42afaf2798ebb50d8fa1a1c`
and independent decoded-pixel SHA-256
`0e9ecaa26ade7ff2cef12872db5654b7f4e4c763fb3def6c8fd120efaec6a7ce`.
All alignment and sequential transform f32 bits matched in manifest
order. The TIFFs preserved genuine depth. This native two-image
correctness check carries no performance or frozen P10 sampling credit.
[Full verification and startup settings](report.json)
[Stock launch](stock/execute-launch.json) [Candidate launch](candidate/execute-launch.json)

All output TIFFs and all staged Canon originals were rehashed and
removed after preserving these receipts. The exact ELFs remain local.
[Final media cleanup](../full-r50-diagnostic/cleanup.json)
