# Focus-stacking research

Research and tools for building a high-quality macro focus-stacking benchmark.

- [Bokehforge](bokehforge/README.md): `uv` project with dataset download and processing CLI, source configuration, tests, and documentation.
- [Pancake](pancake/README.md): future focus-stacking application rewrite in `pancake/`.
- [Research](research/README.md): artifact reviews, algorithms, measurement design, and dataset evidence.
- [Unattended Zerene Stacker](research/experiments/zerene/README.md): run PMax and DMap reference stacks with the installed macOS app.
- `submodules/`: upstream focus-stacking implementations.
- `datasets/`: generated native and standardized data; logs and runtime files stay in `datasets/.bokehforge/`.

Run Bokehforge from the repository root:

```sh
uv run --project bokehforge --frozen bokehforge data download
uv run --project bokehforge --frozen bokehforge data process
uv run --project bokehforge --frozen pytest bokehforge/tests
```

Source policy and exact source pins live in [bokehforge/config/data/](bokehforge/config/data/), outside the disposable data directory. Pass `--dataset D29` to download only InfinigenDefocus.
