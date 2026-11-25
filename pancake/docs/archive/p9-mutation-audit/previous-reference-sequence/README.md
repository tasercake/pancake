# Previous-reference sequence mutation

The `0a9ae85` native mutation shard recorded `MissedMutant` for the single
`src/job/registration.rs:213:35` replacement of `==` with `!=`. The typed
`mutants.json` row and saved mutation diff matched byte for byte. The diff
SHA-256 is `c5ece2e76c3a9a1fa746d45a66b5076622d82a6836f9c072f9657472841f00d7`.
[Original typed roster entry and result](original-survivor.json), [exact diff](native.diff).

Clean test revision `71c27bd280ef0e1239b859c8da48d9f089157a4d` extends the
existing 63×49 native CPU test with its retained original, three-pixel-shifted,
and returning original frames. The public result maps the initial canvas
into each source. Its sequential observer instead maps the previous frame
into the next source, so the correct order is identity, +3, then −3 pixels.
Both existing Previous-reference models, Similarity and Breathing, run
through their genuine output and observer paths. The test also checks common
geometry and the published 16-bit crop against the retained result.

The original saved diff compiled and failed the new sequential assertion
with exit 101. It reported three identity matrices. The old common map
checks can pass because a stale first-frame proxy still sees a shifted
second frame and a returning original third frame. The observer distinguishes
that from actual previous-frame progression. Reversing the exact diff
restored production source byte for byte. The focused clean test and all
six existing `cpu_stack_plan` tests then passed, and scoped strict Clippy
and formatting passed. [Mutant log](mutant.log.gz), [restored test](original.log.gz),
[full target](full-target.log.gz), [Clippy](clippy.log.gz), [restoration](restored.log).

Both test arms used Rust 1.100.0 on Linux x86-64 with an AMD Ryzen 9 7950X3D.
Cargo ran offline on CPUs 10 and 11 with two build workers and compiler
wrappers unset. No desktop or GPU ran. The original mutation shard remains
recorded as run. This experiment catches its one exact survivor.
[Replay commands](command.txt), [timings](mutant.time), [checksums](SHA256SUMS).
