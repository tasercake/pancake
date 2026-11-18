# Public RAW image API assurance

Clean `fbf01d1981546d017c5bacb762c789b16bbf51af` was the focused baseline, and
the same production source, manifests and lock persisted at the test revision
`d8bc1e2`. The public `pancake::image_io::decode` entry point exercised both
RAW formats. The CPU suite used the repository's pinned nightly compiler,
cargo-llvm-cov 0.9.1, two build jobs and stripped local debug information.
No GPU, desktop or benchmark lane was used.

The three permanent DNG tests verify an asymmetric 73×53 metadata crop
against independent row arithmetic over the full developed Bayer frame.
A camera exposure chart declares a black offset, limited white level and
unequal per-channel normalized responses. Interior patches must yield the
standard neutral sRGB levels for four known linear intensities, with at
most two RGB16 code steps of error. The malformed metadata cases remove
each required sensor field or declare zero width. These must fail with
the corresponding preflight diagnostic while preserving the source bytes.

The independent CR3 probe decoded the exact unmodified R50 file
`frame_000.CR3` to 5088×3392 RGB16. ExifTool 12.76 supplied the independent
recommended size. The probe used the same actual container for three
negative controls. It enlarged the second sensor track beyond the declared
axis bound, set its height to zero, then removed both CMP1 identifiers. All
three failed through the public decoder with the specific preflight
diagnostics. The full original and temporary controls were removed.
The permanent automated suite needs only its existing generated Bayer DNG.

| Selected CPU image I/O run | Tests | RAW lines | RAW functions | RAW regions | RAW branches |
| --- | ---: | ---: | ---: | ---: | ---: |
| Baseline | 21/21 | 60/101 | 11/28 | 66/118 | 7/8 |
| Permanent DNG tests | 24/24 | 61/101 | 12/28 | 71/118 | 7/8 |
| DNG tests and actual native CR3 probe | 25/25 | 80/101 | 15/28 | 89/118 | 7/8 |

This report measures one focused integration target. The complete
zero-uncovered-source and mutation assurance gates remain open.

## Reproduce

Run from the `pancake` directory at each indicated revision. Use a fresh
ignored evidence directory for the resulting logs and coverage JSON.

```sh
CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  CARGO_PROFILE_TEST_DEBUG=0 \
  cargo +nightly-2026-11-15 llvm-cov --locked -p pancake --all-features \
    --test cpu_image_io --branch --json --output-path "$proof/coverage.json" \
    -- --include-ignored
```

For the separate native CR3 diagnostic, use a disposable checkout of the
test revision. Verify the corpus file's SHA-256 against the frozen R50
manifest, set `PANCAKE_R50_ORIGINAL` to its absolute path, and temporarily
append the retained probe to `tests/cpu_image_io.rs` before rerunning the
same command. The captured source and original hashes are in
`test-sources.txt`. The actual native file was `frame_000.CR3` by Johannes
Sood, licensed CC BY 4.0, from corpus revision
`0f8256edf2d2e3f1f1ea162f78a8bca36eb6915c`.
[Original dataset](https://huggingface.co/datasets/jjjsood/focus-stack-sample)
[License](https://creativecommons.org/licenses/by/4.0/)
