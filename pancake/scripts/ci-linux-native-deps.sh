#!/usr/bin/env bash
# Provision Slint's native development dependencies on an ephemeral Linux runner.
set -euo pipefail
if [[ ${GITHUB_ACTIONS:-} != true || ${RUNNER_OS:-} != Linux || $(uname -s) != Linux ]]; then
    printf 'Native dependency provisioning requires a Linux GitHub Actions runner.\n' >&2
    exit 1
fi
# Slint 1.19's docs/building.md Linux prerequisites. No Qt/FFmpeg/GStreamer
# examples are enabled. Fontconfig also supplies its FreeType development edge.
sudo apt-get update
sudo apt-get install --yes --no-install-recommends \
    pkg-config libfontconfig1-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev
pkg-config --exists fontconfig xcb-shape xcb-xfixes xkbcommon openssl
