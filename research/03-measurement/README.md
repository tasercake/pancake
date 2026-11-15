# Measurement design

Status: research proposal. No algorithm, detector, renderer or human study has been run.

- [benchmark-design.md](benchmark-design.md): target/evidence conventions, orthogonal pipeline and corpus conditions, score/coverage policy and first pilot.
- [metric-registry.json](metric-registry.json): 27 candidate measurements covering A01–A20, with definitions, units, applicability, abstention and attacks.
- [validation-protocol.md](validation-protocol.md): independent qualification, defect ladders, metric gaming, human protocol, statistics and release gates.
- [synthetic-ground-truth.md](synthetic-ground-truth.md): analytic versus estimated truth, optics/material contracts and generator validation.
- [run-output.schema.json](run-output.schema.json): proposed run/score record; references scene/split manifests owned by dataset review.

[sources.md](sources.md) records primary evidence and access limits. [handoff.md](handoff.md) records decisions and remaining work. Threshold configurations, eligible/scored masks, score maps and actual run records do not exist yet; schema validation cannot substitute for metric qualification.

[run-output.example.json](run-output.example.json) is explicitly fictional and joins the dataset review's fictional scene/split IDs. Its `split_role` equals `scene.split.split_role` and the corresponding split group's role. Example hashes identify illustrative manifest files only; no images or run exist. Both schemas use `leakage_group_id`. Actual-run intake must reject example records and verify every hash/file/role join semantically.
