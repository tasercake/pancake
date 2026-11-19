# Linux private procfs and external process lineage

The lab check ran at 18:51–18:53 SGT on November 18, 2026, with qualification
source `287828a`, Linux 6.8.0-147-generic, util-linux 2.39.3 and strace 6.8.
An unprivileged systemd user scope contained only the diagnostic. The host
namespace and kernel settings were untouched. No product or GPU ran.

## Private procfs test

The ordinary and namespaced controls ran the same synthetic job. Each had a
session leader, a resident worker, and an additional child that exited with
status 23 before the first RSS observation. The namespaced run used
`unshare --user --map-current-user --pid --fork --mount --mount-proc`. The
collector supervised the synthetic job as PID 1 so the job itself had the
ordinary parent/child relationship and started its own session as PID 2.

In each control, the unchanged qualification collector recorded 32 stable
RSS snapshots inside the controller and 32 from the external host observer.
The observer moved the resident worker to an adjacent task-owned cgroup and
repeated both sets of 32 observations before allowing the job to exit. All
corresponding inner and outer observations showed the same two processes,
confirmed from the host using `NSpid`, the PID namespace inode, and the
kernel session ID. Each control's RSS sum remained exact across the move.

| Observation | Ordinary control | Namespaced control |
|---|---:|---:|
| Visible procfs PIDs in controller | 972 | 3 |
| RSS sum agreed by both observers, before and after move | 24,928,256 B | 24,403,968 B |
| Controller scan median, before / after move | 15.519 / 14.291 ms | 0.404 / 0.382 ms |
| External host scan median, before / after move | 1.066 / 1.066 ms | 1.083 / 1.070 ms |

The controller was under strace in both controls. The external scanner was
untraced. The wall numbers show that the narrower list was cheaper in this
probe, with tracing overhead especially visible in full host scans. They
carry no information about a complete, uninterrupted production job's 20 ms
cadence. [Raw probe and external traces](rss-pid-namespace-probe.json.gz)

## Launch equivalence

The selected primary UID and GID values, zero effective capabilities after
exec, and full environment hash matched. External `/proc` retained the
original supplementary GIDs for the running child in both controls. Inside
the new user namespace, only GID 1000 was mapped. The other 12 supplementary
group IDs appeared as overflow GID 65534. The namespace changed the running
job's user-namespace membership and its view of IDs. The PID namespace also
changed `getpid()` and installed a PID 1 supervisor with distinct reaping and
signal semantics.

Trying PID and mount isolation while retaining the original user namespace
returned `Operation not permitted`. Mounting the private procfs on a separate
task-owned path succeeded and kept `/proc` on the original host view. The
job still saw a nested `getpid()` and remapped groups. A separate proc mount
can narrow the roster, but it does not establish launch equivalence. No
qualified runner mode was added. [Host preflight](rss-pid-namespace-preflight.json) ·
[Linux PID namespace behavior](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html) ·
[Linux user namespace mappings](https://man7.org/linux/man-pages/man7/user_namespaces.7.html)

## General lineage alternative

Both controls used external `strace -ff --decode-pids=pidns`. The ordinary
control demonstrates process-event tracking without changing the job's PID,
mount or user namespaces. The trace recorded the fork, wait, and status-23
exit of the short-lived process missing from every RSS snapshot. In the
namespaced control it specifically translated nested PID 4 to host PID
4117690. Both controls followed the resident worker through its termination
after that worker moved to an adjacent cgroup.

A task-owned parent can use event-only ptrace stops on fork, vfork, clone,
exec and exit to track host PID, thread-group identity and process lifetime
independently of cgroups. The verified trace demonstrates the lineage mechanism.
It does not give resident bytes for intervals between actual RSS observations.
An event-only implementation would still pause processes at lifecycle
boundaries. Its performance equivalence has not been verified, so the trace
serves as audit evidence.

A passive kernel fork and exit stream can avoid those stops if set up before
launch and validated against lost records, PID reuse, thread versus process
identity, session changes, and descendant drain. The host denied even
`perf stat -e task-clock:u` and `perf record -e dummy:u` as this user, with
`perf_event_paranoid=4` and no effective capabilities. No policy was changed.
The existing sampled process-tree RSS remains the required metric and the
prior R50 and D150 results remain pending. [ptrace lifecycle events](https://man7.org/linux/man-pages/man2/ptrace.2.html) ·
[perf task and loss records](https://man7.org/linux/man-pages/man2/perf_event_open.2.html) ·
[Kernel perf access policy](https://docs.kernel.org/admin-guide/perf-security.html)

## Replay

The probe creates two leaves only inside its transient delegated scope. It
drains the children, removes both leaves and temporary ptrace files, and then
exits the scope. The retained gzip file includes the raw commands and traces.

```sh
proof=qualification/p10-performance/evidence/resource-sampling
systemd-run --user --scope --quiet -p Delegate=yes \
  --unit codex-pancake-pid-roster-replay \
  python3 "$proof/rss-pid-namespace-probe.py.txt" > /tmp/pancake-pid-roster-replay.json
```

[Receipt checksums](rss-pid-namespace.sha256)
