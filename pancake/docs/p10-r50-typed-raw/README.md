# Owned U16 completion for normal RAW decode

The scoped prototype `c80d59abe0b47a5e831a1e2b4ccad132d96106c7` is
based on exact `e81079d`. The normal three-color Rawler path preserves the
same developed allocation, quantizer and worker policy. Each chunk must
return one unsigned code per sample before writing any normalized values.
The private U16 completion checks the overall dimensions and length. The
native tiled TIFF path shares that completion and keeps its ICC profile.
Arbitrary float construction retains the full finite check. The existing
metadata and orientation routes are preserved.
[Source, tree and host](exact-source-and-host.txt)

## Native red and green

The authenticated Bayer DNG fixture was decoded by the existing native
`cpu_image_io` test with the debugger observing actual entry into the
generic `validate_samples` function. The stock decode reached it once
through `raw::normal_developed_rgb`, failing the zero-scan assertion.
The prototype reached it zero times. The live EXR control reached the
generic validator twice at each revision. Its native test confirmed that
finite HDR values preserved their bits and that the nonfinite input
failed. An initial debugger-language calibration produced empty traces
and was corrected before the real red. [Red](native-red.txt)
[Green](green-observer-result.txt)
[Stock RAW stack](native-red-raw.jsonl.gz)
[Prototype RAW trace](green-raw.jsonl.gz)
[Prototype EXR stacks](green-exr.jsonl.gz)

Both the authenticated Bayer DNG and its independently derived horizontal
mirror preserved all 53,760 working f32 samples and the complete RGB16
TIFF bytes against stock. The mirror used the same Bayer payload and
EXIF orientation 2. Independent metadata inspection confirmed 160×112
unsigned RGB with three 16-bit channels. Both fixture hashes matched
before cleanup. [Working bits and TIFF hashes](parity-sha256.txt)
[Fixture identity](fixture-hashes.txt) [Parity](parity.json)
[TIFF metadata](tiff-metadata.json)

All 23 library and 28 native CPU image I/O tests passed, including the
length guard, source type and malformed CR3 path, DNG orientation,
metadata and source immutability, native TIFF, and nonfinite float
controls. Focused strict Clippy, formatting, reviewed source inventory
validation and its six policy regressions passed. Builds used Rust 1.100.0,
logical CPUs 14–15, up to two Cargo jobs and both compiler wrapper
settings empty. The two-CPU production fixture exercises the one-worker
RAW decode. Positive Canon CR3 originals were unavailable in the
committed fixtures and task-owned staging. Full R50 and the current
parallel RAW route still need authorized replay before integration.
[Native tests](focused-cpu-tests.log.gz)
[Focused lint](focused-clippy.log.gz) [Unit lint](unit-clippy.log.gz)
[Policy validation](miri-ledger-after.log.gz)
[Policy tests](miri-policy-regression.log.gz)

## Replay and cleanup

Use the exact stock `e81079d` or the scoped prototype, with the committed
Bayer fixture. Copy the retained helper sources without the trailing
`.txt` suffix into a task-owned directory. Build
`cargo +1.100.0 test --offline --locked -p pancake --test cpu_image_io
--jobs 2 --no-run --message-format=json` on CPUs 14–15 with compiler
wrappers disabled. Supply its test executable to the observer along
with the task-owned receipt path and a fresh label. The parity probe
builds as a temporary Cargo example and uses only the private DNG
fixtures and an empty output directory.
[Observer](observe-validation.sh.txt) [Fixture recipe](prepare-fixtures.py.txt)
[Native parity probe](typed-raw-probe.rs.txt)

The copied fixtures and produced TIFFs and working-bit files were
rehash-verified before cleanup. This experiment supplies native CPU
correctness and control-flow evidence. No device or R50 performance
result is attached. [Cleanup](cleanup.json)
