# Linux HIP release linkage for 11cfcf8

The locked headless CLI built and linked on exact clean revision
`11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6` using Rust 1.100.0, two direct
offline jobs and CPUs 12 and 13 of a Linux x86-64 Ryzen 9 7950X3D host.
GNU time recorded 4m19.04s through release compilation and linkage.

```sh
cd pancake
env -u CUDARC_CUDA_VERSION -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER \
  CARGO_INCREMENTAL=0 taskset -c 12,13 cargo +1.100.0 build \
  --locked --offline --release -p pancake -p pancake-cli --bin pancake-cli \
  -j 2 --features cubecl/hip
```

The 164,657,400-byte ELF has GNU Build ID
`f96fc289e6f8738018796548caa20cfa104a5d67` and SHA-256
`7f3eb0977f522326b0ccbefcb5b1bcb0ac1d3e11c1344796dda8898c4da03d7e`.
Both selected dependency roots resolve CubeCL HIP, CPU and wgpu. The
CLI feature graph is frontend-free, and the ELF defines 270 HIP runtime,
78 HIP binding, 175 CPU and 2,034 wgpu symbol matches. The graph and ELF
checks recorded zero CUDA or Slint frontend matches.

The pinned `cubecl-hip-sys` 7.14.6085001 build script emitted the single
`hip_60850` binding ABI. As in the earlier 0a9ae85 recipe, the build used
its default binding selection when `hipconfig` was unavailable. The
bindings load `amdhip64` and `hiprtc` dynamically. This receipt qualifies
compilation and linkage. The artifact is retained read-only at
`artifacts/pancake-linux-native/11cfcf8/pancake-cli-hip`.
[Typed receipt](receipt.json) · [Raw records](raw-build-records.tar.gz) · [Archive checksum](raw-build-records.sha256)
