# Shared frozen T50 stage for the exact e81079d Weighted Average CPU pair

The materializer ran from the clean `e81079dee4078d7a1149da0f5950aab8c89f8d44`
checkout. The frozen generator, scalar oracle and performance targets
retain their recorded hashes. The sealed Debian package and its verification
receipt also matched their recorded hashes.

All 50 ordered 6000×4000 RGB16 focus TIFFs and both native 16-bit truth TIFFs
matched the frozen encoded hashes and both independent decoded pixel hashes.
The full staged set totals 2,775,174,405 bytes. The materialization
receipt SHA-256 is
`1d9be3f1cd20a01083b11c10c4d17ab75a4f3b6bb7e2ebef682824e345ba649d`.
The corpus contains the CC0-1.0 generated images from the original frozen
recipe. Both actual applications will receive the same 50 focus TIFFs. The
two independent truth TIFFs remain separate from their input arguments.

Generation took 243.743 seconds with peak RSS of
83,680 KiB. The owner held the shared performance
lock from 00:20:03 to 00:24:07 SGT on 2026-11-18 and limited generation to CPU6,
nice +12, idle I/O, and one numerical thread and decode worker. The generator
reserved 15 GiB. Available disk space was 124,353,531,904 bytes at entry and
118,409,609,216 bytes at exit. Available memory was 45,962,312 KiB at
entry and 45,288,836 KiB at exit. This run generated and verified
input data. The prelaunch census and common lock excluded timed jobs. An
independent setup step used CPU7 to extract and verify the sealed Debian CLI
and instantiate the 46-package frozen reference environment offline. The
archived frozen-runner preflight checked both exact planned argv lists,
including all 50 ordered source names and explicit CPU. It started no jobs.

The named borrower is the matched Weighted Average CPU diagnostic. The owner
retains the corpus until the pair and its authenticated postflight release
the stage and the coordinator clears cleanup.
