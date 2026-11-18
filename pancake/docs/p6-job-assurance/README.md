# Shared job admission and native coverage

The first executor now claims a control ticket once, with the same original
input count as its immutable request. The claim uses the existing lifecycle
mutex before image decode or compute. It leaves pause and cancellation
available to the controlling frontend.

The red runs exercised real source files in temporary directories with Linux
CPU LLVM and default workspace lints. A second `run_controlled` invocation
on an active ticket could publish its own file and prevent the first result
from completing. An incorrect request count reached numerical submission
before failing. After the change, both are rejected before the competing job
submits work. The active job keeps its result and its own export destination.

The same integration target exercises all five methods with textured identity
sources, actual focus metrics, overlapping slabs and in-memory result
metadata. A separate test registers a generated three-pixel shift under all
supported models and reference modes. Another tests first-frame geometry and
constant color with mixed input dimensions, both aligned and unaligned.

| Check | Revision / selection | Outcome |
|---|---|---|
| `cargo +1.100.0 test --locked --test cpu_stack_plan shared_ticket_cannot_admit_a_second_stack` | Existing admission before the fix | Exit 101, competing publication observed. [Log](single-admission-red.log.gz) |
| `cargo +1.100.0 test --locked --test cpu_stack_plan ticket_input_count_must_match_before_processing` | Single admission present, count validation absent | Exit 101, mismatched count reached numerical submission. [Log](request-count-red.log.gz) |
| `cargo +1.100.0 test --locked --lib --test cpu_stack_plan --test cpu_job` | `33a96cf` | 23 passed across the library and both native job targets. [Log](stack-plan-green.log.gz) |
| `bash scripts/check.sh fast` | `33a96cf` | 40 guardrail tests, formatting, pinned source review, strict Clippy and rustdoc pass. [Log](stack-plan-fast.log.gz) |

Host Linux x86-64, kernel `6.8.0-147-generic`, Rust 1.100.0 and CPU LLVM.
The tests used their own temporary encoded RGB16 source files. The earlier
341-case host ledger retains its exact release identity.

## CPU coverage observation

At `c645a70`, `bash scripts/check.sh coverage-cpu` ran 65 passing tests and
returned exit 1 on uncovered source. The stable LLVM report covers 3,404 of
3,961 lines, 320 of 454 functions and 3,445 of 4,312 regions across the CPU
scope. The production job modules cover 817 of 909 lines, 75 of 93 functions
and 742 of 936 regions. The preceding headless job observation was 626 of 895
lines, 58 of 92 functions and 559 of 920 regions. The admission fix added
reviewed source, so the totals differ. Branch coverage is a separate nightly
check. [Coverage report](stack-plan-coverage-cpu.json.gz) and
[command log](stack-plan-coverage-cpu.log.gz).
