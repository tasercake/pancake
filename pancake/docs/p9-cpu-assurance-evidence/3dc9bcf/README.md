# Integrated Linux CPU assurance at 3dc9bcf

The [platform record](platform.txt) identifies the Linux x86-64 host, RTX
3090 driver, GTK/ATK and compiler. The [replay script](replay-exact-gui-assurance.sh.txt)
selects the pinned tools, owned cache and XDG runtime paths, and unsets display
and sanitizer override variables. CPU tests used versioned Rust fixtures,
including real CPU jobs, CLI subprocesses, malformed image files and GUI
binary unit tests. The physical-GPU sanitizer boundary is recorded separately.

| Check | Actual outcome |
|---|---|
| `check.sh fast` | Exit 0. Strict Clippy, rustdoc, policy, infrastructure and graph tests passed. [Log](fast.log.gz) |
| `check.sh deps` | Exit 0. Dependency and vendor checks passed. [Log](deps.log.gz) |
| `check.sh coverage-cpu` | Exit 1. All 97 tests in 19 targets passed with zero skips. Source report covered 3,913/5,640 lines, 385/656 functions and 3,988/6,383 regions. GUI accounted for 330/1,609 covered lines. [Log](coverage-cpu.log.gz) [LLVM report](coverage-cpu.json.gz) |
| `check.sh sanitizer address` | Exit 0. All 97 tests in 19 targets passed, including GUI binary unit tests. [Log](sanitizer-address.log.gz) |
| `check.sh sanitizer thread`, normal 0–31 CPU affinity | Exit 101 after external SIGTERM to the test-owned advanced fusion process. It ran 6:04, consumed about 30 CPU cores and stopped before the other integration targets. [Log](sanitizer-thread.log.gz) [Process sample](thread-stall.txt) |
| Same TSan advanced fusion binary, restricted to CPU IDs 0–3 | Exit 0. The focused integration test finished in 3.12 seconds. [Log](thread-affinity.log.gz) [Command and binary hash](thread-affinity.txt) |
| `taskset --cpu-list 0,1,2,3 bash scripts/check.sh sanitizer thread` | Exit 101. 74 tests passed, one failed and 22 were unrun. The real CLI relative-glob case emitted a TSan warning during CubeCL CPU startup. [Log](sanitizer-thread-affinity.log.gz) |

The bounded runs used the same instrumented compiler output, no race suppressions
and four allowed IDs on distinct physical cores. The full diagnostic command
used `timeout --signal=TERM --kill-after=5s 240s` around `taskset`. The pinned
nightly and `-Zbuild-std` selection were unchanged. The [CLI hash](thread-cli.sha256)
and [ELF build ID](thread-cli-build-id.txt) identify the executable in the
warning stack.

The bounded TSan warning relates a sender's `AcqRel` atomic update in
`oneshot` 0.2.1 to deallocation in the receiver after an acquire fence.
CubeCL 0.11.4 uses the channel during `DeviceRunner::start`.
Rust documents that TSan lacks atomic-fence support. The source sequence
makes an instrumentation limitation plausible. An independent discriminator
and reviewed fix or tool-boundary decision remain necessary.
[Rust sanitizer documentation](https://doc.rust-lang.org/unstable-book/compiler-flags/sanitizer.html#threadsanitizer)

Stable coverage did not instrument branches. Linux TSan and complete source
assurance remain open. Canonical Darwin Miri and native platform or package
acceptance retain separate obligations.
