# Linux dependency and guardrail checks

Both unchanged required assurance commands passed against clean source
`11cfcf8aac5921dfa843bfc5777eb4b2643a6ca6` on Linux x86-64. Each command used
two build jobs on CPU 14 and 15. The commands exercised dependency policy and
disposable compiler fixtures without launching Pancake or using a device.

| Command | Result | Wall time |
| --- | --- | ---: |
| `bash scripts/check.sh deps` | Exit 0. Passed 19 policy and unused-dependency tests and 8 vendor contracts. Cargo Deny cleared advisories, bans, and sources. Cargo Machete found no unused dependencies in the core, CLI, or GUI. | 65.67 s |
| `bash scripts/check.sh guardrails` | Exit 0. Rejected all 9 negative fixtures with the required diagnostics and accepted the fully covered, mutation-resistant positive fixture. | 7.46 s |

Cargo resolution was offline. Cargo Deny contacted its configured RustSec
upstream through its normal advisory check. The database remained at
`472a4c9a785030434f15d7da84964d60fbe789ab`. The run used Rust 1.100.0,
nightly-2026-11-15 for the branch fixture, Cargo Deny 0.20.2, Cargo Machete
0.9.2, Cargo LLVM Cov 0.9.1, and Cargo Mutants 27.1.0. This guardrail run
verifies rejection and acceptance on its fixtures. Application coverage and
mutation need their own gate results.

[Dependency output](deps.log), [guardrail output](guardrails.log),
[provenance](provenance.txt), and [file integrity](SHA256SUMS).
