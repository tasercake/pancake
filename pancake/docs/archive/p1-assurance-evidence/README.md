# P1 headless dependency and host assurance receipts

The 2026-11-18 checks ran on Ubuntu 24.04.5 x86-64, Linux 6.8.0-147-generic,
an AMD Ryzen 9 7950X3D CPU, and a physical NVIDIA GeForce RTX 3090 with
24,576 MiB of VRAM and driver 580.178.04. The Vulkan integration tests admitted
only the NVIDIA ICD via `VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json`.
The run had an owner-only temporary `XDG_RUNTIME_DIR`, and `DISPLAY` and
`WAYLAND_DISPLAY` were unset. The recorded adapter reported `DiscreteGpu`,
`Vulkan`, and PCI device `0000:01:00.0`.

The worktree used the unmodified Rust 1.100.0 and nightly-2026-11-15 toolchain
pins with strict Miri provenance and symbolic alignment checking. The pinned
tools were available under `/tmp/pancake-tools/bin`. Upstream release archives
were checked against their published SHA-256 digests. Cargo Machete 0.9.2 was
built from the locked crate.

| Source revision | Command or profile | Result |
|---|---|---|
| `34dd609dda395171284bf2704b4f6b10dfcea040` | `bash scripts/check.sh fast` | Exit 0. Baseline policy, lint, documentation and workflow checks passed. [Log](p0-fast.log.gz) |
| `009f3e0f774d0227fc4ce93165a97d2adccebdcb` | Strict Linux Miri, library scope | Exit 0. Five pure host tests passed after the graph removed Slint. [Log](p1-initial-headless-linux-miri.log.gz) |
| `009f3e0f774d0227fc4ce93165a97d2adccebdcb` | CPU and physical GPU integration tests | Exit 101. Four test targets retained obsolete Slint imports and failed compilation. [Log](p1-initial-headless-linux-native.log.gz) |
| `f0f32b98e36a189e3d28020bce209c890d5bfd3c` | Same CPU and physical GPU integration tests | Exit 0. All four CPU tests and both NVIDIA Vulkan tests passed with the existing blur, variance, shape, budget and reuse oracles. [Log](p1-boundary-native.log.gz) |
| `4b2f9f745f7a9cfe8da6adcb17a18f39c6f3d9cc` | All-target development Nextest | Exit 0. All 19 tests passed across eight binaries, including native CLI subprocesses and the two Vulkan tests. Zero skips. [Log](p1-cli-dev.log.gz) |
| `4b2f9f745f7a9cfe8da6adcb17a18f39c6f3d9cc` | Strict Linux Miri, library scope | Exit 0. All seven pure host tests passed, including the configuration contracts. [Log](p1-cli-linux-miri.log.gz) |

Commands ran from `pancake/` in a clean worktree:

```bash
export PATH="/tmp/pancake-tools/bin:$PATH"
bash scripts/check.sh fast

export VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json
export XDG_RUNTIME_DIR="$PWD/target/private-xdg"
install -d -m 700 "$XDG_RUNTIME_DIR"
unset DISPLAY WAYLAND_DISPLAY
cargo test --locked --workspace --all-features \
  --test cpu_kernel_contract --test cpu_numerical_contract \
  --test gpu_kernel_contract --test gpu_numerical_contract \
  -- --include-ignored --test-threads=1
cargo nextest run --locked --workspace --all-targets --all-features \
  --cargo-profile dev --profile ci --no-tests=pass --run-ignored all

CARGO_TARGET_DIR="$PWD/target/miri-headless" \
  MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check" \
  cargo +nightly-2026-11-15 miri test --locked --workspace --all-features \
  --lib -- --include-ignored
```

The first-stage CLI lock SHA-256 was
`52095cb40f098ea2c59e54c36a6bcf634db233759e1aff7d57f28ecc2ff02a7c`.
The compatibility tests use the versioned P0 test arrays and their independent
expected values. CLI tests use isolated temporary filesystem fixtures. The CLI
slot at this source started no real processing job with valid inputs.

Required canonical Darwin Miri and retained platform builds remain separate
obligations. The Linux Miri result covers the reported pure host tests. The raw
logs retain first-attempt failures and exact tested source paths.

## First-stage integrated CPU coverage

The unmodified `bash scripts/check.sh coverage-cpu` gate ran on clean source
`98e945f27874c2c8b162ca124d33df23b0fb71a0`, with the same lock hash and pinned
Linux toolchain. It exited 1 after all instrumented CPU tests passed. The LLVM
report recorded 380/404 lines, 33/39 functions and 365/404 regions. The stable
coverage profile did not instrument branches. The core kernels and configuration
types had full line, function and region coverage. The CLI main and input path
resolution still had uncovered paths, including a valid job reaching its
processing diagnostic and file discovery error or limit handling.

[Gate log](p1-first-stage-coverage-cpu.log.gz) and
[raw LLVM report](p1-first-stage-coverage-cpu.json.gz) retain this result.
