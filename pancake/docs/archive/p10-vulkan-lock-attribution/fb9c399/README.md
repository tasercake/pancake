# Physical Vulkan shared-lock attribution

The instrumented `fb9c399` product located a same-device wgpu lock wait in nine uniquely attributable loaded viewer observations. The renderer's snatch write acquisition took 36.434–182.437 ms. The named CubeCL polling thread held the exact same snatch lock for 36.429–182.416 ms of those intervals and spent 32.829–179.305 ms of them in the HAL fence wait. The renderer acquired its write guard about 4–21 µs after the worker's observed release bracket. This explains the dominant measured host boundary in those nine observations. Seven other late observations intersected two plausible present intervals under the conservative rule and remain unresolved.

| Original step | Input | Dispatch to correct XWD | Write acquisition | Overlap with confirmed read hold |
| --- | --- | ---: | ---: | ---: |
| 408 | Pan | 201.829 ms | 182.437 ms | 182.416 ms |
| 470 | Zoom | 96.483 ms | 75.474 ms | 75.454 ms |
| 508 | Comparison | 94.628 ms | 73.646 ms | 73.639 ms |
| 592 | Comparison | 75.310 ms | 52.367 ms | 52.360 ms |
| 568 | Comparison | 70.048 ms | 46.841 ms | 46.834 ms |
| 520 | Comparison | 65.442 ms | 44.689 ms | 44.674 ms |
| 556 | Comparison | 60.183 ms | 39.993 ms | 39.974 ms |
| 484 | Comparison | 59.555 ms | 38.671 ms | 38.663 ms |
| 436 | Comparison | 58.002 ms | 36.434 ms | 36.429 ms |

These renderer threads entered Rust present 1.388–2.235 ms after dispatch. Wgpu present preparation took at most 0.022 ms and the separate command-index lock did not account for the delay. The subsequent driver-facing present spans were 16.815–19.491 ms. The recorder captured both sides of actual acquisition and release. The overlap uses the interval between confirmed acquisition and the pre-release timestamp, excluding any post-unlock scheduling delay. The parsed binding recorded the original typed wgpu device index and epoch on the same lock instance.

## Exact run

One diagnostic captured 200 idle and 80 loaded viewer observations from 22:39:16 to 22:42:22 +08:00 on 18 November 2026 on the RTX 3090, PCI 01:00.0, driver 580.178.04. The independent verifier decoded the 784 retained actual RGB patches against the original 19 witnesses. The maximum channel error was one. The worker returned one exact physical `wgpu-vulkan` completion and the observer authenticated the 50 ordered inputs before and after. The shared 52-file T50 materialization SHA-256 was `71c81abc2af03edaf43373ee3957757dfd9b692a00af84e5e38381ceba3a92d1`. The application PID exited naturally, Nsight flushed and exited zero, and the recorder wrote 28,193 spans with zero overflow, incomplete records or missing TIDs.

The instrumented loaded P95 was 70.047534 ms, with 16/80 above 33 ms. The idle P95 was 23.947744 ms and 1/200 was above 33 ms. These distributions describe the diagnostic. They carry zero frozen latency credit. The last independently observed predispatch worker stage was Fuse for 60 loaded observations and Decode for 20. Timing attribution uses the actual lock and present spans, independent of whether a sample straddled a worker phase.

## Clock and API proof

The primary join uses Python's Linux monotonic dispatch and correct-XWD observations and Rustix `CLOCK_MONOTONIC` spans. It considers a host present only if its actual interval overlaps the witnessed interval on the real rendering TID. Exactly one present is required. The strict UTC-converted Nsight join is retained as an inconclusive result. All 2,722 same-thread native Vulkan presents agree in ordinal order with 2,722 Rust HAL present spans, and every Rust span is at least as long as the paired native call. The median extra duration is 4.516 µs. The implied relative clock centers vary across a 0.527 ms range, so no single exact UTC offset encloses all pairs. The Vulkan trace independently records 132 native semaphore waits on the verified longer-waiting CubeCL poller, with a maximum of 201.756 ms, and four short waits on the second named poller. These are host API observations.

## Reproduction and evidence

- Stock base `fb9c399a96658331cb76b843a1e565b503e94fdd`, diagnostic branch `a87e34fb38d50a0786dc5259702f4eb80e48e4ca`, diagnostic Cargo.lock SHA-256 `2b6b05105e6d98050ac62d73e81fd10e277edd3fe072167cae4104a485a58cdc`, GUI SHA-256 `8386c1da28d4907170939dca9d0aecfcb0cc23dada225c8eae1c2fd0fe73f817`. The local overlays preserve stock indefinite polling and use identical upstream package versions. Wgpu-core patch SHA-256 is `36aacbe5e147cab9d9bb7e005a4afa79416bd0b0fe77b4e635d5c8714f37af59` and CubeCL role-name patch SHA-256 is `201803701cea78c86c2073264d20adab5e5069ab51b5ff5c093853c02fe93faa`.
- Build command `taskset -c 2,3 nice -n 10 cargo build --offline --locked --release -p pancake-gui --bin pancake-gui` on Rust 1.100.0. Locked workspace all-target checks and both focused CPU recorder tests passed. `witness.sha256` binds the independent selector and original witness hashes to the exact binary. `run-lock-trace.sh` directly profiles and closes the private GUI, with a 240-second Nsight cap and a 280-second wrapper cap.
- The compiled GUI, exact executable provenance, inputs, observer and owner-only raw captures live under `artifacts/p10-viewer-lock-trace-fb9c399/`. `raw-receipts.sha256` binds the original native diagnostics and typed lock capture. The redacted direct monotonic result and independent witness verification are retained here. The 50 input files were released to the stage owner after the completed read and postflight rehash.
