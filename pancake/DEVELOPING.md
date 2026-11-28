# Developing Pancake

## Build from source

Install [Rust with rustup](https://rustup.rs/). The Pancake workspace selects
the toolchain pinned in [rust-toolchain.toml](rust-toolchain.toml).

| Platform | Desktop build prerequisites |
| --- | --- |
| macOS on Apple Silicon | Xcode with its command-line tools selected, and Zstandard (`brew install zstd`). |
| Ubuntu 24.04 x86-64 | A C/C++ toolchain, `pkg-config`, and the packages in [ci-linux-native-deps.sh](scripts/ci-linux-native-deps.sh). |
| Windows x64 | Visual Studio Build Tools with **Desktop development with C++**, the Windows SDK, and the x64 MSVC Rust host toolchain. |

Clone the repository and build the desktop app and CLI.

```sh
git clone https://github.com/tasercake/pancake.git
cd pancake/pancake
cargo build --locked --release --workspace --bins
```

Launch `target/release/pancake-gui`, or `target\release\pancake-gui.exe` on
Windows. To check the CLI with the bundled synthetic frames, run this command
from the same directory. It writes `stacked.tif` in the current directory.

```sh
cargo run --locked --release -p pancake-cli -- --cpu --input ../assets/quickstart/focus-near.png ../assets/quickstart/focus-far.png --output stacked.tif
```

For CLI-only builds, use `cargo build --locked --release -p pancake-cli` to
skip compiling the desktop dependencies. Add `--timings` to a Cargo build to
see per-crate compile times in the generated HTML report.

For packaged builds, use the [release guide](docs/releases.md).

GitHub Actions caches compiled dependencies by the dispatched source's Cargo
graph and native toolchain. An exact candidate build-input match also reuses
workspace outputs. A new candidate version can still reuse its dependencies.
Cargo checks the lockfile and fingerprints on every run, and each package is
staged and smoke-tested again. Change the cache key version in the workflow
when changing how build outputs are produced or stored.

For a clean compile benchmark on the package runners, use
`gh workflow run pancake-release.yml --ref <branch> -f mode=build-only -f cold_build=true`.
Package jobs skip cache restores and saves and upload per-platform Cargo
timing reports beside the verified packages.

The release profile applies cheaper optimization to the Slint compiler, the
host-side CubeCL and LLVM IR crates, and the largest native platform bindings.
Use the release profile when benchmarking application throughput so the
measurements match the shipped CLI and desktop app.

## Tests and assurance

Run the Lightroom plug-in contract checks from `pancake/` with `luajit lightroom/test_open_stack.lua` or Lua 5.1.

From the `pancake/` directory, install the pinned tools once with `bash scripts/bootstrap.sh`.
The script uses Bash, rustup, jq and Python 3.11 or newer. It places Cargo tools
in `target/tools`. The lint gates also use actionlint and ShellCheck at the
versions in [tool-versions.sh](scripts/tool-versions.sh).

| Command | Scope |
|---|---|
| `bash scripts/check.sh` or `bash scripts/check.sh test` | Everyday loop. Debug Nextest for the core and headless CLI, including real CPU kernels, registration, fusion, codecs, cancellation, resource limits, and CLI integration. No display or physical accelerator is needed. |
| `bash scripts/check.sh lint` | Repository policy and script regressions, formatting, and Clippy for the core and CLI. |
| `bash scripts/check.sh test-gui` | GUI package tests, including the Slint workspace and software-renderer contracts. Use a supported desktop environment for tests of native window integration. |
| `bash scripts/check.sh gpu` | Physical-GPU integration tests in debug and release. Validation fails if the runtime falls back to software. |
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
| `release` | Platform, native macOS and Windows builds, policy, nightly layout and Miri checks, and Linux address and thread sanitizers. |
| `all` | Every hosted assurance group. |

Run physical GPU and native desktop checks on a suitable machine, especially
for changes to device routing, kernels, rendering, or desktop workflows. Local
commands also expose `coverage`, `branches`, `deps`, `guardrails`, and
`sanitizer address` or `sanitizer thread` on Linux x86-64. Coverage thresholds,
the [Miri host contract](docs/archive/p0-miri-host-boundary.md), and
[dependency policy](docs/dependency-policy.md) apply when their gates run.

Full workspace mutation analysis is an explicit local or dedicated-runner
operation. Start with an affected file or shard and keep the full CPU suite for
each mutant, for example
`bash scripts/check.sh mutants-cpu --file src/lib.rs`. Omit the selector for the
full roster. The routine lane keeps the substantive CPU cases, and the
specialized lanes retain the hardware and instrumentation checks. The runtime
savings come from avoiding repeated builds and test passes on every change.

The [engineering archive](docs/archive/) retains the implementation plan,
qualification reports, and test evidence.
