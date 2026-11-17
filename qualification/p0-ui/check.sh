#!/usr/bin/env bash
# Strict qualification checks; native functional oracle is a separate real-GPU run.
set -euo pipefail
probe=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
root=$(cd -- "$probe/../.." && pwd)
artifacts="$root/artifacts/p0-ui-checks"
mkdir -p "$artifacts"
export PATH="${PANCAKE_TOOL_BIN:-$root/pancake/target/tools/bin}:$PATH"
export RUSTUP_TOOLCHAIN=1.100.0
cd "$probe"
python3 - "$root/pancake/Cargo.toml" Cargo.toml <<'PY'
from pathlib import Path
import sys
import tomllib
production, qualification = [tomllib.loads(Path(path).read_text()) for path in sys.argv[1:]]
assert production['workspace']['lints'] == qualification['workspace']['lints'], 'Qualification lint tables drifted'
assert production['profile'] == qualification['profile'], 'Qualification profiles drifted'
PY
cargo fmt --all -- --check
cargo metadata --locked --all-features --format-version 1 > "$artifacts/metadata.json"
python3 review.py "$root/pancake/dependency-policy.json" dependency-review.json "$artifacts/ledger.json"
python3 "$root/pancake/scripts/dependency_policy.py" \
    --config "$root/pancake/deny.toml" --ledger "$artifacts/ledger.json" \
    --metadata "$artifacts/metadata.json" --effective-config "$artifacts/deny.toml" \
    --license-inventory "$artifacts/licenses.json" \
    --clippy-config "$root/pancake/clippy.toml" --effective-clippy-dir "$artifacts/clippy"
CLIPPY_CONF_DIR="$artifacts/clippy" cargo clippy --locked --all-targets --all-features -- -D warnings
cargo doc --locked --no-deps --document-private-items
cargo test --locked -- --include-ignored
test "$(cargo deny --version)" = "cargo-deny 0.20.2"
cargo deny --locked --all-features --config "$artifacts/deny.toml" \
    check advisories bans sources --deny warnings
python3 "$root/pancake/scripts/check_unused_dependencies.py" "$artifacts/metadata.json"
