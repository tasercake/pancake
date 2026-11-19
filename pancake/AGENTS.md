# Pancake engineering rules

## Code

- Write a meaningful failing test before every behavior change. Cover real
  contracts, errors, boundaries, cancellation, and resource limits.
- Inherit workspace lints in each new crate. Commit the lockfile. Never weaken
  lints, coverage, tool pins, or dependency policy merely to pass a check.
- Unsafe Rust is forbidden. Future FFI needs an explicit boundary design and
  reviewed policy change. Use checked arithmetic/conversions and safe access.
- No unwrap/expect, panic, TODO, unimplemented code, or debug prints, including
  tests. Return recoverable errors. Document private/public items and invariants.
- No `allow` attributes. Necessary local `expect` attributes must name one lint,
  carry a concrete reason, and stay narrow. Stale expectations fail.
- Do not ignore tests, add retries, or introduce broad test/coverage exclusions.

## Verification

Pre-commit is whitespace and formatting only. There is no pre-push hook.
Run focused checks for the change. `bash scripts/check.sh` runs the routine
core/CLI CPU tests. **Pancake required** runs the headless lint and test gates
on pull requests and main pushes. GUI, physical GPU, native platform,
instrumented, and deep assurance checks have explicit entry points in the
README. Select the appropriate ones for the change and report those left unrun.
For hook and documentation changes, use focused script tests and static checks.

The executable P0 library is subject to all assurance gates. Only the exact
documentation-only, single-target scaffold is exempt from coverage reporting.
Executable code must have zero uncovered lines,
functions, regions, and branches. Surviving mutations fail; equivalent mutations
need evidence and a narrow reviewed exception. Coverage does not prove behavior.

When APIs exist, add property tests for numerical contracts, malformed-input
fuzzing, CPU/GPU parity cases, and deterministic performance benchmarks. Preserve
precision and image quality when optimizing. Do not add empty harnesses now.
