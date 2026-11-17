# P0 hardware and native test-lane audit

> Current scope update, 2026-11-17: AMD/Intel GPU and Windows execution/testing
> are best effort; build support remains required. Missing access alone no longer
> blocks P0. The [implementation plan](implementation-plan.md#testing-scope--adopted-user-decision-2026-11-17)
> is authoritative. Observed results below are retained as historical evidence.

Date: 2026-11-17, Asia/Singapore. Source revision: `8fcbc52` (P0 planning
baseline over `00bd137`). Audit worktree: `codex/p0-hardware`.

## Original audit verdict

## Observed local resources

| Resource | Observation | What this establishes |
|---|---|---|
| Host | macOS 26.2, build 25C56; Darwin 25.2.0, arm64; Mac16,7 | A native ARM64 macOS lane is present. |
| CPU | Apple M4 Pro, 14 cores (10 performance + 4 efficiency) | Physical ARM64 CPU identified; native CubeCL CPU execution was not tested by this audit. |
| GPU | Apple M4 Pro, 20 cores; system profiler reports Metal 4 support | Physical Apple GPU identified; neither native CubeCL Metal nor current wgpu execution was tested here. |
| Memory | 51,539,607,552 bytes (48 GiB) | Capacity for local feasibility workloads; this is not a measured application memory budget. |
| Rust | Only `aarch64-apple-darwin` target installed for the default toolchain and pinned 1.100.0 | No installed x86-64 target was found for those toolchains. A cross-build or emulator would not prove execution on physical x86-64 hardware. |
| Xcode | Selected developer directory is `/Applications/Xcode.app/Contents/Developer`; Swift/Clang found there | Native compiler tools are installed. Default SDK-version resolution initially failed; uncached explicit macOS SDK query succeeded with 26.4. |
| Docker | Client/server 29.4.0, active OrbStack context; Linux engine reports aarch64, 14 CPUs, 16,819,834,880 bytes | A responding ARM64 Linux engine exists. No physical NVIDIA/AMD/Intel device or GPU runtime was established. It is not native x86-64 evidence. |

Machine serial numbers, hardware UUIDs, network endpoints, and private-key data
are deliberately excluded from this report.

## Existing virtualization and connection inventory

The inspected `~/.ssh/config` contains an OrbStack include, and the included
configuration declares only the `orb` host. No external x86-64/GPU SSH alias was
found. This is not proof that no remote workstation exists; it means this audit
did not establish its current authorized endpoint or access method.

That initial configuration-only inventory is superseded for NVIDIA access by
the [runtime audit's remote addendum](p0-runtime-feasibility.md): the existing
authorized hostname `krishna-desktop` responded to a BatchMode SSH probe with
Linux 6.8.0-147-generic x86-64, NVIDIA GeForce RTX 3090, driver 580.178.04,
24576 MiB. No installation, compilation, runtime kernel or GUI job was executed.
An absent SSH-config alias was not proof that this host was inaccessible.

**Inventory command side effect:** `orb list` unexpectedly attempted to start
OrbStack's VM and returned `start VM: timed out waiting for VM to start`.
Subsequent read-only Docker queries received engine responses. No conclusion is
drawn about whether the engine was already running or was started by a command.
No further OrbStack machine-start/stop commands were issued. Avoid using that
command as a guaranteed passive preflight in repeat audits.

## Linux physical-GPU GUI route

Source inspection establishes the following route, without executing native
Linux input or an application:

| Owning source | Observed behavior |
|---|---|

On a provisioned Linux GPU host, run the same descriptor without `--gui-vm`:

The private display setup contains Qt environment variables because it also
supports the current reference. Their presence is not proof that Slint works;
Slint's required renderer/accessibility backend must be tested unchanged through
this generic native driver. An offscreen compute path plus software presentation
can still provide genuine GPU fusion, provided telemetry is truthful.

## Historical evidence versus current access

This audit did not replay that run, inspect the machine, or establish its current
connection. The record does not qualify Pancake, CubeCL CUDA, or present hardware
availability. Hosted CI definitions are also not evidence of a physical GPU:
current CI runs Linux/macOS regressions and Linux GUI smoke, but no advertised
NVIDIA/AMD/Intel physical-GPU runner is configured there.

## Original full-matrix prerequisites — testing scope superseded

| Lane | Available now | Remaining requirement |
|---|---|---|
| Apple ARM64 CPU/wgpu/native Metal | Local physical host identified | Execute shared kernel battery and qualify native runtime/JIT/package behavior. Use dedicated isolation for native input. |
| Native x86-64 + NVIDIA CUDA/wgpu | Existing remote RTX 3090 host reachable; Linux x86-64/device/driver read-only probe passed | Verify native CPU/CUDA/wgpu toolchains, isolated desktop/runtime tools, real kernels and physical-GPU GUI completion. Access alone does not qualify execution. |
| AMD HIP/wgpu | Not established | Physical supported AMD device, ROCm/HIP and wgpu driver, native execution and required desktop access. |
| Intel wgpu | Not established | Physical Intel GPU and supported Vulkan/D3D12 driver, actual kernel execution and package validation. |
| macOS GUI VM | Named guests exist; one unrelated guest is running | Dedicated guest ownership/lease, valid key/login, permissions, native compiler/runtime installation, and reproducible binary preparation. |
| Promised Windows/macOS x86 platform coverage | No native lane established by this audit | Actual supported machine/runner access for builds, runtime execution, native libraries, and packages; cross-compilation alone is insufficient. |

## Reproduction and scope

- `bash scripts/check.sh all` passed with existing installed tool executables on
  PATH. This worktree owns its generated Cargo outputs; no tool policy was changed.
- `git diff --check` and local Markdown source-link checks passed.

## Execution update: isolated Linux/NVIDIA lane

The earlier inventory-only status is superseded for the available Linux host.
No host desktop input or host package/driver installation was used. Passwordless
sudo is unavailable; Docker group access and the installed NVIDIA container
runtime provide an isolated qualification route instead.

A task-owned container, `pancake-p0-linux-20261117`, uses existing
`node:22.23.4-bookworm`, image
`sha256:a62a6676db6b68fd1aa959a36ce361a002b74f0936d3e38db544420d9b164990`.
It exposes GPU compute/utility/graphics, with no host display, home, credential or
project mount and no published port. Source copies come from independent P0
worktrees. Existing unrelated containers, applications and VMs remain untouched.

| Qualification | Observed result |
|---|---|
| Native build tools | Isolated Rust 1.100.0 installed; NVIDIA driver 580.178.04 and CUDA 12.5 already exist on host. |
| Private desktop | Xvfb 21.1.7, Openbox 3.6.1, xdotool, private D-Bus, AT-SPI 2.46.0 installed inside container. |
| Actual suite boundary | Unchanged `isolated_desktop` allocated 1440x1000 display, private service home and accessibility bus; all owned desktop processes reaped on exit. Same private environment sees physical RTX 3090 Vulkan. |
| Suite self-tests | 309 passed, two explicit live-GUI smoke skips. These test the harness, not Pancake. |
| Shared blur: native x86-64 CPU | Exact-value/tail/border tests pass using LLVM CPU JIT; identity AMD Ryzen 9 7950X3D, `cpu_x86_64_load-512_f16-chain`. |
| Shared blur: wgpu Vulkan | Same tests pass; adapter NVIDIA GeForce RTX 3090, `DiscreteGpu`, Vulkan. Software fallback is explicitly rejected. |
| Shared blur: native CUDA | Same image definition passes exact-value/border/tail qualification with CUDA runtime enabled. Identity RTX 3090, `ptx_sm86`; existing NVRTC 12.5 libraries copied into container. No driver install. |
| Shared blur: Apple native Metal | Same image definition passes on local M4 Pro through `cubecl/metal-native`, separate from Metal through wgpu. Identity `msl_Apple M4 Pro`. |

Host PCI inventory exposes one GPU: NVIDIA RTX 3090. Vulkan also reports a
software llvmpipe adapter; it is not AMD/Intel GPU evidence. Existing libvirt
inventory has no VM. No AMD GPU, Intel GPU, Windows or macOS guest login lane has
been established by that Linux check. The later dedicated guest qualification
supersedes only the SSH/Aqua access gap, not current AX/Automation identity. User confirmed no AMD/Intel/Windows test hosts or runners while independent
work continued; P0.02 remains blocked. These conditions cannot be removed by dependency exceptions.

Dedicated guest access is now verified separately in
[p0-guest-access-qualification.md](p0-guest-access-qualification.md). The suite-managed
key exists in its cache even though the descriptor-specific `~/.ssh` path was
absent. This closes SSH/Aqua access only; current native permission identity and
actual Pancake GUI remain unqualified.
