# Bounded wgpu polling timing experiment

| Build | Idle P95 | Loaded P95 | Loaded observations over 33 ms |
| --- | ---: | ---: | ---: |
| Stock `c1e2cac` Debian package | 23.747685 ms, pass | 52.322024 ms, fail | 35/200 |
| Experimental `962bbdd` release GUI | 23.697456 ms, pass | 66.942485 ms, fail | 45/200 |

Both collectors and their independent verifiers returned zero, all input hashes were unchanged, and both applications shut down cleanly. Each arm proved real `wgpu-vulkan` compute with a matched PCI device and checked 1,120 presented RGB patches against the original 19 witness PNGs. The largest channel difference was one. The observer produced the same completion ledger bytes in both arms. The frozen reducer and capture code were unchanged, and each scenario rebound only the commit and GUI hash. The full 52-file T50 stage was independently authenticated and the two collectors used the same 50 ordered input frames. Materialization SHA-256 was `cd4942acd3127518340dd7f10a9c68d5768923beb45dfd36614ba7a11bf10eb4`.

## Tail distribution

| Loaded action | Stock P95 | Bounded P95 | Stock / bounded over 33 ms |
| --- | ---: | ---: | ---: |
| Source | 24.14 ms | 24.94 ms | 0 / 0 |
| Result | 40.60 ms | 31.47 ms | 6 / 2 |
| Pan | 52.32 ms | 46.66 ms | 6 / 6 |
| Zoom | 56.64 ms | 82.38 ms | 13 / 20 |
| Comparison | 44.22 ms | 65.57 ms | 10 / 17 |

The maximum loaded observation fell from 207.22 ms to 117.97 ms, while more observations exceeded the gate. The last worker phase sampled before dispatch was Fuse for 145 stock and 143 bounded observations, with 18 and 31 over the gate. In Decode, 17/55 stock and 14/57 bounded observations exceeded it. The phase describes when a sample was dispatched. This single ordered pair did not isolate a code path or hardware cause. The GPU driver was 580.178.04 and the board was at PCI 01:00.0. The matching boundary snapshots contained nine existing ambient GPU clients and 579 MiB in use. The board started the arms at 41 and 48°C, with different graphics and memory clocks. No separate campaign compiler or GPU job was running at launch.

## Reproduction and receipts

- The stock GUI is byte-for-byte the `c1e2cac93ad41c02b328d87bdd5d01e1ce2b46eb` Debian package executable. Its archive SHA-256 is `c6f39324ea4d9a29fb1a4f33253f6d988f44d1f150622bc94b728975a5f96b64`. Its GUI and original lock digests are in `pair-analysis.json`.
- The experimental GUI was built from `962bbdd69b112c88528ea5ed1ea3190e3bf41bbd` with `taskset -c 2,3 nice -n 10 cargo build --offline --locked --release --workspace --bins`. The application-only source is `7a01cf99ec4c30e55048d958f6faebb46b82f485`. The measurement branch applies an external same-version CubeCL WGPU 0.11.4 path override. Its Cargo.lock delta is the two registry source and checksum lines. The upstream Cargo archive matches the stock checksum `413caaf7eaf52cb5922f1e9e766fea404fd1dd549d10f3500e22b61c70b63a10` and the experimental patch SHA-256 is `ad0f57600fbbb10e05ec91d974edfea1660ff4be616612289f4bc36474eda39f`.
- Workspace Clippy passed using the stock effective dependency policy, all targets, all features and warnings denied. The attachment test, five CPU runtime checks and thirteen real stack job tests passed. The exact optimized physical contract also passed on the RTX 3090. It reused the renderer's device and queue for indexed compute, checked a known convolution result, destroyed its own device, and received `DeviceLost` from normal CubeCL sync without an external poll. The process exited cleanly and removed its temporary cache and runtime directory.
- The full raw and verifier archives are retained locally, owner-only, under `artifacts/p10-viewer-queue-trace-preflight/shared-poll-c1e2/`. `raw-archives.sha256` and `capture-receipts.sha256` bind the archives and numerical receipts. The same directory contains the experiment patch, exact binaries, preflight and compiler logs, physical contract, and diagnostic launcher. Both viewer roles released all T50 input reads on capture completion.

## Remaining lock attribution

The prior `8694b55` trace records long waits and frees on a worker thread shortly before the renderer enters native present. Pinned wgpu-core 30.0.2 `Device::maintain` skips its explicit HAL fence wait with `PollType::Poll`, then queries the fence and maintains the queue under `snatchable_lock` read ownership. The Vulkan HAL fence query can still incur driver latency. wgpu fires work-done and device-loss callbacks through polling or submission, so waiting for one of those callbacks alone can stop progress when no future submit occurs. A nonblocking poll with parked periodic progress would need to preserve real callbacks, waiter lifetimes, device loss, and cancellation. This pair provides no evidence that it would resolve the observed tail.

The diagnostic priority is to capture actual Rust renderer submit return, present preparation, and the same device lock's reader and writer acquisition and hold intervals. Queue maintenance, deferred frees, and the separate command-index lock need their own boundaries. That design can distinguish a true writer wait behind the CubeCL poller from delays in the renderer's post-submit path or device resource cleanup.
