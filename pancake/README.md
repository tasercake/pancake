# Pancake application

Dependency-free Rust 2024 scaffold for the Pancake focus-stacking application.
Implementation has not started. The library contains only crate documentation;
there are no GUI/GPU libraries, codecs, APIs, or placeholder tests.

From this directory:

```sh
bash scripts/bootstrap.sh
bash scripts/check.sh all
```

Scripts require Bash (Git Bash on Windows), rustup, jq, and Python 3.11 or newer. Bootstrap installs
pinned Rust tools into `target/tools`, not the global Cargo tool directory.
Install actionlint 1.7.12 and ShellCheck 0.11.0 from official releases or a package
manager. Rust 1.100.0, nightly-2026-11-15, and development tools are pinned.
Missing tools fail checks rather than silently disabling them.

| Gate | Standard |
|---|---|
| Compiler and Clippy | Warnings are errors; all, pedantic, nursery, cargo, and selected restriction lints. Unsafe Rust forbidden; checked numeric operations; public/private documentation required. |
| Format and docs | Stable rustfmt, strict rustdoc including private items, ShellCheck and actionlint for infrastructure. |
| Tests | Debug/release native tests, doctests, exhaustive feature combinations, ignored tests included, no retries, bounded test runtime. |
| Coverage | Zero uncovered instrumented lines, regions, functions, and branches; nightly includes doctests. Only the exact empty scaffold is exempt. |
| Mutation | Surviving mutations fail. A deliberately weak test fixture proves this gate rejects inadequate assertions. |
| Dependencies | Locked resolution; advisories, unmaintained/unsound/yanked crates, unknown sources, wildcards, duplicate versions, and unused dependencies fail except exact, expiring reviewed exceptions. License declarations are inventoried without a license gate. |
| Runtime | Pinned-nightly Miri and two randomized layout seeds; Linux address/thread sanitizers in CI. |

Dependency reviews live in [dependency-policy.json](dependency-policy.json).
[Policy enforcement](docs/dependency-policy.md) explains exact-version/parent checks,
expiry enforcement, protected shared GPU/window boundaries, and the informational
license inventory. Existing local hooks and CI gates invoke these checks.

Run individual gates with `bash scripts/check.sh fast`, `test`, `coverage`,
`branches`, `deps`, `mutants`, `nightly`, or `guardrails`. Linux x86_64 also supports
`bash scripts/check.sh sanitizer address` and `thread`. `bash scripts/check.sh all` runs every
local gate. No executable application code exists, so no test count or coverage
percentage can yet be claimed. Coverage cannot prove correctness or performance.

Clippy's full restriction group is intentionally not enabled: some lints conflict.
Selected high-value restrictions are denied. Exceptions must use a narrow,
reasoned `expect`; broad `allow` attributes and stale expectations fail.

Install hooks from the repository root:

```sh
git config --local core.hooksPath pancake/.githooks
```

Pre-commit checks staged Pancake changes and rejects partially staged project
changes. Pre-push runs the complete local suite for pushes that change Pancake,
and requires the checkout to match the pushed code. Git hooks are bypassable;
server enforcement requires making **Pancake required** a required repository
status check. No repository ruleset is changed by this scaffold.

CI runs on every push and pull request, daily, and manually. Actions use immutable
commit pins and read-only permissions. Property tests, input fuzzing, CPU/GPU
parity tests, and benchmarks become mandatory when their actual APIs exist.

The [implementation plan](docs/implementation-plan.md) covers the application architecture and implementation milestones. It adds no application behavior to this scaffold.
