# E9 pyramid scheduling and Vulkan preparation

2026-11-18, Linux x86-64 6.8.0-147-generic, AMD Ryzen 9 7950X3D and
NVIDIA RTX 3090 at PCI 0000:01:00.0 with driver 580.178.04. The comparison
uses the nine original ordered Balloons PNGs, frozen E9 registration and
exposure settings, and independently decoded RGB16 TIFF output.

The baseline was `bbb4767`, SHA-256
`89e4cb8131f9cc8d7f314ae820c04505962f17cf68b8d4bb988134f9f0b61417`.
The optimized candidate was `44e11d0d25b4327d298bb8766361b5925748f9e1`,
SHA-256 `cf2686dd60ae2e725ac423b76880a9c8040652d1201b51a8e5ab2bac4d5dbbad`.
The locked release builds used
`cargo +1.100.0 build --release --locked -p pancake-cli`.
The candidate lock SHA-256 is
`639c5c88b9286e5583174f05168412db19f39dc3f651b6682b685d1db801e0dd`.

## Diagnosed work

A diagnostic executable observed the production job engine's publicly
acknowledged stages every 250 µs. Its startup interval starts inside the
diagnostic process. Final fusion and readback share a stage. Concurrent
builds and test jobs were permitted during the observation.

| Backend | Prepare | Decode | Register | Per-frame fusion | Finish and readback | Encode |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| CPU | 36 ms | 30 ms | 1,502 ms | 2,276 ms | 113 ms | 7 ms |
| Vulkan | 1,195 ms | 19 ms | 210 ms | 186 ms | 9 ms | 5 ms |

Two actual two-frame exposures at different image sizes previously requested
17 and eight kernel identities. Stable multiscale workgroup geometry made
their key sets identical. A subsequent failing test observed 63 launches for
two four-reduction exposures. Joint RGB/weight reduction and direct weighted
band accumulation reduced that workload to at most 39 launches. The same
CubeCL kernels execute on both backends, with checked tail lanes and f32
intermediates. Exposure still uses the 4+4+1 equal-weight batch tree.

Vulkan index 0 and default selection can qualify the leading discrete
adapter directly, retaining full enumeration for fallback and later indices.
The job-budget check from `2375efa` remains in acquisition and renderer reuse.

## Completed-job diagnostics

Each candidate completed one warmup and three fresh-process observations per
backend, in alternating order. Other work could run concurrently. The rates
compare Pancake revisions and carry no frozen performance gate credit.

| Backend | Baseline median | Optimized median | Candidate / baseline | Largest decoded change |
| --- | ---: | ---: | ---: | ---: |
| Explicit CPU | 2.378 s | 1.569 s | 0.660 | 1 RGB16 code across 174 channel samples |
| RTX 3090 Vulkan, index 0 | 0.885 s | 0.630 s | 0.711 | 1 RGB16 code across 162 channel samples |

The optimized locked CLI passed all 69 P4 cases on a clean checkout.
Eight focused CPU tests and five physical Vulkan tests passed, including
the shared renderer budget rejection, actual hardware witnesses and both
assertions about kernel specialization and launch count. Release-profile
Clippy with the effective dependency policy and the workspace formatting
check passed.
