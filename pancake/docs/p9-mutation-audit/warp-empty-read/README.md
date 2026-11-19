# Registration warp readback progress

The pinned native mutation shard at `287828aac07ffad2252785b73a52a59fc07b3409` timed out after 60.058 seconds when cargo-mutants replaced the entire `registration::kernels::read` body with `Ok(vec![])`. The native log ends in `cpu_registration_contract`. The red receipt and raw test log came from local evidence revision `e4640932c28e9125b79c3f3d409e2f51d0b8f1d4`. The complete `warp` and `read` bodies match the pre-fix base `8694b555666a1ddf4051e2839bbced4bcc625859` byte-for-byte.

The old compositor took the next tile offset from the accumulated readback length. The forced `Ok(vec![])` made that offset stay fixed and bypassed the native blocking read. Normal `read` validates the exact per-tile count before returning samples. The mutation forces a producer-contract violation. It supplies no evidence of an observed short native read or device loss.

The revised warp enumerates positive tile lengths bounded by the checked nonempty RGB extent, then uses the existing `Frame::new` shape check on the assembled output. Each tile uses the existing dispatch and blocking read with its exact sample count. The new native test submits a 301×293 identity warp with nonuniform F32 input and checks all 264,579 samples, including the partial final tile of 2,435 samples.

Both native CPU tests passed on `1ea0c4f394489aec9d77b253a81165c1554d32ad`. The one selected mutation compiled successfully and was caught in 0.254 seconds of Cargo test execution. Both tests returned `registration frame does not match RGB dimensions`. The mutation run exited successfully with one caught and zero missed, timeout, or unviable candidates. Rustfmt and the reviewed Miri source and dependency policy also passed.

The replay uses local x86-64 Linux, AMD Ryzen 9 7950X3D CPU, two build jobs on logical CPUs 20 and 21, Rust 1.100.0, cargo-mutants 27.1.0, and locked offline commands. `receipt.json` records provenance and results. `raw-native.tar.gz` retains the original timeout witness, selected mutant and actual new baseline and mutant logs. This mutation evidence covers the named native CPU test target and forced producer failure.

## Locked P5 release replay

The exact 4097×257 cascade fixture uses 12 complete warp tiles and a final tile of 13,059 samples. A native `__libc_malloc` entry breakpoint attributed two full-frame requests in each release to `registration::kernels::warp`. The staging request was 12,635,152 bytes and the result request was 12,635,148 bytes in both arms. Those arms produced identical PNG bytes and transform events. An earlier `mmap`-only trace found no attributable warp syscall and could not count allocator requests. The raw archive retains that attempt and the attributable libc trace.

[`p5-receipt.json`](p5-receipt.json) records the source, build, binaries, suite and lockfile hashes, device identity, and the 64-case accounting. `p5-raw.tar.gz` contains the frozen selectors, commands, reports, case events and allocation traces. Exact native CUDA verification and the full mutation gate remain pending.
