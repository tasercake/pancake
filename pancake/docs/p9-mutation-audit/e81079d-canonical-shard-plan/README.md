# One complete Linux CPU mutation shard at e81079d

The execution source is `e81079dee4078d7a1149da0f5950aab8c89f8d44`.
The reviewed producer and verifier come from
`edc1d264f7202e82592118b43e756a945787599f`. Since the last complete
listing, the fusion, tiled decode and spectral registration modules
changed. Discover the current shard index, count and membership from a
fresh unfiltered listing when the CPU slot opens. No current roster or
native shard was produced for this plan.

Choose the smallest complete zero-based `k/16` partition of the ordered
full list. Among equal-size partitions, choose the lowest sum of
historically measured package costs, then the lowest index. This rule
chose `11/16` on the 2,642-row historical list, with 165 candidates.
The historical full-shard model estimated 67m 52s using means and
72m 53s using sampled maxima. The ten-row native replay consumed
7.33 GB of target storage at completion. Wall time and peak storage
remain unmeasured for a full shard. The 75-minute hosted limit remains
unqualified. [Read-only controls](artifact-controls.txt).

## Run on an authorized CPU slot

Wait for same-host P9 package and isolated desktop work, the E9
candidate/reference CUDA timing and samplers, and the stock lifecycle
desktop/controller run to reach terminal status and release the host
resources. A terminal failure is a scheduling release when its owner
confirms all processes ended. Keep its original result. Work on another
machine need not delay this local CPU run. Confirm two reserved physical
cores and disk headroom immediately before startup. CPUs 6 and 7
mapped to distinct cores in the planning snapshot. Allow a roughly
75-minute initial reservation with room to extend and measure actual
storage. Keep timed product runs off this host until the shard ends.

Create a fresh isolated Git worktree at the execution source, with new
attempt output and Cargo target directories outside that checkout.
Use the source-pinned Rust 1.100.0 and cargo-mutants 27.1.0. Keep the
historical two-job environment shown below and record the actual
toolchain, compiler cache settings, affinity, kernel, hardware,
free space, launch and completion times, and Cargo/native logs. Use
the registry-only compiler wrapper from the historical local attempt
when available and record if it is absent. Every baseline and runnable
mutant still runs the native tests. Clear inherited GPU overrides and
display state before launch. Use this shell from the fresh checkout's
`pancake` directory. `run` is a fresh absolute attempt path and
`plan_dir` is the absolute path to a separate export of this plan.
`cpus` is the parent-authorized pair, such as `6,7`.

```bash
set -euo pipefail
: "${run:?}" "${plan_dir:?}" "${cpus:?}"
mkdir -- "$run"
want=e81079dee4078d7a1149da0f5950aab8c89f8d44
reviewed=edc1d264f7202e82592118b43e756a945787599f
git rev-parse HEAD > "$run/source-before.txt"
git status --porcelain=v1 --untracked-files=all > "$run/status-before.txt"
test "$(cat "$run/source-before.txt")" = "$want"
test ! -s "$run/status-before.txt"
git show "$reviewed:pancake/scripts/verify_mutation_shards.py" > "$run/reviewed-guard.py"
export PATH="/tmp/pancake-tools/bin:$PATH"
export CARGO_TARGET_DIR="$run/target" TMPDIR="$run/tmp"
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0
export PANCAKE_COMPUTE_CACHE_DIR="$run/compute-cache"
unset DISPLAY WAYLAND_DISPLAY PANCAKE_RUNTIME
mkdir -- "$TMPDIR"
if test -x /tmp/pancake-tools/bin/pancake-rust-cache; then
  export RUSTC_WRAPPER=/tmp/pancake-tools/bin/pancake-rust-cache
else
  unset RUSTC_WRAPPER
fi
test "$(cargo mutants --version)" = "cargo-mutants 27.1.0"
test "$(sha256sum "$(command -v cargo-mutants)" | cut -d ' ' -f 1)" = \
  "f985f265ee3ea3e453aa98b04c52134953911f692f8ce8abf137f3873202a2d0"
test "$(sha256sum Cargo.lock | cut -d ' ' -f 1)" = \
  "f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc"

cargo metadata --locked --format-version 1 --no-deps > "$run/metadata.json"
python3 scripts/assurance_targets.py "$run/metadata.json" cpu > "$run/cpu-targets.txt"
test "$(sha256sum "$run/cpu-targets.txt" | cut -d ' ' -f 1)" = \
  "d1180f38b292c12c077049872a7c8ce68a7517a643fcaecd71b07acdc7878f9e"
tests=(--lib --bins)
while IFS= read -r target; do tests+=(--test "$target"); done < "$run/cpu-targets.txt"
cpu=(taskset --cpu-list "$cpus")
mutate=(cargo mutants --workspace --test-workspace false --cargo-arg=--workspace
  --cap-lints true --all-features --in-place --timeout 60 --build-timeout 180
  --jobserver-tasks 2)
tail_args=(-- --locked "${tests[@]}" -- --include-ignored)
guard=(python3 "$plan_dir/check-local-shard.py")
{
  date --iso-8601=seconds
  uname -srvmo
  lscpu
  "${cpu[@]}" sh -c 'awk "/Cpus_allowed_list/" /proc/self/status'
  rustc -Vv
  cargo -V
  cargo mutants --version
  df -B1 "$run"
  sha256sum Cargo.toml Cargo.lock rust-toolchain.toml scripts/assurance_targets.py \
    "$run/cpu-targets.txt" "$run/reviewed-guard.py" "$(command -v cargo-mutants)"
  printf 'RUSTC_WRAPPER=%s\n' "${RUSTC_WRAPPER:-}"
  printf 'CARGO_BUILD_JOBS=%s\nCARGO_INCREMENTAL=%s\nCARGO_PROFILE_DEV_DEBUG=%s\n' \
    "$CARGO_BUILD_JOBS" "$CARGO_INCREMENTAL" "$CARGO_PROFILE_DEV_DEBUG"
} > "$run/provenance.txt"
"${cpu[@]}" "${mutate[@]}" --list --json "${tail_args[@]}" \
  > "$run/full.json" 2> "$run/full-list.stderr"
"${guard[@]}" plan "$run/reviewed-guard.py" "$run/full.json" > "$run/selection.json"
partition=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["partition"])' \
  "$run/selection.json")
mutate+=(--shard "$partition" --sharding round-robin)
"${cpu[@]}" "${mutate[@]}" --list --json "${tail_args[@]}" \
  > "$run/list.json" 2> "$run/shard-list.stderr"
"${guard[@]}" check-list "$run/reviewed-guard.py" "$run/full.json" "$run"
test "$(git rev-parse HEAD)" = "$want"
test -z "$(git status --porcelain=v1 --untracked-files=all)"
printf '%q ' "${cpu[@]}" "${mutate[@]}" --output "$run" "${tail_args[@]}" > "$run/command.txt"
printf '\n' >> "$run/command.txt"

set +e
"${cpu[@]}" /usr/bin/time -v -o "$run/time.txt" \
  "${mutate[@]}" --output "$run" "${tail_args[@]}" 2>&1 | tee "$run/native.log"
statuses=("${PIPESTATUS[@]}")
set -e
printf '%s\n' "${statuses[0]}" > "$run/native-exit.status"
printf '%s\n' "${statuses[1]}" > "$run/log-writer.status"
git rev-parse HEAD > "$run/source-after.txt"
git status --porcelain=v1 --untracked-files=all > "$run/status-after.txt"
date --iso-8601=seconds > "$run/completed-at.txt"
df -B1 "$run" > "$run/disk-after.txt"
du -sx -B1 "$CARGO_TARGET_DIR" > "$run/target-after.txt"
if test -f "$run/mutants.out/outcomes.json"; then
  cp -- "$run/mutants.out/outcomes.json" "$run/outcomes.json"
fi
test "$(cat "$run/source-after.txt")" = "$want"
test ! -s "$run/status-after.txt"
"${guard[@]}" verify "$run/reviewed-guard.py" "$run/full.json" "$run" > "$run/verdict.json"
```

## Close or restart the attempt

Archive preflight and terminal status, the unfiltered roster and ordered
shard membership hash, native output directory, raw stdout/stderr,
resource observations and verdict. Record disk high-water observations
during the run if available and distinguish those from final size. The
180-second build and 60-second test deadlines apply to each scenario as
configured in the reviewed producer. Keep survivors, timeouts, a failed
baseline, interruptions, producer or log writer errors, and missing
records as failures. Keep compiler-unviable counts distinct, with their
native Build statuses and logs. A clean result requires the exact full
partition, one successful baseline, and a qualifying outcome with
complete phase evidence for every candidate.

If a preflight or execution fails, preserve the raw attempt and any
residual source diff before removing the task-owned worktree and build
products. A restart uses a fresh clean worktree, output path, independent
roster and baseline after recording the cause. Restart all of the shard
after a recoverable machine or producer failure. A product fix requires a
new source receipt and roster. Never reuse partial outcome sets across
attempts or revisions. Credit a successful attempt to its one canonical
partition and exact checkout. The 16-partition gate still requires every
partition at that source and matching assurance provenance.
