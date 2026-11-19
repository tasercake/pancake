# Physical lifecycle latency on the d6359e1 Debian release

The exact packaged Pancake GUI passed all 120 frozen native pause and cancel
requests on the physical RTX 3090. Every acknowledgement met 250 ms. The two
workloads each supplied 30 pauses and 30 cancellations. Each timed request
started during observed numerical processing. [Exact verification](summary.json).

| Workload | Operation | Samples | P50 (ms) | P95 (ms) | Max (ms) |
|---|---|---:|---:|---:|---:|
| Original 64-path fixture | Pause | 30 | 139.580 | 141.270 | 141.325 |
| Original 64-path fixture | Cancel | 30 | 70.446 | 141.503 | 142.509 |
| Real D150 | Pause | 30 | 146.991 | 149.750 | 150.773 |
| Real D150 | Cancel | 30 | 147.253 | 149.915 | 151.283 |
| Combined | Pause | 60 | 143.568 | 149.033 | 150.773 |
| Combined | Cancel | 60 | 139.935 | 149.233 | 151.283 |

Every pause reached worker-confirmed Paused. Two fixture cancellations exposed
Cancelling before reusable Idle, which arrived at 139.871 ms and 139.039 ms.
The remaining 58 cancellations exposed acknowledgement and settled Idle
through the same generation's durable worker marker. The reducer preserves
acknowledgement and settled time separately. Both collectors exited 0.
The unchanged combined latency reducer exited 1 because the viewer
conditions had zero observations. Lifecycle operations and native lifecycle
workload coverage both passed. [Frozen reduction](frozen-reduction.json).

## Package and execution identity

- The unmodified GUI was extracted from the sealed Linux `.deb` at
  `d6359e129737f1a9077fb4abc2a8f783fa1f7b38`. The archive SHA-256 was
  `e6b4d83e8100d04d1692fc8d2f0e276e249fa5a1d79e6e8f623801632c860758`.
  The GUI SHA-256 was
  `eea214ed6c57477480b0f5897a12ae8b9c05a9595730dbfcedabfd15650f6f50`.
  The package and P9 physical receipts agreed on bytes, source, Cargo lock,
  and descriptor. The collector independently checked the exact executable
  against its owned checkout and release receipt. [Build receipt](build-receipt.json).
