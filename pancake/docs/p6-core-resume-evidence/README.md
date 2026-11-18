# Shared job and runtime assurance

Reviewed starting source `bbb4767`. The prior nightly coverage map at
`5216ba4` has identical Cargo.lock and job/runtime source. Its file-level
segments identified untested public job edges. Native CPU cases now cover
pausing a real stack before publication, exact manual crop pixels, invalid
source and override text, runtime budget and index rejection, and automatic
CPU completion when an isolated Vulkan loader finds no physical GPU.
The tests use real encoded RGB16 temporary images and independent re-decodes.
Runtime fault classification checks the upstream skipped-work error type.

## Renderer reuse

A physical NVIDIA Vulkan test before the fix showed that an already prepared
renderer bypassed the new job's one-byte budget check and reached fusion.
The equivalent fresh request failed during admission against the 44-byte
runtime probe. Both paths now validate the same job budget, including any
total capacity exposed by the selected runtime. The runtime can still be
reused for a valid job. The passing test independently verifies export pixels
and a physical GPU completion record. The RTX 3090 Vulkan runtime reports
its per-allocation limit and leaves total capacity unspecified.

| Command | Source | Observed result |
|---|---|---|
| `cargo +1.100.0 test --locked --test gpu_runtime_contract prepared_wgpu_respects_each_jobs_budget` | Before the budget fix | Exit 101 with distinct fresh and reused diagnostics. [Log](prepared-wgpu-red.log.gz) |
| `cargo +1.100.0 test --locked --test gpu_runtime_contract -- --test-threads=1` | `2375efa` application source | Four passed on NVIDIA Vulkan. [Log](prepared-wgpu-green.log.gz) |
| `cargo +1.100.0 test --locked -p pancake -p pancake-cli --lib --test cpu_stack_plan --test cpu_job --test cpu_runtime_contract --test cpu_runtime_options` | `2375efa` application source | 41 passed. [Log](native-final.log.gz) |
| `bash scripts/check.sh fast` | `2375efa` | Exit 0, including 42 guardrail tests, strict Clippy and rustdoc. [Log](fast.log.gz) |

GPU tests used the system NVIDIA Vulkan ICD via `VK_DRIVER_FILES`, with
task-owned runtime and compute-cache directories and both display variables
unset. The fallback test supplies an absent Vulkan ICD to its own subprocess,
disables that subprocess's CUDA visibility, and verifies actual CPU
completion for Auto while forced GPU preserves the existing export.
[Platform and lock](platform.txt).

## Pinned nightly CPU coverage

`bash scripts/check.sh branches-cpu` ran at `2375efa` with
`nightly-2026-11-15`, `cargo-llvm-cov 0.9.1`, four build jobs,
`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, the registry-only
compiler cache wrapper and a worktree-owned `CARGO_TARGET_DIR`.
All 106 tests passed. The gate returned exit 1 on uncovered CPU source.

| Scope | Lines before / after | Functions before / after | Regions before / after | Branches before / after |
|---|---:|---:|---:|---:|
| Job production | 882/953 → 898/954 | 82/101 → 86/101 | 810/996 → 833/998 | 124/160 → 135/160 |
| Runtime production | 134/316 → 223/317 | 22/55 → 35/55 | 124/327 → 218/330 | 14/38 → 26/38 |
| Workspace | 3913/5773 → 4018/5775 | 385/670 → 402/670 | 3988/6549 → 4105/6554 | 466/612 → 489/612 |

The per-file comparison preserves the full baseline and candidate totals,
LLVM report hashes, and each source path included in the scoped totals.
The shared job/runtime source matched the preceding observation before these
changes. The workspace includes changes since that baseline outside shared
job/runtime scope. [Comparison](coverage-comparison.json),
[raw LLVM report](branches-cpu.json.gz) and [command log](branches-cpu.log.gz).

## Scoped lifecycle mutations

At `2375efa` the unmutated core baseline passed. The pinned mutation tool
tested all 18 generated mutants of five selected lifecycle methods.
Seventeen were caught, one timed out, and none were missed or compiler
unviable. The timed-out variant replaced resume with `Ok(true)` without
releasing the paused worker. The scope returned exit 3.

```sh
cargo mutants --package pancake --file src/job/lifecycle.rs \
  --re 'JobControl::(claim|complete_frame|commit|pause|resume)' \
  --cap-lints true --all-features --in-place --timeout 60 --build-timeout 180 \
  --output ../artifacts/core-assurance/mutation \
  -- --locked --lib --test cpu_job --test cpu_stack_plan
```

[Selection](mutation-selection.txt), [outcomes](mutation-outcomes.json) and
[command log](mutation.log).
