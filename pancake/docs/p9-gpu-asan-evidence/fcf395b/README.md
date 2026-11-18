# Integrated Linux GPU AddressSanitizer diagnosis

The unchanged GPU ASan gate failed at exact source
`fcf395b280f9cbdd4b4ee8c7ea41d40523c1c722` and Cargo.lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
All six targets compiled under the pinned Rust nightly. The gate reached
`gpu_advanced_fusion`, whose single test failed during Vulkan device
acquisition. The other five targets remained unrun by the gate. A separate
focused run of the unchanged runtime contract reproduced the same error.

| Process | Device or kernel result | Exit | LeakSanitizer |
| --- | --- | ---: | --- |
| Focused application runtime contract | `Parent device is lost` during physical wgpu-Vulkan acquisition. The kernel and its hardware receipt were never reached. | 1 | 4,868 bytes, 8 allocations |
| Rust ASan minimal wgpu mode | Created the RTX 3090 Vulkan device. | 1 | 2,522 bytes, 5 allocations |
| Rust ASan direct CubeCL mode | The same executable reported `Parent device is lost` during device acquisition. Its kernel was never reached. | 1 | 4,868 bytes, 8 allocations |
| Standalone native C | `vkCreateDevice=0` on RTX 3090. | 0 | Native control |
| Standalone GCC ASan C | `vkCreateDevice=0` on RTX 3090. | 1 | 2,522 bytes, 5 allocations |
| Full integrated GPU ASan gate | First test failed during acquisition with `Parent device is lost`. | 1 | 4,868 bytes, 8 allocations |

The C and minimal wgpu ASan reports match across all five allocation sizes,
including the same D-Bus allocation frames. The C execution independently
reproduces this leak path without the application or Rust. The three
failed CubeCL acquisition processes share the larger report and include
a second D-Bus allocation path with a frame in `libnvidia-gpucomp`.
The matching stacks identify the external allocation path. The specific
device descriptor condition responsible for the acquisition failure and
ownership of the external allocations' cleanup remain unresolved.

The system used Linux x86-64, driver `580.178.04`, RTX 3090 at PCI
`0000:01:00.0`, and `libvulkan1` 1.3.275.0. The replay restricted Vulkan
discovery to the recorded NVIDIA ICD and used a private D-Bus session and
cache for each command. Sanitizer controls retained their default diagnostics,
with no suppressions. The Rust and C control sources come from the earlier
audit. The temporary Rust control example preserves the archived behavior
and adds crate-use imports for the unchanged workspace lints.

[Receipt](receipt.json) and [raw records](raw-records.tar.gz) retain the
original test selectors, exact commands, test and process outcomes,
untruncated logs, lock/source and binary identities, and hardware provenance.
The archived replay script separates the build stage from the physical
execution stage. Use the stated source revision and place the scripts under
`artifacts/p9-gpu-asan-prep` in its checkout. Run `replay.sh build` before
`replay.sh run`. The [P0 Rust diagnostic](../../p0-sanitizer-investigation.md)
and [P9 C control](../receipt.json) preserve the earlier results.
