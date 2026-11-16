# Bokehforge

A measurement-first workbench for building better focus stacks.

## Run

Python 3.12.12 and dependency versions are pinned in this project directory.

From the repository root:

```sh
cd bokehforge
uv sync --frozen
uv run --frozen bokehforge data download
uv run --frozen bokehforge data process
```

Run the two data commands in order. Downloads preserve native files and enforce the default **2 GiB total budget**. Processing writes full-resolution float32 linear-sRGB TIFFs with provenance and separate references. LSFD is disabled.

```sh
uv run --frozen bokehforge data --help
uv run --frozen bokehforge data download --dataset D05 --dataset D08
uv run --frozen bokehforge data process --dataset D05 --dataset D08 --workers 2
uv run --frozen pytest
```

## Layout

```text
src/bokehforge/
  cli.py                 # bokehforge data download / process
  data/download.py       # acquisition, budgets, verification and resume
  data/process.py        # extraction and canonical conversion
tests/
  test_cli.py
  data/
config/data/
  sources.json           # editable source policy, budget and storage defaults
  sources.lock.json      # pinned source identities and byte hashes
../datasets/             # generated data at repository root; may be rebuilt
  raw/                   # original provider files
  standardized/          # canonical TIFFs and manifests
  .bokehforge/           # logs, reports, plan, cache and conversion state
../research/             # literature reviews and benchmark design
```

Raw and processed paths remain at repository-root `datasets/raw/` and `datasets/standardized/`. Commands automatically save logs and reports beneath repository-root `datasets/.bokehforge/`; no shell redirection is needed.

Source policy and exact source pins live in [config/data/](config/data/), outside the disposable data directory. Configure storage and budget there, or pass `--root`, `--config`, `--lock` and download budget options. Removing `datasets/` does not remove source policy or pins; its generated layout is recreated on the next data command.

See [dataset contracts and reproducibility](docs/data.md) and [research findings](../research/README.md).

An all-source command can exit with code 2 when catalogue sources are inaccessible or incomplete. Disabled and budget-excluded sources are intentional skips. Completed selected sources remain usable; reports identify each status. Run processing separately even when another source is inaccessible.
