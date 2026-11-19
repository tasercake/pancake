# Exact integrated supplemental Linux Miri

At clean `0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`, pinned Miri passed all
23 actual workspace library tests with zero failures, ignored tests or
filters. The selected fixtures included the 32,768-pixel typed RAW allocation
and working-sample comparison, TIFF cursors and lane policy, and host
lifecycle, readback and resource validation. Strict provenance and symbolic
alignment checks remained enabled. The reviewed-source gate passed and the
Miri process exited 0. [Original log](miri.log.gz), [selected tests](selected-tests.txt),
[reviewed source](reviewed-source.log), [exit status](miri.status).

This is the documented supplemental Linux command used by the earlier
integrated run. It exercised the current library in the exact checkout.
The canonical Apple Miri gate remains separate.
[Earlier Linux Miri evidence](../8694b55/linux-nightly/README.md).

The pinned `nightly-2026-11-15` toolchain reported Miri 0.1.0
(`5f4effd579`) and LLVM 23.1.5. The AMD Ryzen 9 7950X3D Linux x86-64 host
ran offline on CPUs 0 and 1 with at most two build jobs. The entire cold
command took 3m26.77s and reported 1,481,828 KiB peak RSS. No process
suspension, desktop session or device case occurred.
[Exact command](command.txt), [toolchain and input hashes](input-hashes.txt),
[timings](miri.time), [checksums](SHA256SUMS).

Source Git tree `fde57a0b461e3ea031ea969e1e841927bead17f9`.
Original test log SHA-256 `99e408141e407cc2a02ef8526b83c90ce98b6edc32716a1d74e564ca7fa9c23f`.
