# Exact resident RGB allocation mutation replay

At `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`, the saved canonical
survivor `src/job/stream.rs:199:31` replaced `settings.slab_size == 0`
with `!= 0` inside the registration RGB retention policy. The saved
cargo-mutants patch SHA-256 is
`e94307b97fadc0716aa9798ae32b2faef1d409457cc926e551ac5e6e49291274`.
The replay applied those exact saved bytes to `src/job/stream.rs`.

The one-test native CPU integration executable pauses the real job on
its first `smooth_luma` launch. Both runs complete registration readback
before acknowledging the pause. Before the first fusion submission,
the test reads the runtime's actual active allocation count and bytes
through `Client::memory_report(MemoryScope::Device)`. Configured slab
sizes zero and two have identical effective two-frame Exposure Fusion
plans. The unslabbed job keeps one 9,216-byte packed f32 RGB allocation
alive at that boundary. A separate binary isolates the process-global
launch observer and native allocation pools.

Both jobs resume and complete using native CPU. The test requires
bit-identical full f32 samples, RGB16 TIFF bytes, registration maps and
completion events before checking the allocation delta. Its scope guard
cancels a paused job on an error to allow the worker to settle. The final test
uses no elapsed-time threshold.

Four fresh executions passed on the stock source, two before and two
after the mutation. The exact mutant completed both outputs but inverted
the allocation delta: the unslabbed job held 10 live allocations and
16,512 bytes, while the configured-slab job held 11 and 25,728 bytes.
It failed the allocation assertion with exit 101. The restored source
passed with no production code, dependency or lock changes. The final test
source is `a3a1037`.

Rust 1.100.0 used the locked offline CPU graph with two direct Cargo jobs
pinned to logical CPUs 14 and 15. The archive retains the original
mutant, actual applied diff, both generations of test commands and results, source and
lock hashes, and static checks. This is an exact targeted native catch.

The initial scoped lint rejected unchecked JSON field indexing. The final
test uses checked getters to require explicit CPU completion, nonphysical
device provenance and a null fallback. The final version repeated the
four stock passes and exact mutant failure. Effective-policy Clippy with
warnings denied, workspace formatting, the reviewed Miri source inventory,
and native CPU target classification passed. The test exercises the
Exposure policy shared by `0a9ae85` and the later Weighted Average
candidate `a49ce69`. The latter adds a separate eligible method while
retaining the configured zero-slab condition.

[Exact native replay records](raw-records.tar.gz)
[Archive checksum](raw-records.sha256)
