# P0 host memory-checking boundary

The adopted smaller P0 checks all authored production Rust host code, including
kernel IR construction, allocation, launch and readback, with CPU execution under
Linux AddressSanitizer and ThreadSanitizer. Native Apple/NVIDIA GPU qualification
remains mandatory. This is not a claim that sanitizers instrument GPU shaders,
LLVM JIT machine code, prebuilt LLVM libraries or proprietary device drivers.

## Gates

| Entry point | Required behavior |
|---|---|
| `check.sh test`, `gpu`, `all` | Physical GPU tests are real tests, never ignored or silently replaced by software. `test` and `all` run every integration target; `gpu` explicitly requires a hardware target. |
| `check.sh test-cpu`, `sanitizer address`, `sanitizer thread` | Run library tests and every `cpu_*` integration target. Shared host launch/readback code is exercised through the CPU runtime, not replaced with a scalar twin. |
| `check.sh coverage`, `branches` | Both CPU-only and native suites must independently reach zero uncovered lines/functions/regions/branches. Exact covered production function/region identities in native coverage must also occur in CPU coverage. |
| Hosted CI CPU assurance | Linux/macOS use CPU test profiles; Linux retains actual ASan/TSan. New source/test layouts fail closed. Native GPU qualification is a separate physical-hardware gate; hosted CPU jobs do not claim hardware support. |
| Windows MSVC build/link | Required Windows job compiles and links all targets with `cargo test --no-run`; Windows runtime testing is best effort and is not run by this job. |

The source-universe comparison covers current production `src/` and `build.rs`.
A new authored source layout inside this package is rejected for review instead
of silently omitted. Integration test harnesses are not production source; their
GPU setup/assertions stay in the mandatory hardware suite. No LLVM coverage
source exclusion or ignored test was added. Generic function names and source
regions are compared, so GPU-only host instantiations cannot disappear simply
because the CPU route selects different integration binaries.

Every first-party integration test target must be named `cpu_*` or `gpu_*`.
Unclassified targets fail before the selected profile starts. CPU unit tests are
always included with `--lib`; `gpu` fails if no physical-GPU target exists.
Doctest coverage is accumulated in a separate documentation-only nightly run,
because pinned cargo-llvm-cov forbids combining `--lib` and `--doctests`.

A future production GPU-only host path must either execute safely in the memory-
checking boundary or receive a new explicit assurance design. Moving its test to
a GPU target does not satisfy the CPU source-universe comparison. The current
proof does not extend automatically to separate qualification workspaces or
application modules that have not yet been implemented.

## Known GPU sanitizer defect remains open

`check.sh sanitizer-gpu address` retains an explicit instrumented GPU profile for
P9 follow-up. The prior default CubeCL setup fails device acquisition under ASan;
a minimal safe wgpu device request succeeds, and a separate 184-byte loader-path
leak reproduces in standalone C. Descriptor features, limits, memory hints and
experimental flags were not isolated. The
[original investigation](p0-sanitizer-investigation.md) remains authoritative.
No suppression, source-level sanitizer conditional or blanket claim that GPUs
cannot run with ASan was introduced.

## Validation status

The final compiled core includes the numerical/budget change `572e601` and shape
error oracle `65185d9`. The retained
[receipt](p0-host-assurance-evidence/receipt.json) records exact source, lockfile,
binary and runtime-library hashes. Compiled Rust/Cargo hashes match the final
checkout. The later `1f21cb9` change fixes only Bash 3.2 native nightly dispatch;
it changes no compiled source or sanitizer argument.

| Actual check | Result |
|---|---|
| Linux ASan, normal owned task container | **Pass:** five host unit tests plus two CPU kernel and two CPU numerical tests; exit 0; no memory/leak report. Std and project rebuilt with ASan. |
| Linux TSan, default container and privileged exec | **Did not execute tests:** both stop before the first test with exit 66 because inherited seccomp prevents TSan's ASLR compatibility setup. These are retained failures, not race results. |
| Linux TSan, isolated exact-binary route | **Pass:** same nine tests, exit 0, no race report. Three binaries and five runtime libraries were copied without rebuilding, hash-verified, and mounted read-only. |
| Parent final full-all, tested source `85ae943` | **Pass via unmodified pre-push hook:** CPU/native 139/139 lines, 14/14 functions, 143/143 regions, 14/14 branches; source-universe guard passed. 79/79 mutants caught, five strict Miri tests, 11 native tests per debug/release/layout profile, ten guardrail outcomes. |

The TSan qualification container used the exact existing base image, UID 65534,
zero capabilities, no network, no-new-privileges, read-only inputs/root filesystem
and a small writable tmpfs. Only its seccomp policy was unconfined so the sanitizer
could perform its own process-level address-layout setup. No host sysctl, driver
or persistent container privilege/configuration was changed. The ephemeral
container and duplicate staged binaries were removed after evidence retention.
This is an actual isolated TSan pass, not a claim that the default container's
`check.sh sanitizer thread` succeeded.

The routing/source guards have nine focused regression tests; all ten real-tool
guardrail outcomes passed. One shared-cache coverage collection contained foreign
worktree paths and was rejected. It is not a source finding or accepted coverage
result. Instrumented coverage/profile collection must have one owner; only normal
build/test caches may be shared across parallel worktrees. No source exclusion was
used to hide that contamination.

Windows workflow syntax passed pinned actionlint. The
[final integration full-all receipt](p0-close-evidence/local-aggregate.json)
is retained by its producer. Actual MSVC build/link results remain separate and
are not claimed here. This agent pushed no branch or workflow dispatch.
