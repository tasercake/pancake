# Weighted CPU control completed before trace validation failed

Under the exclusive stage6 lease, original preparation `4356c59` launched
one exact `e81079d` stock Weighted Average CPU job. The launcher timed it
at **43.383004 s** and reaped exit 0 with no remaining child session. It
then applied mapping operations to `sequential_transforms`, which the
actual completion event emitted as an ordered list. The verifier raised
`TypeError` and the launcher exited 1 at 00:59:20.972 SGT. GDB and the
second app were never started. This attempt contains no phase times,
tracer-overhead estimate, or frozen performance credit.
[Typed outcome](summary.json) · [Original report, completion and error](raw-attempt.tar.gz)

The postflight reverified all 52 staged files and the unchanged
materialization and executable digests. Independent offline TIFF
validation subsequently reproduced the previous exact encoded and decoded
stock output digests at 5998×3998 RGB16. The unchanged application event
reported explicit CPU without fallback, 50 keyed identity maps for the
expected original paths, and 50 ordered sequential identity matrices.
The input seal remains with its owner.

The verifier now checks unique expected source paths, exact keyed-map
order and frame matching, and the ordered sequential list's length and
matrices. The regression uses the untouched completed event in this
archive and exercises malformed map shapes, missing or reordered maps,
wrong source paths, and duplicate requested sources. Five schema tests
passed. Four already-passing trace-accounting tests also passed. Only the
diagnostic consumer changed. [Consumer regression](../t50-weighted-phase-preflight/test-control-schema.py.txt) ·
[Failure before the fix](schema-red.txt) · [Test result after the fix](schema-green.txt)

A later experiment requires its own quiet authorization, report directory
and invocation. Use the corrected scripts from the current
[preparation directory](../t50-weighted-phase-preflight/README.md).
Preserve the original failed attempt and leave its orchestration status
intact. The follow-up tests read events and synthetic malformed cases.
They launched no stock, trace or reference process.
