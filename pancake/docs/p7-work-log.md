# P7 native UI development

The first safe Slint application checkpoint is
`ea487ef3bbbce3e04212895aa758f479aca79be3`. Its isolated Linux native run
passed these real application cases, using the unchanged contract and a
descriptor that pointed at the debug GUI binary.

The checkpoint used Rust 1.100.0, Ubuntu 24.04 x86_64 kernel
6.8.0-147-generic and a Ryzen 9 7950X3D host with a GeForce RTX 3090 and NVIDIA
580.178.04. These six cases exercise visual controls, preferences and native
input. They supply no physical GPU computation evidence.

Run `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo build --locked
--jobs 4 -p pancake-gui` from `pancake/`. The tested executable SHA-256 was
`f7e8bc7cc759895f3b6eb49e027719dc883b5332625a967e4c4bcef72aae5ee5`.
Cargo.lock SHA-256 was
`6b2cac8f004397ed97a4fb45cab802acccc86bf3e33e67de33074f8ab75864e3`.
The focused GUI Rust tests and strict Clippy checks with the
policy-generated configuration passed. The marked checkpoint has no
integrated GPU job or release build result.

### Isolated desktop setup

Source import, render, stack jobs, export, lifecycle and the remaining GUI
cases require further implementation and independent verification.
