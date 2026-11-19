# Native worker outcome generation guard

The canonical mutation at `src/job/worker.rs:153:28` survived on
`0a9ae85b88519a795e3b6ba90860c3b3e664bcaa`. It replaced the match guard
`outcome.generation == generation` in `StackWorker::take_result` with
`true`. The [raw records](raw-records.tar.gz) preserve the saved mutation
and its original outcome row.

The regression uses the actual capacity-one outcome channel and the
production receipt method. A stale caller leaves the queued message
alone. The current caller receives the stale-message diagnostic, then
can retrieve its own outcome. The two generations carry distinct
payloads so the test observes which result the method transfers.

The exact saved patch applied with zero fuzz. The native test failed
with exit 101 when the current caller accepted the older payload. After
the exact reverse patch, all 24 stock library tests passed. The
reviewed source and dependency closure check passed. Scoped Clippy
passed with `-D warnings` and the generated reviewed dependency
configuration. An initial Clippy invocation omitted that configuration
and reported known duplicate-version diagnostics. Both logs are retained.

The test source checkpoint is `df6a317f7d8bdf1739f5530d17339ae663e3aa49`.
It adds one test-only module and refreshes the reviewed source hash.
The production worker bytes match the baseline. The native replay used
Rust 1.100.0, two direct offline Cargo jobs on CPUs 2 and 3, and the
canonical mutation build's `--cap-lints=warn` for the red and green test
replays. The Clippy check used no lint cap. The owned Cargo target was
cleaned after verification. [Receipt](receipt.json) and
[checksums](SHA256SUMS).
