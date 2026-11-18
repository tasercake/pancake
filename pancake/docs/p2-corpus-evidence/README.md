# Pancake P2 corpus preparation

Verified on 2026-11-18 in Singapore on Ubuntu 24.04.5 x86-64, kernel
6.8.0-147-generic, Ryzen 9 7950X3D and RTX 3090 driver 580.178.04. The operations
were downloads and CPU-side generation or decoding. Full stacking jobs and
performance samples remain pending.

## Generated T50

At `9057cff6662b4dccd985169388e4550837ec975b`, all 50 6000×4000 RGB16
ZIP-TIFF inputs and both truth images were present together. All 52 encoded
files and decoded pixel data matched the P0 reference receipt exactly,
including both independent full-image decode paths. Preflight required
3,067,213,930 bytes in addition to the 15 GiB reserve. The retained TIFF set
weighed 2,775,174,405 bytes. The cleanup command rehashed then deleted all 52
TIFFs. Zero generated images remained. [Run summary](t50-linux-verification.json)
[Full materialization receipt](t50-linux-materialization.json.gz)
[Cleanup receipt](t50-linux-cleanup.json)

The original tests failed at `34dd609` because the command lacked the new
materialization option. The receipt collision test failed at `9057cff` after
deleting images before reserving the cleanup receipt. The storage-only
fix at `2383cf1` reserves the receipt before deletion. All 15 focused tests
and Ruff 0.17.5 pass. The full native receipt identifies the first revision.
[Generator and commands](../../../qualification/t50-generated/README.md)

```sh
artifacts/t50-venv/bin/python -m unittest discover -s qualification/t50-generated -v
uvx ruff check qualification/t50-generated
```

## Frozen real inputs

All 209 native files matched the frozen size and SHA-256 in their original
order. All files were fully decoded independently. [Verification receipt](real-validation.json)
[Acquisition receipt](real-acquisition.json)

| Workload | Native inputs | Result |
| --- | --- | --- |
| R50 | 50 Canon CR3, 5118×3412 visible pixels | All 50 full LibRaw developments give RGB16. All visible RAW planes have nonzero low eight bits. |
| D150 | 150 original 2304×1728 JPEG8 frames | Shape, RGB channels and depth verified after full decode. Only the selected dorsal archive from Dmitry Vorontsov's capture was fetched. |
| E9 | Nine original 512×339 PNG8 Balloons exposures | Shape, RGB channels and depth verified after full decode. The source archive identifies the bracket sequence. No standard exposure-time EXIF was present in the selected PNGs. |

- D150 uses 150 native ordered entries from `26E_dorsal_60x.zip`, verified at
  53,459,291 bytes. Figshare v1 metadata records CC BY 4.0.
  [Versioned Figshare record](https://doi.org/10.6084/m9.figshare.14707077.v1)
- E9 uses nine entries in `MEFDatabase.rar`, verified at 69,869,523 bytes.
  The creator's terms permit reuse with the full copyright notice and require
  attribution to the University of Waterloo IVC lab and citations to Ma,
  Zeng and Wang (TIP 2015) and Zeng et al. (QoMEX 2014) in published research.
  The temporary staging directory retains the source notice.
  [Waterloo database and terms](https://ivc.uwaterloo.ca/database/MEF/MEF-Database.html)

Selected payload was 565,427,810 bytes, within the 2,147,483,648-byte download
cap. Extracted selected frames total 496,635,636 bytes. The acquisition fetched
just the two selected archives and 50 RAW files. Original archives and images
stay in ignored task storage and must be removed after use. This run moved
verified data to `artifacts/pancake-corpus/p2-real-inputs`, with an ownership
marker listing the temporary archive, dataset and source metadata directories.
Resolve the adapter manifests' `path` entries relative to a newly acquired
root. After the P10 consumers finished, the owned staging directory was
removed after rechecking all 209 inputs, both archives, and four metadata
files. A future machine can reconstruct it using the commands below.
[Verified cleanup receipt](../../../qualification/p10-performance/evidence/real-corpus-cleanup.json)

From a clean worktree root with a **new** `artifacts/p2-real-inputs` directory,
the preserved acquisition command uses versioned manifest URLs, bounded
streaming, hash-verified writes and exact member extraction (Python 3.12.3
stdlib ZIP and 7-Zip 23.01 for RAR). The decode command uses Python 3.12.12,
RawPy 0.26.1/LibRaw 0.22.0, Pillow 13.0.0 and NumPy 2.5.4. Source notice
material in this run included the pinned R50 LICENSE and README, the Figshare
v1 article metadata and the Waterloo database HTML.

```sh
mkdir -p artifacts
mkdir artifacts/p2-real-inputs
curl --fail --silent --show-error --location \
  https://ivc.uwaterloo.ca/database/MEF/MEF-Database.html \
  --output artifacts/p2-real-inputs/waterloo-source-notice.html
python3 pancake/docs/p2-corpus-evidence/acquire-real.py.txt
uv venv --python 3.12.12 artifacts/p2-corpus-venv
uv pip install --python artifacts/p2-corpus-venv/bin/python \
  numpy==2.5.4 rawpy==0.26.1 Pillow==13.0.0
OMP_NUM_THREADS=2 OPENBLAS_NUM_THREADS=1 artifacts/p2-corpus-venv/bin/python \
  pancake/docs/p2-corpus-evidence/check-real.py.txt
```

The commands compare against the frozen per-file hashes. The receipt also
records a SHA-256 of concatenated binary SHA-256 digests in input order for
each workload's source bytes and decoded pixels. Decode hashes document
the particular decoder versions and configuration used here.
