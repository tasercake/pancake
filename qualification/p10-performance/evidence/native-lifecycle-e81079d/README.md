# Physical lifecycle latency on the e81079d Debian release

The exact packaged GUI passed all 120 native lifecycle requests on the
physical RTX 3090. The frozen limit was 250 ms for each acknowledgement.
Each trial began during observed Decode, Register, or Fuse work.

| Workload | Operation | Samples | P50 (ms) | P95 (ms) | Max (ms) |
| --- | --- | ---: | ---: | ---: | ---: |
| Original 64-path fixture | Pause | 30 | 147.089 | 148.562 | 148.740 |
| Original 64-path fixture | Cancel | 30 | 147.198 | 148.199 | 148.703 |
| Real D150 | Pause | 30 | 140.667 | 142.239 | 211.515 |
| Real D150 | Cancel | 30 | 141.050 | 211.970 | 212.889 |
| Combined | Pause | 60 | 140.950 | 148.418 | 211.515 |
| Combined | Cancel | 60 | 146.818 | 210.751 | 212.889 |

All pauses reached worker-confirmed Paused. All cancellations settled at Idle
with a same-generation worker acknowledgement. One D150 request showed
Cancelling at 69.296 ms and settled at 138.338 ms. The other 59 exposed the
actual acknowledgement through the durable Idle marker. In both sessions a
new job then completed on physical Vulkan on the same device, and the GUI
shut down cleanly. [Independent verification](summary.json).

The frozen reducer passed pause, cancel, and native lifecycle workload coverage
for `D150` and `lifecycle`. Its overall exit was 1 because both viewer conditions
had zero observations in these captures. Viewer qualification remains open.
The prior exact stock `fb9c399` loaded viewer P95 was 42.208 ms against 33 ms.
[Frozen reduction](frozen-reduction.json) and
[prior viewer evidence](../../../../pancake/docs/p10-shared-poll-evidence/fb9c399/README.md).

## Package, inputs, and native capture

- Source `e81079dee4078d7a1149da0f5950aab8c89f8d44`. Package SHA-256
  `8ed6e70d7552140c0257520a07ac2b9fc5f97ff82a5fb3126864df821df4507e`.
  GUI SHA-256 `770ad910a8ac1f29a519504fd06dcd1051e593965813ce94665c986b57d16310`.
  The Cargo lock, descriptor, release build log, extracted CLI, and
  verified P9 package identities are in the [build receipt](build-receipt.json).
- The original 123 steps for each workload selected physical GPU processing,
  exercised the existing native controls, and required a subsequent completed job.
  Only the expected executable revision and digest were rebound. The
  adjacent runner hashes bind the same collector, observer, protocol, and
  reducer used in the d6359e1 campaign. The frozen target SHA-256 is
  `58bb68533311c48696a3a844d80eef2f37a3c1f39649305948c838b7002663dc`.
  [Frozen runner hashes](runner.sha256) and [original scenario and commands](../native-lifecycle-d6359e1/README.md).
- The first workload used 64 original paths, hardlinked to the same
  1024 × 1536 RGB8 PNG with SHA-256
  `d9a1425f2bb6492d06009521cd2ac30b0e754fe0da5b7fa189803f17bb7cd6e8`.
  The second used all 150 ordered 2304 × 1728 JPEG originals totaling
  52,911,148 bytes. The Figshare record declares CC BY 4.0. The 53,459,291-byte
  ZIP had SHA-256 `311ca6aab7d3bff03c714fd2d81c741f3847de1d4d3f2f9cfa767b6eb62cf56a`.
  The reacquisition receipt reproduced SHA-256
  `0fc4a4f4eb80a64e0faba1b50e2df57288941f6d8490f57670d34dd9cdd1e05e`.
  Both collectors rehashed all selected inputs on exit.
  [Source dataset](https://doi.org/10.6084/m9.figshare.14707077.v1).
- The 2026-11-19 quiet windows were 00:08:15–00:08:53 SGT for the fixture and
  00:17:12–00:18:07 SGT for D150. The host used a Ryzen 9 7950X3D, Ubuntu
  24.04.5 LTS, and a GeForce RTX 3090 with driver 580.178.04 at PCI
  `0000:01:00.0`. Both sessions used private 1440 × 1000 Xvfb, the
  packaged GUI, and the same UUID and driver. Visible-launch physical
  residency was 91,922,432 bytes and 87,728,128 bytes respectively.
- The exact launchers were `run-stock-lifecycle-corrected.sh` and
  `run-stock-lifecycle-real.sh` under the local artifact root
  `artifacts/p10-stock-lifecycle-e81079d/`. Each collector used an outer
  180-second limit with a 20-second kill grace. The unchanged
  `p10.py latency` command reduced the two successful
  `native-latency.json` ledgers. The archived historical independent
  verifier exited 0. [Retained command and capture hashes](retained-captures.sha256).

## Setup failure and retention

The first fixture attempt ended before the application launched.
Its private runtime path omitted the `artifacts/` component required by
the staged Openbox wrapper. The zero-observation failure is preserved.
The corrected launcher placed the same capture under its owned
`artifacts/` directory. [Invalid setup receipt](first-setup.json).

The generated fixture, private profiles, runtime, and caches were removed
after capture. The three original capture directories and exact scripts are
held in the owner-only local artifact directory named above. The D150 media
stage remains privately owned for an approved follow-on reader. It is outside
the retained evidence and must be rehashed before removal after that reader
releases it.
