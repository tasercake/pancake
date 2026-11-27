#!/usr/bin/env bash
# Provision the native desktop development dependencies on an ephemeral Linux runner.
set -euo pipefail
if [[ ${GITHUB_ACTIONS:-} != true || ${RUNNER_OS:-} != Linux || $(uname -s) != Linux ]]; then
    printf 'Native dependency provisioning requires a Linux GitHub Actions runner.\n' >&2
    exit 1
fi
# Slint 1.19's docs/building.md Linux prerequisites. No Qt/FFmpeg/GStreamer
# examples are enabled. Fontconfig supplies FreeType. GTK 3 provides the file panels.
sudo apt-get update
sudo apt-get install --yes --no-install-recommends \
    pkg-config libfontconfig1-dev libxcb-shape0-dev libxcb-xfixes0-dev \
    libxkbcommon-dev libssl-dev libgtk-3-dev "$@"
pkg-config --exists fontconfig xcb-shape xcb-xfixes xkbcommon openssl 'gtk+-3.0 >= 3.24' \
    'atk >= 2.34' 'gdk-x11-3.0 >= 3.22' x11
