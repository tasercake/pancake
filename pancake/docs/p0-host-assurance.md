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

The routing and source-universe guards have nine focused regression tests,
including native-only host regions, unknown integration targets and a changed
production source layout. On the initial blur-only source snapshot, CPU/native
line/function/region coverage and nightly branch coverage passed the comparison;
all ten real-tool guardrail outcomes passed. Final source hashes and actual Linux
sanitizer outcomes will be recorded after the numerical/budget implementation is
integrated. Those results are not claimed by this initial routing change.

Windows workflow syntax was checked with pinned actionlint. An actual MSVC build
has not been run locally; its result must come from the required Windows job.
No branch was pushed or remote workflow dispatched by this agent.
