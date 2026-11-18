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

# Classify every integration target before selecting the memory-checking boundary.
# Shared launch/readback code is in the library and must retain full CPU coverage.
assurance_profile() {
    local family=$1 target
    mkdir -p target/assurance
    cargo metadata --locked --no-deps --format-version 1 > target/assurance/metadata.json
    python3 scripts/assurance_targets.py target/assurance/metadata.json "$family" \
        > target/assurance/selected-targets.txt
    assurance_args=()
    if [[ $family == cpu ]]; then assurance_args=(--lib --bins); fi
    while IFS= read -r target; do
        assurance_args+=(--test "$target")
    done < target/assurance/selected-targets.txt
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
        python3 -m unittest scripts/test_dependency_policy.py scripts/test_guardrail_manifest.py \
            scripts/test_assurance_targets.py scripts/test_assurance_coverage.py scripts/test_miri_host_policy.py scripts/test_pre_commit.py \
            scripts/test_headless_dependencies.py
        python3 scripts/miri_host_policy.py "$root" validate
        dependency_policy
        cargo fmt --all -- --check
        CLIPPY_CONF_DIR="$root/target/dependency-policy/clippy" \
            cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
        cargo doc --locked --workspace --all-features --no-deps --document-private-items
        shellcheck --source-path=SCRIPTDIR --external-sources \
            scripts/bootstrap.sh scripts/check.sh scripts/test-guardrails.sh \
            scripts/install-actionlint.sh scripts/ci-linux-native-deps.sh .githooks/*
        actionlint "$root/../.github/workflows/pancake.yml"
        ;;
    test | test-cpu | gpu)
        require_tool cargo-nextest
        require_tool cargo-hack
        if [[ $1 == test ]]; then
            assurance_args=(--all-targets)
        elif [[ $1 == gpu ]]; then
            assurance_profile gpu
        else
            assurance_profile cpu
        fi
        # Docs-only crates have no tests. Coverage rejects untested executable code.
        for profile in dev release; do
            cargo nextest run --locked --workspace "${assurance_args[@]}" --all-features \
                --cargo-profile "$profile" --profile ci --no-tests=pass --run-ignored all
        done
        if [[ $1 != gpu ]]; then
            cargo test --locked --workspace --all-features --doc -- --include-ignored
            cargo hack test --locked --workspace --feature-powerset \
                "${assurance_args[@]}" -- --include-ignored
        fi
        ;;
    coverage | coverage-cpu)
        require_tool cargo-llvm-cov
        if empty_scaffold; then
            printf 'Coverage: not applicable to the exact documentation-only scaffold.\n'
            exit 0
        fi
        mkdir -p target/assurance
        assurance_profile cpu
        rm -f target/assurance/coverage-cpu.json
        coverage_result=0
        cargo llvm-cov --locked --workspace --all-features "${assurance_args[@]}" \
            --include-build-script \
            --json --output-path target/assurance/coverage-cpu.json \
            --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0 \
            -- --include-ignored || coverage_result=$?
        if [[ -f target/assurance/coverage-cpu.json ]]; then
            python3 scripts/compare_assurance_coverage.py "$root" \
                target/assurance/coverage-cpu.json target/assurance/coverage-cpu.json
        fi
        if ((coverage_result != 0)); then exit "$coverage_result"; fi
        if [[ $1 == coverage ]]; then
            cargo llvm-cov --locked --workspace --all-features --include-build-script \
                --json --output-path target/assurance/coverage-native.json \
                --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0 \
                -- --include-ignored
            python3 scripts/compare_assurance_coverage.py "$root" \
                target/assurance/coverage-cpu.json target/assurance/coverage-native.json
        fi
        ;;
    branches | branches-cpu)
        require_tool cargo-llvm-cov
        if empty_scaffold; then
            printf 'Branch coverage: not applicable to the documentation-only scaffold.\n'
            exit 0
        fi
        mkdir -p target/coverage
        assurance_profile cpu
        cargo +"$PANCAKE_NIGHTLY" llvm-cov --locked --workspace --all-features "${assurance_args[@]}" \
            --include-build-script --branch --no-report -- --include-ignored
        # cargo-llvm-cov forbids --lib with --doctests. Accumulate the separate
        # documentation-only run without starting native GPU integration tests.
        rm -f target/coverage/branches-cpu.json
        coverage_result=0
        cargo +"$PANCAKE_NIGHTLY" llvm-cov --locked --workspace --all-features --doc \
            --include-build-script --branch --no-clean --json \
            --output-path target/coverage/branches-cpu.json \
            --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0 \
            -- --include-ignored || coverage_result=$?
        if [[ -f target/coverage/branches-cpu.json ]]; then
            python3 scripts/compare_assurance_coverage.py "$root" \
                target/coverage/branches-cpu.json target/coverage/branches-cpu.json
        fi
        if ((coverage_result != 0)); then exit "$coverage_result"; fi
        if ! jq -e '.data | length > 0 and all(.[];
            .totals | all(.lines, .functions, .regions, .branches; .count == .covered)
        )' target/coverage/branches-cpu.json; then
            printf 'Uncovered branches or other coverage deficits remain.\n' >&2
            exit 1
        fi
        if [[ $1 == branches ]]; then
            cargo +"$PANCAKE_NIGHTLY" llvm-cov --locked --workspace --all-features \
                --include-build-script --branch --doctests --json \
                --output-path target/coverage/branches.json \
                --fail-uncovered-lines 0 --fail-uncovered-functions 0 --fail-uncovered-regions 0 \
                -- --include-ignored
            python3 scripts/compare_assurance_coverage.py "$root" \
                target/coverage/branches-cpu.json target/coverage/branches.json
        fi
        ;;
    deps)
        require_tool cargo-deny
        require_tool cargo-machete
        python3 -m unittest scripts/test_dependency_policy.py scripts/test_unused_dependencies.py
        python3 -m unittest discover -s vendor/tests -v
        dependency_policy
        # Omitting [licenses] alone would retain cargo-deny's default license gate.
        # User chose an informational inventory instead of license enforcement.
        cargo deny --locked --workspace --all-features \
            --config "$root/target/dependency-policy/deny-effective.toml" \
            check advisories bans sources --deny warnings
        python3 scripts/check_unused_dependencies.py target/dependency-policy/metadata.json
        ;;
    mutants | mutants-cpu)
        require_tool cargo-mutants
        if [[ $1 == mutants-cpu ]]; then
            assurance_profile cpu
            cargo mutants --cap-lints true --all-features --in-place --timeout 60 \
                --build-timeout 180 -- --locked "${assurance_args[@]}" -- --include-ignored
        else
            cargo mutants --cap-lints true --all-features --in-place --timeout 60 \
                --build-timeout 180 -- --locked -- --include-ignored
        fi
        ;;
    miri)
        python3 scripts/miri_host_policy.py "$root" miri
        # Miri checks pure host contracts. Native integration tests below exercise
        # LLVM JIT and GPU FFI, which Miri cannot interpret (mach_host_self on macOS).
        MIRIFLAGS="-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check" \
            cargo +"$PANCAKE_NIGHTLY" miri test --locked --workspace --all-features --lib -- --include-ignored
        ;;
    nightly | nightly-cpu)
        bash scripts/check.sh miri
        if [[ $1 == nightly-cpu ]]; then
            bash scripts/check.sh layouts-cpu
        else
            bash scripts/check.sh layouts
        fi
        ;;
    layouts | layouts-cpu)
        for seed in 1 42; do
            # Bash 3.2 treats an empty array expansion as unset under nounset.
            # Native tests retain Cargo's default unit/integration/doctest scope.
            if [[ $1 == layouts-cpu ]]; then
                assurance_profile cpu
                RUSTFLAGS="-Dwarnings -Zrandomize-layout -Zlayout-seed=$seed" \
                    cargo +"$PANCAKE_NIGHTLY" test --locked --workspace --all-features \
                        "${assurance_args[@]}" -- --include-ignored
            else
                RUSTFLAGS="-Dwarnings -Zrandomize-layout -Zlayout-seed=$seed" \
                    cargo +"$PANCAKE_NIGHTLY" test --locked --workspace --all-features -- --include-ignored
            fi
        done
        ;;
    sanitizer | sanitizer-gpu)
        kind=${2:?Usage: check.sh sanitizer address|thread (Linux x86_64 only)}
        case "$kind" in
            address | thread) ;;
            *) printf 'Unknown sanitizer: %s\n' "$kind" >&2; exit 2 ;;
        esac
        if [[ $1 == sanitizer-gpu ]]; then
            assurance_profile gpu
        else
            assurance_profile cpu
        fi
        RUSTFLAGS="-Dwarnings -Zsanitizer=$kind" \
            cargo +"$PANCAKE_NIGHTLY" test -Zbuild-std --locked \
                --target x86_64-unknown-linux-gnu --workspace --all-features "${assurance_args[@]}" \
                -- --include-ignored
        ;;
    guardrails)
        require_tool cargo-llvm-cov
        require_tool cargo-mutants
        bash scripts/test-guardrails.sh deep
        ;;
    all)
        mkdir -p target/assurance
        python3 scripts/miri_host_policy.py "$root" all-plan > target/assurance/local-gates.txt
        while IFS= read -r gate <&3; do
            printf '\nPancake gate: %s\n' "$gate"
            bash scripts/check.sh "$gate" 3<&-
        done 3< target/assurance/local-gates.txt
        ;;
    *)
        printf 'Usage: check.sh [all|fast|test|test-cpu|gpu|coverage|coverage-cpu|branches|branches-cpu|deps|mutants|mutants-cpu|miri|nightly|nightly-cpu|layouts|layouts-cpu|sanitizer|sanitizer-gpu|guardrails]\n' >&2
        exit 2
        ;;
esac
