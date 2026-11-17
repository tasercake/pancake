# Isolated codec qualification

This test-only workspace qualifies concrete dependency capabilities. It is
outside `pancake/`; it does not add an application dependency, a production
decoder. Its Rust and Clippy lint tables exactly match
the application scaffold. The Cargo lockfile pins the full probe graph.

The Rust tests cover a full real CR3 sensor decode, an actual generated
Bayer DNG,
true RGB16 PNG/ZIP TIFF, float32 TIFF/EXR, ICC passthrough, an actual ICC
conversion, and basic malformed image/profile rejection. Float fixtures
include negative/HDR values and `0x3f000001`, which cannot survive an f16
intermediate. Sensor decode uses `RawSource::new_from_shared_vec` and owned
bytes, avoiding file-backed mapping and borrowed input lifetimes. No first-party
unsafe Rust is used.

Every RAW decode/development call uses one shared process-wide local Rayon pool
with exactly two named workers. `install` keeps nested parallel work inside that
pool. No global pool is configured and no `RAYON_NUM_THREADS` setting is needed.

`oracle.py` independently checks LibRaw sensor samples, OpenCV PNG pixels,
tifffile TIFF samples/tags, OpenEXR channel precision, and system Little CMS
conversion. The color oracle currently uses the macOS Homebrew library path
`/opt/homebrew/lib/liblcms2.dylib`; this is qualification tooling, not a packaging
solution. Python dependencies are pinned in `requirements.txt`.

From the repository root, with the already-acquired pinned D02 corpus:

```sh
export PANCAKE_RAW_SAMPLE=/absolute/path/to/datasets/p0-d02/raw/frame_000.CR3
uv venv artifacts/p0-codec-oracle/.venv --python 3.12
uv pip install --python artifacts/p0-codec-oracle/.venv/bin/python -r qualification/p0-codecs/requirements.txt
artifacts/p0-codec-oracle/.venv/bin/python qualification/p0-codecs/oracle.py prepare qualification/p0-codecs/target/qualification --raw "$PANCAKE_RAW_SAMPLE"
CARGO_BUILD_JOBS=2 cargo +1.100.0 test --locked --manifest-path qualification/p0-codecs/Cargo.toml -- --test-threads=1
artifacts/p0-codec-oracle/.venv/bin/python qualification/p0-codecs/oracle.py verify qualification/p0-codecs/target/qualification
cargo +1.100.0 fmt --manifest-path qualification/p0-codecs/Cargo.toml -- --check
CARGO_BUILD_JOBS=2 cargo +1.100.0 clippy --locked --manifest-path qualification/p0-codecs/Cargo.toml --all-targets --all-features -- -D warnings
```

`prepare` only reads the photograph. It writes generated fixtures and results
under the probe's ignored `target/qualification/` directory. The real input's
expected SHA-256 is
`4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca`.
Run `prepare` before tests. Set `CARGO_TARGET_DIR` to an existing codec cache when disk
space is constrained. Generated data remains below this probe's `target/`.

Important remaining gap: Rawler's recommended crop matches the CR3 file, while
LibRaw's rendered visible area follows a camera compatibility table and is
larger. This probe does not alter crop metadata or assert rendered RAW parity.
See [`../../pancake/docs/p0-codec-qualification.md`](../../pancake/docs/p0-codec-qualification.md)
for the original review. The adopted smaller P0 RAW/I/O minimum and initial
RAW policy are now qualified in
[`../../pancake/docs/p0-raw-development-qualification.md`](../../pancake/docs/p0-raw-development-qualification.md).

The combined graph snapshot is metadata only. To reproduce its resolution from
the repository root without adopting application dependencies:

```sh
mkdir -p artifacts/p0-close-io/combined/src
cp qualification/p0-codecs/combined-graph-manifest.toml artifacts/p0-close-io/combined/Cargo.toml
cp qualification/p0-codecs/combined-graph.lock artifacts/p0-close-io/combined/Cargo.lock
printf '%s\n' '//! Metadata-only dependency consumer.' > artifacts/p0-close-io/combined/src/lib.rs
cargo +1.100.0 metadata --locked --manifest-path artifacts/p0-close-io/combined/Cargo.toml --format-version 1 > artifacts/p0-close-io/combined/metadata.json
```

This records compatibility of selected package/resource generations. It is not
a combined build, kernel execution, policy audit, coverage or performance pass.
