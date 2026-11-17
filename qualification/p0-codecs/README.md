# Isolated codec qualification

This test-only workspace qualifies concrete dependency capabilities. It is
outside `pancake/`; it does not add an application dependency, a production
decoder. Its Rust and Clippy lint tables exactly match
the application scaffold. The Cargo lockfile pins the full probe graph.

The two Rust tests cover a full real CR3 sensor decode, an actual generated
Bayer DNG, true RGB16 PNG/ZIP TIFF, float32 TIFF/EXR, ICC passthrough, an actual
ICC conversion, and basic malformed image/profile rejection. Float fixtures
include negative/HDR values and `0x3f000001`, which cannot survive an f16
intermediate. Sensor decode uses `RawSource::new_from_shared_vec` and owned
bytes, avoiding file-backed mapping and borrowed input lifetimes. No first-party
unsafe Rust is used.

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
RAYON_NUM_THREADS=2 CARGO_BUILD_JOBS=2 cargo +1.100.0 test --locked --manifest-path qualification/p0-codecs/Cargo.toml
artifacts/p0-codec-oracle/.venv/bin/python qualification/p0-codecs/oracle.py verify qualification/p0-codecs/target/qualification
cargo +1.100.0 fmt --manifest-path qualification/p0-codecs/Cargo.toml -- --check
CARGO_BUILD_JOBS=2 cargo +1.100.0 clippy --locked --manifest-path qualification/p0-codecs/Cargo.toml --all-targets --all-features -- -D warnings
```

`prepare` only reads the photograph. It writes generated fixtures and results
under the probe's ignored `target/qualification/` directory. The real input's
expected SHA-256 is
`4538bc75741cc16292b1838c52989050d084c4a51f0e005a554819190c379fca`.
Run `prepare` before tests; missing corpus/fixtures fail rather than skip.

Important remaining gap: Rawler's recommended crop matches the CR3 file, while
LibRaw's rendered visible area follows a camera compatibility table and is
larger. This probe does not alter crop metadata or assert rendered RAW parity.
See [`../../pancake/docs/p0-codec-qualification.md`](../../pancake/docs/p0-codec-qualification.md)
for exact dimensions, dependency review and unresolved gate conditions.
