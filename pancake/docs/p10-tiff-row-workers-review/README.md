# Independent TIFF row-worker review

Reviewed `3529ea4` against `287828a` and followed the native test scope
fix through `1a887017`. The review found one assurance blocker in the
initial revision. Its `tiled/tests.rs:40` and `:96` opened real temporary
files from the library test suite. The unchanged gate invokes that suite
with pinned Miri isolation. A supplementary Linux probe using the same
temporary file dependency and Miri flags exited 1 on the host `open`.
The revision moved the real-file worker, ICC and malformed later-tile
witness to `tests/cpu_image_io.rs:155`. The two exact pure library tests
from the revised module both passed in a supplementary pinned Linux
Miri harness. The canonical Darwin workspace gate remains separate.
[Initial isolation error](miri/miri.log.gz) [Revised test run](miri/pure-miri.log.gz)

The revised same-handle source at `tiled.rs:91–98` holds one mutex across
each seek and read. The independent buffered cursors cap payload reads
and require the declared chunk start at lines 112–157. The pinned TIFF
decoder explicitly seeks to that chunk start before expanding its data,
which clears older buffered bytes. The whole directory validation checks
counts and end offsets before assembly. The original TIFF decoder reads
the profile after tile workers join. Orientation other than none or
1 uses the normal raster fallback.

Complete groups lend disjoint mutable output bands. The band size is a
whole number of image scanlines, tail bands account for the bottom edge,
and the checked tile stride limits the right edge. The lane budget caps
the extra threads at three, and each working tile holds at most 6 MiB
of RGB16 samples. Separate TIFF metadata and codec buffers add to that
tile scratch budget. The process-wide permit covers all worker joins.
Ordinary decode and spawn errors release it, joined worker panics
report an error, and a caller-thread unwind can poison it into a serial
fallback. No reviewed production path needs a change for that fallback.

The single original open handle holds file identity through a pathname
replacement. In-place changes to that same object remain outside this
guarantee. The worker rechecks layout and its span list at lines 380–386.
Those checks do not compare every codec tag or snapshot the payload.

## Verified compatibility

An isolated harness included the exact production tiled module from
both revisions and exercised two independently generated 1025×1027
images. Classic TIFF Deflate and big-endian BigTIFF with the legacy
Deflate tag and horizontal predictor both matched the coordinate
formula and pinned TIFF native decoder across all 3,158,025 RGB values.
The ICC bytes matched. EXIF orientation 6 selected the standard raster
fallback. Replacing a pathname after opening the source retained the
original samples and profile. The native probe used Rust 1.100.0,
`tiff=0.11.3` and CPU affinity 0–3. The auxiliary crate exercised the
decoder component without a full Pancake build.
[Fixture generation](native/generate.py.txt) [Initial results](native/native.log)
[Revised results](native/revised-native.log)

The developer's generated-T50 diagnostic uses byte-identical
`tiled.rs` from 287828a and 3529ea4. Its separate probes only remove the
unused non-TIFF front-end paths. The alternating probe compared all
72 million sample bits per frame on three existing frozen T50 frames.
Independent calculation from the retained samples confirms medians
0.471289 s and 0.140305 s. Fresh one-frame diagnostics recorded
285,440 and 287,664 KiB peak RSS. The four-CPU candidate used three
extra file workers and the one-CPU candidate used none. These are
decode-stage diagnostics on the initial source.
