# Shared frozen T50 stage for fb9c399 diagnostics

The materializer ran from the clean `fb9c399a96658331cb76b843a1e565b503e94fdd`
checkout. The frozen generator, scalar oracle and performance targets
retain their recorded hashes. The sealed Debian package and its verification
receipt also matched their recorded hashes.

All 50 ordered 6000×4000 RGB16 focus TIFFs and both native 16-bit truth TIFFs
matched the frozen encoded hashes and both independent decoded pixel hashes.
The full staged set totals 2,775,174,405 bytes. The materialization
receipt SHA-256 is
`71c81abc2af03edaf43373ee3957757dfd9b692a00af84e5e38381ceba3a92d1`.
The application diagnostics receive the 50 focus TIFFs. Both truth TIFFs
remain separate.

Generation took 225.980 seconds with peak RSS of
83,536 KiB. The owner held the shared performance
lock from 22:15:39 to 22:19:25 SGT on 2026-11-18 and limited generation to CPU14,
nice +12, idle I/O, and one numerical thread and decode worker. The generator
reserved 15 GiB. Available disk space was 128,073,469,952 bytes at entry and
123,104,538,624 bytes at exit. Available memory was 44,909,624 KiB at
entry and 45,080,652 KiB at exit. This run generated and verified
input data. The sealed Debian package was identified by archive and package receipt
hashes. Application and device measurements are tracked by their consumers.

The isolated CLI comparison completed four physical GPU jobs, verified
identical outputs, and rehashed all 52 inputs. The same-lock viewer trace
captured 280 events, verified all 50 focus files unchanged,
and recorded a clean native exit. Both readers released the stage. The owner
authenticated the directory and rehashed every TIFF before removing all 52
files and 2,775,174,405 bytes at 22:44:39 SGT. No staged TIFFs remain.

[Borrower receipts](released-borrower-roles.json) ·
[Cleanup summary](cleanup-summary.json) · [Authenticated cleanup](cleanup.json.gz)
