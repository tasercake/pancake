<h1 align="center">
  <img src="assets/pancake-stacker-wordmark.png" alt="Pancake Stacker" width="260">
</h1>

<p align="center">
  <a href="https://github.com/tasercake/pancake/actions/workflows/pancake.yml"><img src="https://img.shields.io/github/actions/workflow/status/tasercake/pancake/pancake.yml?branch=main&amp;event=push&amp;label=CI" alt="Pancake CI"></a>
  <a href="pancake/LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="Pancake license: GPL-3.0-or-later"></a>
  <a href="https://github.com/tasercake/pancake/releases"><img src="https://img.shields.io/badge/downloads-GitHub_Releases-6f42c1" alt="Downloads on GitHub Releases"></a>
</p>

Pancake Stacker combines photos taken at different focus distances into a single image with more of the subject in focus. Use the desktop app to work with a stack interactively or the CLI to run it from a script.

[Build from source](#build-from-source) · [Desktop and Lightroom guide](pancake/README.md) · [Releases](https://github.com/tasercake/pancake/releases) · [Development](pancake/DEVELOPING.md)

## Build from source

Install [Rust with rustup](https://rustup.rs/). The checkout selects the pinned toolchain automatically. For a desktop build, install the tools for your platform.

| Platform | Desktop build prerequisites |
| --- | --- |
| macOS | Apple Silicon, Xcode with its command-line tools selected, and Zstandard (`brew install zstd`) |
| Ubuntu 24.04 | A C/C++ toolchain, `pkg-config`, and the development packages listed in [the Linux build setup](pancake/scripts/ci-linux-native-deps.sh) |
| Windows | Visual Studio Build Tools with **Desktop development with C++**, the Windows SDK, and the x64 MSVC Rust host toolchain |

Clone the repo and build both the app and CLI.

```sh
git clone https://github.com/tasercake/pancake.git
cd pancake/pancake
cargo build --locked --release --workspace --bins
```

The two executables are in `target/release/` with an `.exe` suffix on Windows. Run `pancake-gui` for the desktop app. On macOS, see the [app guide](pancake/README.md#lightroom-classic-on-macos) for the packaged app and Lightroom Classic workflow.

### Try the CLI

From the same directory, run Pancake on the two bundled [synthetic focus frames](assets/quickstart/). The Pancake project created these frames as a quick check for the command. It writes `stacked.tif` in the current directory.

```sh
cargo run --locked --release -p pancake-cli -- --cpu --input ../assets/quickstart/focus-near.png ../assets/quickstart/focus-far.png --output stacked.tif
```

## What's in this repository

- [Pancake](pancake/README.md) is the focus-stacking desktop app, CLI, and Rust core.
- [BokehForge](bokehforge/README.md) is the companion Python toolkit for verified dataset downloads and on-demand focus-stack frame loading.
- [Benchmark research](research/README.md) covers stack quality, evaluation methods, and dataset selection.
