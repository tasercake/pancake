# P10 comparison viewport repair

The sealed Linux release `5216ba4` rendered the comparison source too far
right. Independent projection of native T50 framebuffer captures localized
the shift to the comparison rectangle's default horizontal centering.
The original [case accounting](../p7-ui-evidence/5216ba4/interface-accounting.json)
and package remain bound to the sealed revision. The physical diagnosis is
retained in `eb2f9c5` under
`qualification/p10-performance/evidence/native-physical-pair-calibration/`.

At `07d74bb`, the embedded workspace's real Slint software renderer showed
result RGB `(237, 19, 63)` where the left side of the default split required
source RGB `(13, 229, 91)`. The regression derives pixel probes from the
visible result and checks both sides of the default split. It dispatches
the actual Slint pointer press, move, and release to change the split,
then checks that the middle probe switches images while the outside probes remain correct.
[Expected red result](renderer-red.log).

The production clip at `74dfaa3` binds its origin to the viewport origin.
All 12 GUI unit tests, strict GUI Clippy, formatting, and the reviewed
source and dependency policy passed. Cargo and Rust 1.100.0 built the
locked release with `CARGO_INCREMENTAL=0 cargo build --locked --release --jobs
4 -p pancake-gui -p pancake-cli`. The CLI digest matches the historical
seal, and the new GUI digest is bound to `74dfaa3`.
[Green test result](renderer-green.log), [Clippy](clippy.log), and
[build and binary digests](release-build.log).
