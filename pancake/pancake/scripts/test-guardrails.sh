#!/usr/bin/env bash
# Exercise real tools against disposable, deliberately broken crates.
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
export PATH="$root/target/tools/bin:$PATH"
mkdir -p "$root/target"
fixture=$(mktemp -d "$root/target/guardrails.XXXXXX")
trap 'rm -rf -- "${fixture:?}"' EXIT
for file in Cargo.toml Cargo.lock README.md rust-toolchain.toml clippy.toml; do
    cp "$root/$file" "$fixture/$file"
done
mkdir -p "$fixture/src"
cd "$fixture"
export CARGO_TARGET_DIR="$root/target/guardrail-build"

reject() {
    local diagnostic=$1
    shift
    if "$@" >"$fixture/check.log" 2>&1; then
        printf 'FAIL: guardrail accepted %s\n' "$diagnostic" >&2
        cat "$fixture/check.log" >&2
        exit 1
    fi
    if ! grep -Fq -- "$diagnostic" "$fixture/check.log"; then
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
/// Deliberately untested implementation.
#[must_use]
pub const fn untested(value: u8) -> u8 {
    value
}
RS
    reject uncovered cargo llvm-cov --offline --locked --workspace --all-features \
        --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0

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
    reject missed cargo mutants --in-place --timeout 30 -- --locked
fi
