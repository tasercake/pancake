#!/usr/bin/env bash
# Exercise real tools against disposable, deliberately broken crates.
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
export PATH="$root/target/tools/bin:$PATH"
mkdir -p "$root/target"
fixture=$(mktemp -d "$root/target/guardrails.XXXXXX")
trap 'rm -rf -- "${fixture:?}"' EXIT
for file in README.md rust-toolchain.toml clippy.toml; do
    cp "$root/$file" "$fixture/$file"
done
python3 "$root/scripts/guardrail_manifest.py" "$root/Cargo.toml" "$fixture/Cargo.toml"
mkdir -p "$fixture/src"
printf '//! Guardrail fixture bootstrap.\n' >"$fixture/src/lib.rs"
mkdir -p "$fixture/scripts"
cp "$root/scripts/check.sh" "$root/scripts/tool-versions.sh" "$fixture/scripts/"
cp "$root/scripts/assurance_targets.py" "$root/scripts/compare_assurance_coverage.py" "$fixture/scripts/"
cd "$fixture"
export CARGO_TARGET_DIR="$root/target/guardrail-build"
cargo generate-lockfile --offline --manifest-path "$fixture/Cargo.toml"

reject() {
    local diagnostic=$1
    shift
    if "$@" >"$fixture/check.log" 2>&1; then
        printf 'FAIL: guardrail accepted %s\n' "$diagnostic" >&2
        cat "$fixture/check.log" >&2
        exit 1
    fi
    if ! tr '_' '-' <"$fixture/check.log" | grep -F -- "${diagnostic//_/-}" >/dev/null; then
        printf 'FAIL: wrong rejection for %s\n' "$diagnostic" >&2
        cat "$fixture/check.log" >&2
        exit 1
    fi
    printf 'PASS: rejected %s\n' "$diagnostic"
}

lint() {
    cargo clippy --offline --locked --all-targets --all-features -- -D warnings
}

cat >src/lib.rs <<'RS'
//! Negative fixture.
/// Deliberately unsafe declaration.
///
/// # Safety
/// This fixture must never be accepted.
pub const unsafe fn rejected() {}
RS
reject unsafe_code lint

cat >src/lib.rs <<'RS'
//! Negative fixture.
/// Deliberately unchecked optional value.
#[must_use]
pub const fn rejected(value: Option<u8>) -> u8 {
    value.unwrap()
}
RS
reject clippy::unwrap_used lint

cat >src/lib.rs <<'RS'
//! Negative fixture.
/// Deliberately unfinished implementation.
#[must_use]
pub const fn rejected() -> u8 {
    todo!()
}
RS
reject clippy::todo lint

cat >src/lib.rs <<'RS'
//! Negative fixture.
pub struct Undocumented;
RS
reject missing_docs lint

cat >src/lib.rs <<'RS'
//! Negative fixture.
#[cfg(pancake_unregistered)]
const HIDDEN: u8 = 0;
RS
reject unexpected_cfgs lint

cat >src/lib.rs <<'RS'
//! Negative fixture.
/// Deliberately obsolete suppression.
#[expect(clippy::unwrap_used, reason = "This expectation must be rejected.")]
#[must_use]
pub const fn rejected(value: u8) -> u8 {
    value
}
RS
reject unfulfilled_lint_expectations lint

if [[ ${1:-lint} == deep ]]; then
    cat >src/lib.rs <<'RS'
//! Negative fixture.
/// Deliberately undertested implementation.
#[must_use]
pub const fn untested(value: i32) -> i32 {
    if value > 0 {
        value
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_positive_is_covered() {
        assert_eq!(super::untested(1), 1, "positive sample must pass");
    }
}
RS
    reject 'uncovered CPU lines' bash scripts/check.sh coverage

    cat >src/lib.rs <<'RS'
//! Negative fixture.
/// Checks whether an integer is positive.
#[must_use]
pub const fn positive(value: i32) -> bool {
    value > 0
}

#[cfg(test)]
mod tests {
    #[test]
    fn insufficient_test() {
        assert!(super::positive(4), "positive sample must pass");
    }
}
RS
    reject missed bash scripts/check.sh mutants

    cat >src/lib.rs <<'RS'
//! Negative fixture for a branch that line coverage cannot distinguish.
/// Checks whether an integer is nonnegative.
#[must_use]
pub const fn nonnegative(value: i32) -> bool {
    if value > 0 || value == 0 {
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn misses_zero_condition() {
        assert!(super::nonnegative(1), "positive value must pass");
        assert!(!super::nonnegative(-1), "negative value must fail");
    }
}
RS
    reject 'uncovered CPU branches' bash scripts/check.sh branches

    cat >src/lib.rs <<'RS'
//! Positive control: a small, fully tested contract.
/// Checks whether an integer is positive.
#[must_use]
pub const fn positive(value: i32) -> bool {
    value > 0
}

#[cfg(test)]
mod tests {
    #[test]
    fn covers_boundary_and_both_signs() {
        assert!(super::positive(1), "positive value must pass");
        assert!(!super::positive(0), "zero must fail");
        assert!(!super::positive(-1), "negative value must fail");
    }
}
RS
    if ! bash scripts/check.sh coverage >"$fixture/positive.log" 2>&1 || \
        ! bash scripts/check.sh mutants \
        >>"$fixture/positive.log" 2>&1; then
        cat "$fixture/positive.log" >&2
        exit 1
    fi
    printf 'PASS: accepted fully covered code with mutation-resistant tests\n'
fi
