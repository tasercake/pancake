# Canonical Miri host and retained native checks

Linux run [44096988927, nightly job](https://github.com/tasercake/focus-stacking/actions/runs/44096988927/job/134086614107),
source `93aaaf1a1f98caa46220fcf2cc7f0828ccab22b7`, compiled successfully but
failed **before the first test entered**:

```text
error: unsupported operation: can't call foreign function `dlopen` on OS `linux`
```

The actual trace is `tiny_xlib::XLIB::__CTOR_FUNCTION` → initialization of XLIB
→ `Xlib::load` → `libloading` → `dlopen`. Tiny-xlib 0.2.5 eagerly loads Xlib and
captures its default error hook before main/test entry. This is a transitive
window-system constructor, not a failed shape/budget/readback assertion. The
Miri diagnostic explicitly says unsupported operation does not indicate a program
bug. The complete failed job log is retained in `p0-miri-host-evidence/`.
No dependency feature, constructor, vendor source or test is disabled to hide it.

## Required host contract

| Check | Required execution |
| --- | --- |
| Five pure-host library Miri tests | Actual macOS CI `nightly` job, strict provenance and symbolic alignment. Local macOS `all` retains this real run. |
| Linux randomized layouts | Both seeds 1 and 42, library and every classified CPU integration target, in Linux assurance job. |
| Linux branch coverage | Real CPU/source-universe branch gates, including doctest accumulation, in Linux assurance job. |
| macOS layouts/branches | Existing required nightly CPU profiles remain on macOS as well. |
| Native GPU / sanitizers | Local native test/coverage/mutation/layout/branch gates remain unchanged. Linux ASan/TSan remain required and execute the owned CPU paths. |

The required aggregate CI job still depends on `nightly`; moving its host does
not make Miri optional. CI must actually execute and pass it for the same source
revision. `miri` is a real tool command, not a substitute result. `nightly` and
`nightly-cpu` compose actual Miri with `layouts` or `layouts-cpu` respectively.

On Linux, `all` runs the complete applicable local native gate list and prints
`Miri NOT RUN on Linux; required macOS CI job: nightly`. Its local result cannot
qualify the complete cross-platform matrix. It does not print or record Linux
Miri PASS. Direct `miri`/`nightly` on a noncanonical host fails with an explicit
host-contract error. The original direct Cargo command remains reproducible for
diagnosis; no `cfg(miri)`, ignored test, suppression or source exclusion is added.

`assurance-platforms.json` pins the complete current `src/` Rust inventory,
Cargo manifest and lockfile. The guard checks these hashes and the exact nightly
tool pin before `fast`, local `all` planning or Miri. Changed/new production
source, `build.rs`, dependency resolution or tool pin fails closed until reviewed.
Four regressions exercise explicit Linux obligations, actual macOS selection,
source changes/additions, changed dependencies/tool pins and unknown hosts.

The current Miri tests call OS-neutral checked shape/byte budgets and typed
readback validation. Both available targets are 64-bit little-endian hosts.
They do not invoke native LLVM JIT, GPU execution, UI compilation or OS services.
New OS-specific authored code reopens this boundary: a macOS run must not be
claimed to test a new Linux-only branch. Re-review and qualify an appropriate
boundary/tool lane before that phase exits. A routine source hash update records
review; it does not grant an unsafe exception or prove arbitrary new code portable.

This tool limitation is distinct from Rowan's separately retained ARC Miri
undefined-behavior report. Neither relocating this host-only lane nor native
ASan/TSan passes resolves that UI runtime trust question. No claim certifies
all transitive unsafe, JIT machine code or drivers.

## Local verification and provenance

The production source and lockfile remain byte-identical. Policy regressions
fail before implementation and pass after it. Actual local macOS Miri results,
commands, source bindings, host plans and the original Linux failure log are
retained under `p0-miri-host-evidence/`; a new hosted workflow run remains required.
Normal fast/test commit checks remain mandatory. Parent integration performs the
full native aggregate; no kernel or full hardware matrix change is introduced.
