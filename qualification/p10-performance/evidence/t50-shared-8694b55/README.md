# Shared frozen T50 stage for the 8694b55 release

The materializer used a clean checkout of
`8694b555666a1ddf4051e2839bbced4bcc625859`. Its generator and scalar oracle
match the prior qualified T50 materializer. The frozen receipt and
performance targets retain their original hashes.

All 50 ordered 6000×4000 RGB16 focus TIFFs and the two independent truth TIFFs
were generated. All 52 encoded file hashes and both independent decoded hash
sets matched the freeze, totaling 2,775,174,405 bytes. The materialization
receipt has SHA-256
`317f4b0c9be5837cd7082227bf700ee232e7e995471ee44be8488cbf3511603c`.
The two truth files remain separate from benchmark source arguments.

The generator finished between 20:22:52 and 20:26:37 SGT on 2026-11-18.
It ran on CPU14, nice +12, idle I/O priority, one decode worker and numeric
thread limits of one. Runtime was 224.730 seconds, peak RSS was 83,692 KiB,
and generator reserve was 15 GiB. Available disk space was 120,291,086,336 B
before, and 127,636,058,112 B after the generation window. Concurrent
ordinary work includes the authorized cleanup of R50 scratch. Available host
memory was 46,441,680 KiB at entry and 44,957,144 KiB at exit. The shared lock
covered the complete generation. No application or GPU benchmark ran here.

The [stage receipt](shared-stage.json) records the temporary corpus used by
the matched full-product Hybrid pair, the packaged 400-event viewer, and the
full-stack 80-event loaded attribution trace. All three terminal receipts
identify this materialization and verify the source inputs after use. The
two borrowers released all reads. The owner authenticated the directory
and rehashed every file before removing all 52 TIFFs and 2,775,174,405 bytes
at 20:52:47 SGT. No staged TIFF remains.
[Matched role proofs](released-borrower-roles.json) ·
[Cleanup summary](cleanup-summary.json) · [Authenticated cleanup](cleanup.json.gz)
