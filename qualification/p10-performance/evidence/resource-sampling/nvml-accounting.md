# RTX 3090 post-exit accounting capability

Read-only calls on the physical RTX 3090 at PCI `0000:01:00.0`, UUID
`GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver 580.178.04, returned success
from `nvmlDeviceGetAccountingMode` and `nvmlDeviceGetAccountingBufferSize`.
The mode was disabled and buffer capacity was 4,000. Both already-exited
GPU context PIDs from the actual sealed E9 Vulkan/OpenCL warm-one pair
returned `NVML_ERROR_NOT_FOUND` for accounting statistics. They had positive
NVML residency during the original paired run. Post-exit allocation high water
remains unverified on this card. The capability inquiry changed no settings.
[Original native PIDs and paired traces](../integrated-e9-12f2151/README.md) ·
[Read-only probe](nvml-accounting-readonly.py.txt) ·
[Result](nvml-accounting-readonly.json)

NVIDIA specifies `maxMemoryUsage` as a per-process allocation high water in
bytes, available after exit on supported devices with accounting enabled.
Mode changes require administrator permissions. NVIDIA may report a missing
value, and the driver’s finite PID-keyed ring can overwrite old records or
replace records when a PID is reused. Disabling mode clears accounting data.
[Accounting calls and lifecycle](https://docs.nvidia.com/deploy/nvml-api/latest/api/group__nvmlAccountingStats.html) ·
[Allocation high-water field](https://docs.nvidia.com/deploy/nvml-api/latest/api/structnvmlAccountingStats__t.html)
