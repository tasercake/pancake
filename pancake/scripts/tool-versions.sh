#!/usr/bin/env bash
# Exact tool versions shared by bootstrap and CI.
PANCAKE_NIGHTLY=nightly-2026-11-15
PANCAKE_RUST_TOOLS=(
    cargo-nextest@0.9.152
    cargo-llvm-cov@0.9.1
    cargo-deny@0.20.2
    cargo-mutants@27.1.0
    cargo-hack@0.6.46
    cargo-machete@0.9.2
)
PANCAKE_ACTIONLINT_VERSION=1.7.12
PANCAKE_SHELLCHECK_VERSION=0.11.0
