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

Run the two data commands in order. Downloads fetch the full pinned inventory for each selected source. Processing writes full-resolution float32 linear-sRGB TIFFs with provenance and separate references.

```sh
uv run --frozen bokehforge data --help
uv run --frozen bokehforge data download --dataset D04 --dataset D20
uv run --frozen bokehforge data process --dataset D04 --dataset D20 --workers 2
uv run --frozen pytest
```

## Layout

```text
src/bokehforge/
  cli.py                 # bokehforge data download / process
  data/download.py       # acquisition, verification and resume
  data/process.py        # extraction and canonical conversion
tests/
  test_cli.py
  data/
config/data/
  sources.json           # editable source policy and storage defaults
  sources.lock.json      # pinned source identities and byte hashes
../datasets/             # generated data at repository root; may be rebuilt
  raw/                   # original provider files
  standardized/          # canonical TIFFs and manifests
  .bokehforge/           # logs, reports and conversion state
../research/             # literature reviews and benchmark design
```

Raw and processed paths remain at repository-root `datasets/raw/` and `datasets/standardized/`. Commands automatically save logs and reports beneath repository-root `datasets/.bokehforge/`; no shell redirection is needed.

Source policy and exact source pins live in [config/data/](config/data/), outside the disposable data directory. Configure storage there, or pass `--root`, `--config`, or `--lock`. Removing `datasets/` does not remove source policy or pins; its generated layout is recreated on the next data command.

See [dataset contracts and reproducibility](docs/data.md) and [research findings](../research/README.md).

An all-source command can exit with code 2 when catalogue sources are inaccessible or incomplete. Completed selected sources remain usable; reports identify each status. Run processing separately even when another source is inaccessible.
