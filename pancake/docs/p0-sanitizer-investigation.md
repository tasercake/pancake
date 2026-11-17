# Linux GPU AddressSanitizer investigation

**P0.05 remains failed for Linux GPU AddressSanitizer. No sanitizer suppression,
test skip, target split, source change or gate exemption was made.** Investigation
stopped after three distinct discovery/profile changes failed to establish a fix.
The assumption that all GPU execution is incompatible with ASan is not supported:
a minimal safe wgpu device request succeeds under the same Rust ASan executable.

## Scope and retained state

Evidence was collected on 2026-11-17 in the existing task container
`krishna-desktop/pancake-p0-linux-20261117`: Linux x86_64, NVIDIA RTX 3090,
driver 580.178.04. The original native unsanitized CPU/Vulkan qualification had
passed before this investigation. This investigation did not retest AMD, Intel
or Windows. Source hashes, commands and bounded evidence are recorded in
[`p0-sanitizer-evidence/evidence.json`](p0-sanitizer-evidence/evidence.json).

All diagnostic sources, downloads and build outputs were under the container's
`/work/pancake/target/`. Application manifests, lockfile, library and integration
test source were not edited. At the final process inspection no diagnostic
process was running. PID 29503 was a killed, defunct child (`STAT=Z`, parent PID 1),
not an executing probe. The container had 29 GiB free, above the 15 GiB reserve.
The parent agent retains control of container shutdown and cache retention.

## Observations

| Probe | Result | Interpretation |
|---|---|---|
| Original `check.sh sanitizer address` | Three host tests, CPU blur and invalid shapes passed. GPU test failed `Unable to request a wgpu device: Parent device is lost`; LSan reported 440 bytes in three allocations. | The GPU sanitizer gate fails. Host/CPU successes do not clear the GPU failure. |
| GPU-only original test, loader tracing | Same device error and 440-byte leak, without executing the CPU test. Vulkan loader reports NVIDIA ICD `vkCreateDevice` failure. | Running the CPU JIT workload is not required to trigger this failure. Linked CPU dependencies remain present, so binary-level interactions are not ruled out. |
| NVIDIA-only Vulkan ICD discovery | Same device error; leak reduced to 184 bytes in two allocations. | Restricting unrelated Vulkan driver discovery removes the 256-byte report but does not fix device acquisition. |
| Standalone C Vulkan device request | Native exit 0. GCC ASan creates/destroys the NVIDIA Vulkan instance/device successfully, then exits 1 for the same 184-byte leak. | This leak reproduces without Rust, wgpu, CubeCL or application kernel code. GCC and Rust use different ASan runtimes; this is a control, not an application gate pass. |
| NVIDIA-only EGL vendor discovery, C ASan | Repeated `AddressSanitizer:DEADLYSIGNAL`; stopped the probe and its wrapper. | This discovery change is not a valid fix. The unbounded initial log was reduced to its 20-line head after recording its 465,346,740-byte size. Later probe runs used timeouts and streaming log caps. |
| Minimal safe wgpu request, explicit Vulkan, same Rust ASan binary | RTX 3090 device created successfully. Process still exits 1 for the 184-byte external leak. | Basic wgpu device acquisition works under Rust ASan. This remains a failing process, not a sanitizer pass. |
| CubeCL setup pinned to Vulkan, same Rust ASan binary | Same `Parent device is lost` error and 184-byte leak. | Automatic GL enumeration alone does not explain device loss. Descriptor/runtime differences require further isolation. |
| Slow-unwind standalone C ASan control | Device created; 184-byte leak stack passes through `vkCreateInstance` and driver/library loading. | Allocation happens during instance/driver discovery, before application kernel execution. |

The LLVM-ASan Rust control uses the same locked dependency graph and vendor
patches as the task source, plus a separate diagnostic root package. The two
modes share one compiled executable. Neither changes application test selection.
`XDG_RUNTIME_DIR` was unset for these direct diagnostic runs and produced loader
warnings; the minimal request nevertheless succeeded. The original gate log
remains the authoritative application failure.

## Attribution and unresolved cause

The loader trace maps the 256-byte allocation PC to `libvulkan_lvp.so` at offset
`0x78455`; its disappearance when only the NVIDIA ICD is discovered supports
attribution to unused Mesa software-driver discovery. The 40-byte allocation PC
maps to `libEGL_mesa.so.0` at offset `0x181e9` in the captured loader run. The
144-byte allocation cannot be assigned an exact function/library with confidence:
libraries are unloaded, addresses can be reused, and the fast stack contains a
non-address value. Later reports label the same numerical offset as unrelated
live mappings. Those labels must not be treated as exact allocation provenance.

The slow-unwind C control places both allocations under `vkCreateInstance`.
This establishes an external driver/loader path independently of application
code, but does not prove which upstream library owns cleanup or whether the
allocation is a real lifetime leak versus an unload/reachability interaction.
No suppression was added to hide that uncertainty.

CubeCL 0.11.4's WGSL device request differs from the minimal safe wgpu control in
several ways: it requests nearly all advertised features (excluding
`MAPPABLE_PRIMARY_BUFFERS`), adapter maximum limits, `MemoryHints::MemoryUsage`,
and enabled experimental features. The control uses wgpu's default descriptor.
The experiment **does not isolate feature selection from limits, memory hints,
experimental flags or other setup differences**. No exact feature is identified
as the cause. A future fix must isolate those differences and preserve required
numerical/resource contracts; blindly reducing features would not establish that.

CubeCL does not read `WGPU_BACKEND` for this selection path. The pinned diagnostic
used `WgpuDevice::default().on(WgpuBackend::Vulkan)` and
`try_init_setup::<Vulkan>`. NVIDIA ICD discovery used `VK_DRIVER_FILES`, as defined
by the [Vulkan loader driver interface](https://github.com/KhronosGroup/Vulkan-Loader/blob/main/docs/LoaderDriverInterface.md#overriding-the-default-driver-discovery).
The automatic production qualification remains unchanged.

## Limits and next required work

No additional fix or retry was attempted after the stop instruction. A standalone
ThreadSanitizer preflight had already been attempted without rebuilding std; the
compiler rejected sanitizer ABI mismatch and no executable ran. This is not a
TSan runtime result or an environment blocker. The actual `-Zbuild-std` TSan lane
was not run during this investigation.

An early diagnostic Rust build also failed because a file-size log cap applied
to compiler output files. That control was corrected; the diagnostic test-profile
build then completed. Neither failed setup attempt is application evidence.

P0.05 needs the existing failing GPU ASan test fixed, or a separately reviewed
assurance design backed by stronger evidence. These results do not justify a
blanket GPU sanitizer exemption. Normal CPU/GPU tests remain mandatory, and no
coverage or sanitizer scope was reduced here.
