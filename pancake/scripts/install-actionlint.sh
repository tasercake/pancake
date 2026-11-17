#!/usr/bin/env bash
# Install the exact reviewed upstream release, without a host Go dependency.
set -euo pipefail
root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
# shellcheck source=tool-versions.sh
source "$root/scripts/tool-versions.sh"
destination=${1:?Usage: install-actionlint.sh DESTINATION_DIRECTORY}
system=$(uname -s)
machine=$(uname -m)
case "$PANCAKE_ACTIONLINT_VERSION:$system:$machine" in
    1.7.12:Darwin:x86_64)
        platform=darwin_amd64
        digest=5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644 ;;
    1.7.12:Darwin:arm64)
        platform=darwin_arm64
        digest=aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f ;;
    1.7.12:Linux:x86_64)
        platform=linux_amd64
        digest=8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8 ;;
    1.7.12:Linux:aarch64)
        platform=linux_arm64
        digest=325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6 ;;
    *) printf 'No reviewed actionlint release for %s:%s:%s\n' \
        "$PANCAKE_ACTIONLINT_VERSION" "$system" "$machine" >&2; exit 1 ;;
esac
temporary=$(mktemp -d)
trap 'rm -rf -- "$temporary"' EXIT
asset="actionlint_${PANCAKE_ACTIONLINT_VERSION}_${platform}.tar.gz"
curl --fail --location --silent --show-error \
    "https://github.com/rhysd/actionlint/releases/download/v$PANCAKE_ACTIONLINT_VERSION/$asset" \
    --output "$temporary/release.tar.gz"
python3 - "$temporary/release.tar.gz" "$digest" <<'PY'
import hashlib
from pathlib import Path
import sys
archive, expected = sys.argv[1:]
actual = hashlib.sha256(Path(archive).read_bytes()).hexdigest()
if actual != expected:
    raise SystemExit(f'actionlint archive checksum mismatch: expected {expected}, got {actual}')
PY
tar -xzf "$temporary/release.tar.gz" -C "$temporary" actionlint
version=$("$temporary/actionlint" --version | head -n 1)
if [[ $version != "$PANCAKE_ACTIONLINT_VERSION" ]]; then
    printf 'Required actionlint %s; release reports %s\n' "$PANCAKE_ACTIONLINT_VERSION" "$version" >&2
    exit 1
fi
mkdir -p "$destination"
install -m 0755 "$temporary/actionlint" "$destination/actionlint"
if [[ -n ${GITHUB_PATH:-} ]]; then printf '%s\n' "$destination" >> "$GITHUB_PATH"; fi
"$destination/actionlint" --version
