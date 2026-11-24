#!/usr/bin/env bash
# Replace Tracel LLVM's dynamic zstd link request with the release archive.
set -euo pipefail
archive=${PANCAKE_ZSTD_ARCHIVE:?Set PANCAKE_ZSTD_ARCHIVE to the pinned libzstd.a}
if [[ ! -f "$archive" ]]; then
    printf 'Cannot find the static Zstandard archive: %s\n' "$archive" >&2
    exit 1
fi

link_args=()
for arg in "$@"; do
    if [[ "$arg" == -lzstd ]]; then
        link_args+=("$archive")
    else
        link_args+=("$arg")
    fi
done
exec xcrun clang "${link_args[@]}"
