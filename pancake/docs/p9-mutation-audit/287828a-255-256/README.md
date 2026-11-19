# Native CPU mutation subpartition 255/256

Exact clean source `287828aac07ffad2252785b73a52a59fc07b3409` finished a
native cargo-mutants 27.1.0 run with **six caught, three missed and one
timeout**. The process returned exit 3. The unmutated baseline passed all
137 tests across 19 targets. The full workspace, all features, libraries,
binaries and the 16 classified CPU integration targets ran with the original
test selectors. Each mutant retained the 180-second build and 60-second test
deadline. The checked-out source was clean before and after the run.

| Outcome | Actual mutation |
| --- | --- |
| Missed | Replaced division by multiplication in the panoramic thumbnail pixel cap. |
| Missed | Returned success from `desktop_menu::reveal` before checking or revealing the source. |
| Missed | Returned default GTK widgets from the native file dialog builder. |
| Timeout | Forced the registration read helper to return an empty successful vector. |

The read mutant timed out in `cpu_registration_contract` after 60 seconds.
The existing warp loop advances by the accumulated output length, so an
empty successful chunk prevents progress. Production readback rejects a
sample count different from the requested size. The forced empty success
was introduced by cargo-mutants, and the raw evidence establishes no
observed empty read from a native backend.

The run used Rust 1.100.0 and two Cargo jobs pinned to CPUs 20 and 21 on the
AMD Ryzen 9 7950X3D host. It took 10m 02.17s. The cold baseline built in
203.98s and ran in 11.47s. The task-owned target occupied 7,326,187,520
allocated bytes after the run. The core mutations required 29–33 seconds
of build time each. The two GUI builds took 21.31s and 2.37s. Test phases
varied from an early 0.81-second failure to the 60-second timeout. This
local sample can guide shard sizing. The full local and hosted gates
remain open. [Replay](replay-255-of-256.sh.txt).

The isolated matrix-prototype verifier at `8a238913` passed 13 artifact
controls. The local receipt checker reproduced both genuine historical CLI
shard outcomes and rejected modified CPU selectors and missing native
rows in separate controls. [Prototype log](prototype-tests.log) ·
[Checker controls](native-scope-checks.json).
