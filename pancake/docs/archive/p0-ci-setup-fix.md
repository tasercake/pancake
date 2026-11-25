# P0 hosted-runner setup repair

Run [44090076402](https://github.com/tasercake/focus-stacking/actions/runs/44090076402),
source `85ae9433f35220761d390e5aa4a90eb3363114c3`, failed before application
checks could complete. The six complete failed-job logs are retained in
`p0-ci-evidence/red-*.log.gz`.

| Failed job | Exact failure | Repair |
| --- | --- | --- |
| macOS platform | `go: command not found` while building actionlint | Install the pinned upstream release binary; no Go toolchain requirement. |
| Ubuntu platform | `Required actionlint 1.7.12; found: v1.7.12` | Official release reports the exact existing version token; preserve strict version check and pin. |
| Coverage, nightly, ASan, TSan | `yeslogic-fontconfig-sys v6.0.1` cannot locate `fontconfig.pc` | Provision explicit Linux native development packages before Cargo in every Linux build job. |

[Slint 1.19's build guide](https://github.com/slint-ui/slint/blob/v1.19.0/docs/building.md#linux)
requires fontconfig, xcb shape/xfixes, xkbcommon and OpenSSL for Linux development.
The shared runner script installs `pkg-config`, `libfontconfig1-dev`,
`libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libxkbcommon-dev` and `libssl-dev`,
then verifies their pkg-config records. Fontconfig pulls its FreeType development
dependency. Qt, multimedia examples and feature changes are unnecessary. The
script refuses execution outside a Linux GitHub Actions runner; no developer
host packages are changed by local verification.

Actionlint remains 1.7.12. Four Linux/macOS release archives have literal reviewed
SHA-256 values taken from the official
[checksum asset](https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_checksums.txt)
and matching release API digests. New platforms or versions fail until reviewed.
Only a checksum-verified archive is extracted, and exact executable version is
checked before installation. The actual macOS ARM64 release was downloaded and
run with `PATH=/usr/bin:/bin`, without Go. That binary passes workflow lint.

A separate hosted-runner flaw was found before rerun: `mutants` runs physical-GPU
integration tests. New `mutants-cpu` uses the existing fail-closed integration
classifier to select the library and every `cpu_*` target. It mutates the same
production library and retains all CPU numerical oracles. Native `mutants` and
local `all` still run the full hardware scope; no source or mutant is excluded.
Cargo-mutants 27.1.0 accepts Cargo target selectors after its `--` separator.
The actual macOS ARM64 CPU baseline passes all nine tests, followed by **79/79
caught mutations**, zero misses, timeouts or unviable mutations. The root library
SHA-256 is unchanged after the in-place run. Complete outputs, diffs and per-case
logs are retained in `p0-ci-evidence/cpu-mutants-complete.tar.gz`.

Routing is qualified with a real setup-negative: `CUBECL_WGPU_MAX_TASKS=0` makes
the unchanged GPU kernel test fail with exit 101 and
`Invalid wgpu configuration: tasks_max must be greater than zero`. The same
setting remains active during the successful CPU baseline and all 79 mutations.
This variable is read by CubeCL wgpu RuntimeOptions, not its CPU runtime. The
mutation build stage still compiles test binaries; only execution selects the
library and both CPU integration targets. This proves CPU routing when GPU setup
is blocked; it does not demonstrate a hosted software adapter or a driver fault.
Exact commands, tool output, restored-source hashes and a live 15 GiB disk guard
receipt are retained. Minimum free space was 108,239,835,136 bytes; the guard did
not trigger.

Local verification covers checksum-pinned installation without Go, exact version
output, host-provisioning refusal, shell syntax, ShellCheck, actionlint and normal
root fast/test commit hooks. A new hosted run is required; these setup repairs
do not turn the failed run into a passing CI result.
