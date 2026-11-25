# P9 Linux native CLI profiles

On 2026-11-18, the optimized production CLI compiled and linked on Linux
x86-64 at clean source revision `79db6fc62bed7f61b136d7384992c4970dd09ef3`.
Rust 1.100.0 used the committed Cargo lock. Both resolved native graphs passed
dependency policy and the pinned offline advisory, ban, and source checks.

| Production build | Link verification and execution |
|---|---|
| `cubecl/hip` | Passed. The x86-64 ELF defined 270 HIP runtime symbols. HIP was compile and link only. |
| `cubecl/cuda` | Passed. The separate x86-64 ELF defined 281 CUDA runtime symbols. That binary passed all four frozen CPU/CUDA cases below. |

These commands select the workspace core and CLI together. CubeCL is a direct
dependency of the core. The opt-in graph requires no CLI feature forwarding.

```bash
cd pancake
cargo build --locked --release -p pancake -p pancake-cli --bin pancake-cli -j 3 --features cubecl/hip
CUDARC_CUDA_VERSION=12050 cargo build --locked --release -p pancake -p pancake-cli --bin pancake-cli -j 3 --features cubecl/cuda
```

|---|---|

Both GPU events reported `cuda`, physical hardware at PCI `0000:01:00.0`,
and the `ptx_sm86` fingerprint. The run used driver 580.178.04 and NVRTC
12.5.40. The CPU run used the same CUDA-linked binary with explicit CPU
requests and the runtime override and CUDA library path unset. All jobs
published output and recorded one successful completion with no fallback.
The audit also matched all 90 adopted frozen contract files.

These focused release runs qualify the Linux native build profiles and the
four selected contract cases at this revision. Full packaging, final revision
coverage, retained Windows builds, and Apple execution require separate evidence.
