# P6 runtime selection qualification

Focused checks on 2026-11-18 used Rust 1.100.0, Linux x86-64, the Ryzen 9
7950X3D and the RTX 3090 at `0000:01:00.0` with driver `580.178.04`.
[The receipt](receipt.json) pins the application source, Cargo lock, tested
executables, inputs, expected results and retained report hashes.

| Check | Outcome |
|---|---|
| Test-first red | Both new tests failed to compile against the P1 core because `pancake::runtime` was missing. |
| CPU and physical NVIDIA Vulkan | Five focused tests passed (two CPU, three wgpu). Independent values passed after runtime qualification. Repeated wgpu acquisition retained the exact device and queue. |
| NVIDIA CUDA | The exact explicit-runtime test passed with NVRTC 12.5 and `--features cubecl/cuda`. The completed kernel used the same RTX 3090 identified in the Vulkan receipt. |
| Software Vulkan negative | The strict GPU path failed with zero physical adapters. Auto ran the native CPU and recorded the rejected GPU routes. |
| Pinned lint | Clippy passed for the runtime library and both focused integration targets with the generated dependency policy. |

Run from `pancake/`. All runtime commands unset `DISPLAY` and
`WAYLAND_DISPLAY` and use a private writable `XDG_RUNTIME_DIR` and
`PANCAKE_COMPUTE_CACHE_DIR`. The receipt file arguments request optional
hardware identity from the test process.

```bash
VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json \
  PANCAKE_TEST_RUNTIME_RECEIPT=/path/to/vulkan.txt \
  cargo test --locked --test cpu_runtime_contract --test gpu_runtime_contract -j 3 -- --test-threads=1

VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json \
  PANCAKE_TEST_REQUIRE_AUTO_CPU=1 PANCAKE_TEST_RUNTIME_RECEIPT=/path/to/software-auto.txt \
  cargo test --locked --test gpu_runtime_contract -j 3 -- --exact auto_reports_the_qualified_device

VK_DRIVER_FILES=/usr/share/vulkan/icd.d/lvp_icd.json \
  cargo test --locked --test gpu_runtime_contract -j 3 -- --exact explicit_gpu_override_completes_on_named_runtime

CUDARC_CUDA_VERSION=12050 LD_LIBRARY_PATH=/usr/local/cuda-12.5/lib64 \
  PANCAKE_TEST_GPU_RUNTIME=cuda PANCAKE_TEST_RUNTIME_RECEIPT=/path/to/native-cuda.txt \
  cargo test --locked --features cubecl/cuda --test gpu_runtime_contract -j 3 \
  -- --exact explicit_gpu_override_completes_on_named_runtime
```

Native runtime Cargo profiles remain `cubecl/cuda`, `cubecl/metal-native`
on Apple, and `cubecl/hip` for the retained HIP build. The independent runtime
check must pass the corresponding `PANCAKE_TEST_GPU_RUNTIME` value to prove
that explicit execution. Required Apple execution and retained HIP/Windows
builds remain pending at this revision. AMD and Intel GPU and Windows
execution are best effort.
