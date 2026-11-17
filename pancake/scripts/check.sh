#!/usr/bin/env bash
# Shared local/CI entry points. Missing tools fail closed.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
export PATH="$root/target/tools/bin:$PATH"
# shellcheck source=tool-versions.sh
source "$root/scripts/tool-versions.sh"
export RUST_BACKTRACE=1

require_tool() {
    local tool=$1 expected=${2:-} entry output
    if [[ -z $expected ]]; then
        for entry in "${PANCAKE_RUST_TOOLS[@]}"; do
            if [[ ${entry%@*} == "$tool" ]]; then expected=${entry#*@}; break; fi
        done
    fi
    if [[ $tool == cargo-* ]]; then
        output=$(cargo "${tool#cargo-}" --version)
    else
        output=$("$tool" --version)
    fi
    if [[ -z $expected ]] || ! awk -v expected="$expected" '
        { for (field = 1; field <= NF; field++) if ($field == expected) found = 1 }
        END { exit !found }
    ' <<<"$output"; then
        printf 'Required %s %s; found: %s\n' "$tool" "$expected" "$output" >&2
        exit 1
    fi
}

# Cargo-deny cannot expire reviews or bind advisory ignores to exact versions.
# The same checks constrain Clippy's name-only duplicate allowances.
dependency_policy() {
    mkdir -p target/dependency-policy
    cargo metadata --locked --format-version 1 --all-features \
        > target/dependency-policy/metadata.json
    python3 scripts/dependency_policy.py \
        --config deny.toml --ledger dependency-policy.json \
        --metadata target/dependency-policy/metadata.json \
        --effective-config target/dependency-policy/deny-effective.toml \
        --license-inventory target/dependency-policy/licenses.json \
        --clippy-config clippy.toml \
        --effective-clippy-dir target/dependency-policy/clippy
}

# Only this exact, single-target documentation-only scaffold has no coverage data.
empty_scaffold() {
    [[ $(find src -type f -print) == src/lib.rs ]] || return 1
    cargo metadata --locked --no-deps --format-version 1 | jq -e '
        (.workspace_members | length) == 1 and
        (.packages[0].targets | length) == 1 and
        .packages[0].targets[0].kind == ["lib"] and
        (.packages[0].targets[0].src_path | gsub("\\\\"; "/") | endswith("/src/lib.rs"))
    ' >/dev/null || return 1
    cmp -s src/lib.rs <(cat <<'RS'
//! Pancake focus-stacking application.
//!
//! Implementation has not started. This crate intentionally contains no
//! executable code, public API, dependencies, or placeholder tests.
RS
    )
}

case ${1:-all} in
    fast)
        require_tool actionlint "$PANCAKE_ACTIONLINT_VERSION"
        require_tool shellcheck "$PANCAKE_SHELLCHECK_VERSION"
        python3 -m unittest scripts/test_dependency_policy.py scripts/test_guardrail_manifest.py
        dependency_policy
        cargo fmt --all -- --check
        CLIPPY_CONF_DIR="$root/target/dependency-policy/clippy" \
            cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
        cargo doc --locked --workspace --all-features --no-deps --document-private-items
        shellcheck --source-path=SCRIPTDIR --external-sources \
            scripts/bootstrap.sh scripts/check.sh scripts/test-guardrails.sh .githooks/*
        actionlint "$root/../.github/workflows/pancake.yml"
        ;;
    test)
        require_tool cargo-nextest
        require_tool cargo-hack
        # Docs-only crates have no tests. Coverage rejects untested executable code.
        for profile in dev release; do
            cargo nextest run --locked --workspace --all-targets --all-features \
                --cargo-profile "$profile" --profile ci --no-tests=pass --run-ignored all
        done
        cargo test --locked --workspace --all-features --doc -- --include-ignored
        cargo hack test --locked --workspace --feature-powerset -- --include-ignored
        ;;
    coverage)
        require_tool cargo-llvm-cov
        if empty_scaffold; then
            printf 'Coverage: not applicable to the exact documentation-only scaffold.\n'
            exit 0
        fi
        cargo llvm-cov --locked --workspace --all-features \
            --include-build-script \
            --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0 \
            -- --include-ignored
        ;;
    branches)
        require_tool cargo-llvm-cov
        if empty_scaffold; then
            printf 'Branch coverage: not applicable to the documentation-only scaffold.\n'
            exit 0
        fi
        mkdir -p target/coverage
        cargo +"$PANCAKE_NIGHTLY" llvm-cov --locked --workspace --all-features \
            --include-build-script --branch --doctests --json \
            --output-path target/coverage/branches.json \
            --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0 \
            -- --include-ignored
        if ! jq -e '.data | length > 0 and all(.[];
            .totals | all(.lines, .functions, .regions, .branches; .count == .covered)
        )' target/coverage/branches.json; then
            printf 'Uncovered branches or other coverage deficits remain.\n' >&2
            exit 1
        fi
        ;;
    deps)
        require_tool cargo-deny
        require_tool cargo-machete
        python3 -m unittest scripts/test_dependency_policy.py scripts/test_unused_dependencies.py
        dependency_policy
        # Omitting [licenses] alone would retain cargo-deny's default license gate.
        # User chose an informational inventory instead of license enforcement.
        cargo deny --locked --workspace --all-features \
            --config "$root/target/dependency-policy/deny-effective.toml" \
            check advisories bans sources --deny warnings
        python3 scripts/check_unused_dependencies.py target/dependency-policy/metadata.json
        ;;
    mutants)
        require_tool cargo-mutants
        cargo mutants --cap-lints true --all-features --in-place --timeout 60 \
            --build-timeout 180 -- --locked -- --include-ignored
        ;;
    nightly)
        MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check" \
            cargo +"$PANCAKE_NIGHTLY" miri test --locked --workspace --all-features -- --include-ignored
        for seed in 1 42; do
            RUSTFLAGS="-Dwarnings -Zrandomize-layout -Zlayout-seed=$seed" \
                cargo +"$PANCAKE_NIGHTLY" test --locked --workspace --all-features -- --include-ignored
        done
        ;;
    sanitizer)
        kind=${2:?Usage: check.sh sanitizer address|thread (Linux x86_64 only)}
        case "$kind" in
            address | thread) ;;
            *) printf 'Unknown sanitizer: %s\n' "$kind" >&2; exit 2 ;;
        esac
        RUSTFLAGS="-Dwarnings -Zsanitizer=$kind" \
            cargo +"$PANCAKE_NIGHTLY" test -Zbuild-std --locked \
                --target x86_64-unknown-linux-gnu --workspace --all-features --lib --tests \
                -- --include-ignored
        ;;
    guardrails)
        require_tool cargo-llvm-cov
        require_tool cargo-mutants
        bash scripts/test-guardrails.sh deep
        ;;
    all)
        for gate in fast test coverage deps mutants nightly branches guardrails; do
            printf '\nPancake gate: %s\n' "$gate"
            bash scripts/check.sh "$gate"
        done
        ;;
    *)
        printf 'Usage: check.sh [all|fast|test|coverage|branches|deps|mutants|nightly|sanitizer|guardrails]\n' >&2
        exit 2
        ;;
esac
