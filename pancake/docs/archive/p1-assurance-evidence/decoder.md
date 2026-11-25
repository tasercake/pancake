# P1 decoder-integrated assurance

The host and tools match the [earlier assurance receipt](README.md). Commands
ran from `pancake/` with `PATH=/tmp/pancake-tools/bin:$PATH` and
`CARGO_BUILD_JOBS=4`. The native profile unset both desktop display variables,
restricted Vulkan to the NVIDIA ICD, and used private XDG runtime and cache
directories. Tests exercised the existing independent P0 kernel oracles and
test-owned real CLI and malformed codec files.

| Check | Result |
|---|---|
| `cargo nextest run --locked --workspace --all-targets --all-features --cargo-profile dev --profile ci --no-tests=pass --run-ignored all` | Exit 0. All 27 tests passed with zero skips, including actual CPU and NVIDIA Vulkan blur/variance. [Log](decoder-native-dev.log.gz) |
| `bash scripts/check.sh fast` | Exit 0. Lint, rustdoc, policy and workflow checks passed. [Log](decoder-fast.log.gz) |
| `bash scripts/check.sh deps` | Exit 0. Dependency advisory, source, duplicate and unused-dependency checks passed. [Log](decoder-deps.log.gz) |
| `bash scripts/check.sh coverage-cpu` | Exit 1. All instrumented CPU tests passed. The production report recorded 522/824 lines, 60/128 functions and 524/893 regions. [Log](decoder-coverage-cpu.log.gz) [Raw LLVM report](decoder-coverage-cpu.json.gz) |

Kernel and shared configuration type summaries were complete. Coverage
deficits appeared in the CLI entry point (71/78 lines), path resolver (87/108),
raster input (69/149), NumPy input (10/133) and RAW input (25/96). The stable CPU
coverage run collected no branch instrumentation. Coverage, mutation, nightly
layout and required platform assurance remain open at this revision.
