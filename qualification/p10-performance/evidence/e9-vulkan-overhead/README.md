# E9 Vulkan startup diagnostic

**Status** Local prototype. The 12f2151 package and frozen gate record use the production lock and executable. Vulkan E9 warm remains failed at 0.559721 s / 0.793928 s (0.705×, limit 0.50×) and RSS at 396,156,928 B / 550,313,984 B (0.720×, limit 0.50×). The cold median passed. Peak GPU memory remains pending due to acquisition cadence. See `integrated-gate-extract.json` for the historical comparison provenance.

## What accounts for the startup and RSS

Stock release `12f2151c8a198b5209c8f4d506dbd17fd9aaa74f` opened the Vulkan loader twice in a single completed exposure job. In the first trace the opens were at 13:22:54.590955 and 13:22:54.939123 local time. CubeCL's pinned Vulkan path probes for an API using a temporary instance before handing the same API to `request_adapter`. That step creates the retained instance and independently enumerates its indexed adapter. The temporary call enumerated NVIDIA and installed Mesa ICDs and mapped NVIDIA's rtcore before the retained instance was created.

The first `/proc/<pid>/smaps` sample reached 398,040 KiB of mapped RSS late in the stock job. Its largest mappings were heap 79,140 KiB, NVIDIA rtcore 67,500 KiB, CLI pages 44,956 KiB, Mesa-loaded LLVM 44,412 KiB, nvidiactl 41,448 KiB, NVIDIA GPU compiler 29,356 KiB and anonymous mappings 20,868 KiB. These sampled mapping totals include shared and device mappings and do not measure allocation ownership inside libraries. The retained Vulkan instance still maps the NVIDIA and Mesa libraries after removing the probe. Maximum `wait4` RSS in the paired diagnostic moved by 1,436 KiB, leaving a substantial gap to the frozen RSS target.

## Narrow experiment

`prototype-v2.patch` applies to the crates.io `cubecl-wgpu 0.11.4` source with checksum `413caaf7eaf52cb5922f1e9e766fea404fd1dd549d10f3500e22b61c70b63a10`. It uses the existing retained instance's enumeration for a pinned Vulkan discrete or integrated ordinal. The enabled graphics API check runs before setup, and the retained enumeration preserves the no-adapters and ordinal errors. Device requests, feature negotiation, queue reuse and the physical qualification kernel all use their existing paths. Other backend candidates and the regular driver inventory retain their usual rules. The experimental lock changes only the source and checksum lines for the explicitly patched package. An earlier reconnaissance build re-resolved three unrelated lock edges and was discarded for source attribution.

On the NVIDIA GeForce RTX 3090 at PCI `0000:01:00.0` with driver 580.178.04, v2 reduced driver loader opens from two to one. Three alternating observations after discarding a first pair gave a median 0.631613 s stock and 0.517950 s patched (18.0% faster). Maximum `wait4` RSS was 370,352 and 368,916 KiB respectively. The byte-encoded TIFF was identical within each backend across executables. Both executables returned the same CLI errors for index 8 with one visible physical adapter and for index 0 with an intentionally unavailable test driver manifest. The negative driver-manifest override was scoped to that subprocess. Each successful GPU job reported `wgpu-vulkan`, a physical RTX 3090 and no fallback.

The Ryzen 9 7950X3D host ran Ubuntu 24.04.5 with Linux 6.8.0-147 x86-64. All comparisons used the nine original Balloons Erik Reinhard PNGs from the 69,869,523-byte D28 MEFDatabase archive and checked all input hashes before and after. `input-receipt.json` records the original archive, input order, byte sizes and hashes. The command used first-reference Euclidean registration, exposure fusion, kernel 6, eight pyramid levels, scale 6, automatic crop and RGB16 TIFF. The results used an instrumented diagnostic workflow with concurrent software-GUI and CPU work, per-process compute-cache paths, and three retained observations per executable. The frozen quiet, thermal, resource acquisition and reference-pair gates were out of scope.

The stock CLI SHA256 was `32caffd65ed785b1883c39ec69cfd726f2a6f836e42c653a6385dacf80267055`. The patched CLI SHA256 was `e2ede18e6236a37e41332a6d44ec0f7d263cd21e4cc291a313bc2beeb496db46` and its lock SHA256 was `bc328dbe8185901d2ca1f21f82b0dc59201afb43997eed1c0fca53fbca1c590f`. The existing `cpu_runtime_contract` passed 5/5 and `gpu_runtime_contract` passed 4/4 on physical Vulkan, including actual blur output, device/queue reuse and budget rejection. The build logs, binary hashes and GPU hardware receipt accompany the test results.

## Reproduce

On a detached disposable checkout at the stock revision, apply `prototype-v2.patch` with `patch -p1` to a copy of the exact crate from the registry and apply `experimental-lock.patch` with `patch -p1` from `pancake/`. From `pancake/`, using the absolute experiment crate directory in `PANCAKE_EXPERIMENT_WGPU`, build with the command below. The build used Rust 1.100.0 and the release profile.

```sh
CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 cargo +1.100.0 \
  --config "patch.crates-io.cubecl-wgpu.path='$PANCAKE_EXPERIMENT_WGPU'" \
  build --offline --release --locked -p pancake-cli
```

Run `python3 compare_startup.py STOCK_CLI PATCHED_CLI E9_PHOTO_DIRECTORY NEW_OUTPUT_DIRECTORY` from the evidence directory. The script uses ordinary driver discovery and validates the physical receipt, byte-encoded output and exact input hashes. It runs `strace -ff -tt -e trace=openat,openat2,mmap,munmap -s 224` for the loader witness and samples procfs in separate jobs. The compact trace archive preserves the exact commands, physical completion events, per-process memory and output hashes. TIFF outputs were deleted after hashing.

Promotion would require dependency-patch review and an exact integrated P9/P10 rerun. A future frozen run would need to show the required median and RSS with matching GPU cadence. This branch adds evidence and a reproducible prototype patch only.
