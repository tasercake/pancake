# Retained Hybrid input validation

Full-resolution inputs enter the job as immutable `RgbImage` values. The constructor checks extent, packed RGB length and finite values before the registration stage can retain the existing upload. The public borrowed `Frame` checks geometry and both public thumbnail entry points request a thumbnail alone. The production job is the retaining caller. It requests retention for the existing aligned Hybrid path without slabs.

`AdvancedAccumulator::add_normalized_frame` rescanned the host slice before checking for a retained device input. In that arm it used only `ResidentRgb`, which verifies the packed RGB extent, contiguous strides, f32 storage and originating compute service before submission. `0b4aeb3` moves the full validation into the host upload arm. Public `add_frame` always uses that arm. Invalid host shape or NaN still returns its existing diagnostic before submitting. The initial sticky failure check precedes selection, and both arms use the existing submission and synchronization path.

The private native witness ran on the Linux CubeCL CPU runtime with two 19×13 nonuniform RGB frames. The ordinary accumulator received validated host samples. The second used the actual production luma upload and dropped its host `RgbImage` before adding the resident input. The old code rejected the first retained frame with `sample count does not match packed RGB dimensions`. The candidate completed both and matched all 741 output f32 bit patterns against the host arm. The permanent integration regression checks short and NaN rejection through the public API, then accepts a valid frame and verifies the result.

The transient private test is retained as `private-probe.patch` inside `raw-native.tar.gz`. Applying it to either exact revision exposes the private boundary to the native test harness without adding a runtime call to the repository’s host-only library Miri roster. The probe was removed before checking the reviewed source manifest and running the permanent integration roster. The same locked Rust build and native test command was used for both arms.

## Performance scope

The earlier physical T50 phase trace at `64fbdb6` recorded 50 fusion validation spans totaling 942,718,406 ns. The T50 resident path can therefore avoid scanning 14.4 GB of logical host samples across the 50 RGB16 source frames after conversion to f32. The archived observation predates the latest TIFF decode and resident-input changes. It establishes a former stage cost and supplies no current speedup measurement. [Historical phase trace](../p10-t50-phase-trace/README.md).

The later `8694b55` T50 diagnostic measured 22.171825 s candidate versus 26.301475 s reference. A 0.5 ratio at that reference observation would require another 9.021087 s. The change here addresses one redundant scan. Physical RTX proof, current T50 timing and frozen acceptance are pending. [Latest diagnostic pair](../../../qualification/p10-performance/evidence/t50-hybrid-pair-8694b55/README.md).

The exact candidate passed all 17 selected native integration tests and the pinned scoped Clippy invocation. [Reproduction and provenance](receipt.json).
