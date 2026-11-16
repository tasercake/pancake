#!/usr/bin/env bash
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
# shellcheck source=tool-versions.sh
source "$root/scripts/tool-versions.sh"
export PATH="$root/target/tools/bin:$PATH"
rustup show active-toolchain
rustup toolchain install "$PANCAKE_NIGHTLY" --profile minimal \
    --component miri --component rust-src --component llvm-tools-preview
for entry in "${PANCAKE_RUST_TOOLS[@]}"; do
    tool=${entry%@*}
    version=${entry#*@}
    if ! cargo "${tool#cargo-}" --version 2>/dev/null | awk -v expected="$version" '
        { for (field = 1; field <= NF; field++) if ($field == expected) found = 1 }
        END { exit !found }
    '; then
        cargo install "$tool" --version "=$version" --locked --root "$root/target/tools"
    fi
done
printf 'Also install actionlint %s and ShellCheck %s for infrastructure linting.\n' \
    "$PANCAKE_ACTIONLINT_VERSION" "$PANCAKE_SHELLCHECK_VERSION"
