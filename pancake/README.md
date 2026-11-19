# Pancake application

Rust 2024 foundation and P0 image-kernel qualification for Pancake. Slint 1.19,
CubeCL 0.11.4 and wgpu 30 share one dependency graph. One safe CubeCL kernel
applies a full-precision u16 horizontal blur; a second computes stable f32 row
variance. Both use checked logical byte budgets on CPU LLVM JIT and GPU runtimes.

## Tests and assurance

From this directory, install the pinned tools once with `bash scripts/bootstrap.sh`.
The script uses Bash, rustup, jq and Python 3.11 or newer. It places Cargo tools
in `target/tools`. The lint gates also use actionlint and ShellCheck at the
versions in [tool-versions.sh](scripts/tool-versions.sh).

| Command | Scope |
|---|---|
| `bash scripts/check.sh` or `bash scripts/check.sh test` | Everyday loop. Debug Nextest for the core and headless CLI, including real CPU kernels, registration, fusion, codecs, cancellation, resource limits, and CLI integration. No display or physical accelerator is needed. |
| `bash scripts/check.sh lint` | Repository policy and script regressions, formatting, and Clippy for the core and CLI. |
| `bash scripts/check.sh test-gui` | GUI package tests, including the Slint workspace and software-renderer contracts. Use a supported desktop environment for tests of native window integration. |
| `bash scripts/check.sh gpu` | Physical-GPU integration tests in debug and release. Validation fails if the runtime falls back to software. See the [hardware lanes](docs/p0-hardware-test-lanes.md). |
| `bash scripts/check.sh test-cpu` / `test-release` | All CPU tests, including the GUI package, for the debug or release profile. |
| `bash scripts/check.sh test-full` | All workspace targets in debug and release, plus doctests. If any workspace package declares Cargo features, also runs the feature powerset. Requires a physical GPU. |
| `bash scripts/check.sh fast` | Workspace-wide lint and private Rust documentation in addition to the policy checks. |
| `bash scripts/check.sh all` | Run the full local gate sequence, including physical GPU, coverage, mutation, and platform-appropriate nightly checks. |

**Pancake required** runs `lint` and `test` on pull requests and pushes to
`main`. It uses one headless Linux runner, with no native desktop package
provisioning. Pushes to PR branches use the pull request run. Git hooks
check staged whitespace and formatting and run no tests. Install the hooks
from the repository root with `git config --local core.hooksPath pancake/.githooks`.

Use the **Pancake assurance** manual workflow for checks appropriate to a
change or release.

| Workflow input | Checks |
|---|---|
| `platform` | Linux and macOS workspace lint plus debug and release CPU suites, including the GUI crate. Windows builds and links every test target with MSVC. |
| `coverage` | Independent stable line/function/region coverage and nightly branch coverage on Linux. |
| `nightly` | Canonical macOS Miri, randomized layouts, and macOS branch coverage. Linux runs randomized layouts in a separate job. |
| `sanitizers` | Independent Linux address and thread sanitizer jobs for CPU targets. |
| `policy` | Advisory and unused-dependency checks, vendored patch fixtures, and lint, coverage, and mutation guardrail fixtures. |
| `all` | Every hosted assurance group. |

Run physical GPU and native desktop checks on a suitable machine, especially
for changes to device routing, kernels, rendering, or desktop workflows. Local
commands also expose `coverage`, `branches`, `deps`, `guardrails`, and
`sanitizer address` or `sanitizer thread` on Linux x86-64. Coverage thresholds,
the [Miri host contract](docs/p0-miri-host-boundary.md), and
[dependency policy](docs/dependency-policy.md) apply when their gates run.

Full workspace mutation analysis is an explicit local or dedicated-runner
operation. Start with an affected file or shard and keep the full CPU suite for
each mutant, for example
`bash scripts/check.sh mutants-cpu --file src/lib.rs`. Omit the selector for the
full roster. The routine lane keeps the substantive CPU cases, and the
specialized lanes retain the hardware and instrumentation checks. The runtime
savings come from avoiding repeated builds and test passes on every change.

The [implementation plan](docs/implementation-plan.md) covers the application architecture and implementation milestones. The [P0 work log](docs/p0-work-log.md) consolidates
current and historical proofs, failures and later acceptance owners.
