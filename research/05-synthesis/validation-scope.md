# Validation scope

Research date: 2026-11-15. This file prevents document/metadata checks from being mistaken for experimental validation.

| Check | Meaning | Does not establish |
|---|---|---|
| JSON/CSV parsing and schema checks | Research registries and examples can be read consistently. | Correctness of proposed artifact detectors. |
| Artifact/source ID and local-link checks | Review handoffs and evidence pointers resolve. | That every source claim has been independently reproduced. |
| Snapshot hashes and immutable revisions | The inspected text/metadata can be identified later. | Current binary-download access or rights beyond recorded terms. |
| Dataset API/tree spot checks | Advertised file entries, license metadata, and count mismatches were inspected. | Image decoding, real frame quality, complete focal coverage or verified checksums of image bytes. |
| Code inspection | Specific implementation paths and assumptions were read at pinned revisions. | Runtime quality, GPU parity, precision round trips or performance. |

No focus stacker, metric detector, rendering experiment, human study or benchmark timing was run. No image corpus was downloaded or finally divided into train/validation/test scenes. Proposed numerical pilot sizes and acceptance criteria are design choices awaiting qualification.

The final metadata audit is recorded separately in `research-validation.json` after both parallel reviews finish.
