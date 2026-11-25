# Repository instructions

Pancake Stacker is the primary product in this repository. It is a desktop app
and CLI for focus stacking and exposure fusion. The [root README](README.md)
serves photographers and starts with downloads and a first stack. Put user
guides in [Pancake documentation](pancake/docs/) and maintainer instructions in
the [developer guide](pancake/DEVELOPING.md).

## Find your area

| Path | What lives there |
| --- | --- |
| [`pancake/`](pancake/) | Rust workspace. `src/` contains image I/O, registration, fusion, and the shared job engine. `crates/pancake-gui/` contains the Slint desktop app, `crates/pancake-cli/` contains the CLI, and `lightroom/` contains the macOS Lightroom Classic plug-in. |
| [`bokehforge/`](bokehforge/) | Python toolkit for dataset downloads, lazy frame loading, benchmarking, and local stack review. Start with its [README](bokehforge/README.md) and keep its user documentation there. |
| [`research/`](research/) | Dataset and algorithm research, benchmark methods, and experiments. The comparison stackers in `.submodules/` support research and some optional review runners. Ordinary Pancake builds do not need them. |
| [`assets/`](assets/) | Pancake branding and small synthetic quickstart frames. |

## Real-time data and events

- Do not use polling or polling-like implementations for real-time data transfer,
  progress, or events. This includes interval requests, timer-driven refreshes,
  repeated status or file checks, and long-polling fallbacks.
- Publish updates when state changes and use an event-driven or push-based
  mechanism such as callbacks, notifications, Server-Sent Events, or WebSockets.
  On reconnect, send the current state over the event channel.

## Bootstrap and verify

- For Pancake, read [`pancake/AGENTS.md`](pancake/AGENTS.md) before changing
  anything in that workspace. Run Cargo and verification commands from
  `pancake/` so the pinned toolchain and workspace configuration apply.
  `bash scripts/bootstrap.sh` installs the pinned Rust tools.
  `bash scripts/check.sh lint` and `bash scripts/check.sh test` match the routine
  pull request CI. Use the developer guide to select GUI, native GPU, and
  release checks. For Lightroom bridge changes run
  `luajit lightroom/test_open_stack.lua` or the same script with Lua 5.1.
- For BokehForge, run `uv sync --frozen` and `uv run --frozen pytest` from
  `bokehforge/` using the pinned Python 3.12 environment.
- [Pancake packages](pancake/docs/releases.md) documents the manual build and
  publish workflow for Apple Silicon, Windows x64, and Ubuntu x86-64. Keep
  download instructions in sync with the package workflow. The
  [LLVM notices](pancake/notices/llvm/) are release inputs.
- [Engineering records](pancake/docs/archive/) contain historical test evidence
  and plans. Check the current source and CI when a historical recipe differs.
  Put generated experiment outputs under the ignored `artifacts/` directory
  and downloaded datasets under `datasets/`.

## Zerene Stacker experiments

Use the [Zerene runner](research/experiments/zerene/run_stack.py) for unattended PMax and DMap experiments on the macOS host. Run from the repository root in a logged-in desktop session with a valid Zerene license or trial. Close the interactive Zerene app before starting a batch.

```sh
caffeinate -i python3 research/experiments/zerene/run_stack.py "/path/to/frames" \
  --output artifacts/zerene/experiment-01
```

Pass a TIFF/JPEG frame directory or an explicit list of frame files. Use a fresh output directory for each run and run batches serially. The runner saves its batch XML and log alongside the 16-bit TIFFs. Keep generated images and logs out of Git.

See the [Zerene experiment guide](research/experiments/zerene/README.md) for setup, parameter sweeps, and failure handling. Keep scripts for research experiments under `research/experiments/`.
