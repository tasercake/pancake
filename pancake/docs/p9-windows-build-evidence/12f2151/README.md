# Windows MSVC build-only investigation

Tested revision `12f2151c8a198b5209c8f4d506dbd17fd9aaa74f`, lock SHA-256
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.
Host Ubuntu 24.04.5 LTS, Linux x86-64, AMD Ryzen 9 7950X3D. Pinned
`rustc 1.100.0 (0ef1bc386 2026-11-09)` and
`cargo 1.100.0 (3acc79c0c 2026-11-05)`.

From the Pancake workspace, with a fresh task-owned target and one build job:

```sh
export CARGO_TARGET_DIR="$PWD/target/windows-retained-build/cargo-target"
export CARGO_BUILD_JOBS=1 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0
cargo test --locked --workspace --all-targets --all-features \
  --target x86_64-pc-windows-msvc --no-run
```

The retained CI-shaped command exited 101 in `serde_core` with E0463. The
pinned Rust installation contains only the Linux standard library target.
The host also has no preinstalled Microsoft SDK or VC CRT sysroot. [Cargo output](msvc-no-run.log).

The target dependency graph resolves CubeCL CPU, CubeCL LLVM, wgpu, Slint and
rfd. The locked Tracel LLVM bundler 23.1.0-4 is a build dependency. Its
`Environment::current` chooses the bundle from compile-time host
`cfg!(target_os)` and `cfg!(target_arch)`. Its linker helper likewise uses
host cfg to choose `llvm-config` and the C++ runtime. Inference from those
implementations and Cargo's documented build-script cfg semantics is that
the unmodified Linux-host graph would select Linux LLVM even after installing
the Windows Rust target and SDK. [Cargo build-script environment](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

For an independent linker check, `llvm-config` from the installed locked
Tracel bundle reported 23.1.0, `x86_64-unknown-linux-gnu`, and system
libraries `-lrt -ldl -lm`. The first member of its `libLLVMCore.a`, extracted
using `ar p`, was an ELF relocatable object. Rust's bundled LLD 23.1.5 COFF
linker rejected that object with “unknown file type” and exit 1:

```sh
sysroot=$(rustup run 1.100.0 rustc --print sysroot)
linker="$sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin/gcc-ld/lld-link"
"$linker" /dll /noentry /nodefaultlib /out:check.dll tracel-llvm-host-object.o
```

[LLVM object and linker output](llvm-archive-link.log).

The exact release still needs the native `windows-2025` build-and-link check
already specified in the workflow. Windows application execution was unrun.
The cross-build produced no Windows executable. The investigation made no
system, product, toolchain or lockfile changes.
