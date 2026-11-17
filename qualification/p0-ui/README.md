# Native UI qualification

This standalone P0 harness proves a small real Slint UI, safe CubeCL compute and
shared wgpu resources.

From this directory:

```sh
cargo +1.100.0 build --locked
bash check.sh
```

Set `CARGO_TARGET_DIR` when sharing a build cache. `check.sh` uses the root pinned tools directory unless
`PANCAKE_TOOL_BIN` is set. Linux needs a physical Vulkan GPU, Xvfb, Openbox,
xdotool, xprop, D-Bus, AT-SPI, system Python GI and the suite Python dependencies.
Missing hardware or controls fails the run; software adapters are rejected.
The oracle enables accessibility only on its owned private D-Bus session.

The source image is a generated 2048 x 1536 u16 checker, processed by the root
checked CubeCL blur. A fixed downsample creates one 512 x 384 RGBA8 display
copy. Slint and CubeCL import the exact same wgpu 30 device and queue. The
renderer confirms those identities before the oracle accepts computed pixels.
No alternate algorithm, unsafe buffer cast, control socket or synthetic progress
is used. The worker executes real batches, polls cancellation before submitting
the next batch, and accepts an explicit shutdown message after window close.

Commands and pending previews each have one slot. One worker owns at most two
full u16 planes; staging frames have a structural maximum of three owners
(producer, pending queue and presenter); the texture cache holds two slots.
These bounds do not claim whole-process or GPU peak memory. See
[recorded qualification](../../pancake/docs/p0-ui-qualification.md) for evidence,
limits and the interpreter trust boundary.
