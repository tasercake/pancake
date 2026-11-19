# Nonblocking shared wgpu polling experiment

The nonblocking polling experiment missed the frozen 33 ms loaded viewer target in a complete matched uninstrumented pair. It stays on its local experimental branch. Both builds completed the exact original 200 idle and 200 loaded actions, with the same TIFF decoding and fusion code, real physical Vulkan work, and the unchanged native collector and numerical reducer.

| Build | Idle P95 | Loaded P95 | Loaded observations over 33 ms |
| --- | ---: | ---: | ---: |
| Stock `fb9c399` Debian package | 23.192439 ms, pass | 42.208036 ms, fail | 14/200 |
| Experimental `71ec40d` optimized GUI | 23.794694 ms, pass | 68.006819 ms, fail | 34/200 |

The only application differences were the shared polling attachment, renderer entry, and the targeted contract test. Stock and candidate used the exact same 50 ordered focus files from the authenticated 52-file T50 materialization, SHA-256 `dd20ddfddf96aecccbf08e27d3d240a848efdfce927577ead0cf9c96261f78a4`. Both collectors rehashed the inputs after capture. Every arm produced one genuine same-PCI `wgpu-vulkan` completion. The observer completion ledgers matched byte for byte. The independent verifier decoded 1,120 actual RGB patches per arm against the original 19 witnesses and saw a maximum channel error of one. Both collectors and independent verification commands exited zero. Both frozen reducers exited one for the measured loaded misses.

## Changed late observations

| Loaded action | Stock P95 | Nonblocking P95 | Stock / nonblocking over 33 ms |
| --- | ---: | ---: | ---: |
| Source | 22.79 ms | 25.09 ms | 0 / 0 |
| Result | 23.87 ms | 27.14 ms | 0 / 2 |
| Pan | 42.40 ms | 33.85 ms | 4 / 4 |
| Zoom | 48.00 ms | 83.42 ms | 7 / 18 |
| Comparison | 34.55 ms | 44.01 ms | 3 / 10 |

The last observed predispatch worker state was Fuse for 159 stock and 151 experimental events, with seven and 26 late respectively. Decode contributed seven late observations from 41 stock events and eight from 49 experimental events. The largest loaded observation was 49.90 ms on stock and 100.39 ms on the experiment. A single ordered capture pair supplies these observed differences. The board began the stock and candidate arms at 42 and 47°C and in P8 and P5 respectively. The prelaunch host census found no campaign compiler, fixture generator, app or profiler activity, no queued NVMe work, and 0% GPU utilization at the measured boundary.

The prior instrumented scope directly established the polling read and renderer write lock contention for nine late events. This experiment bypasses that poller's explicit fence wait and keeps real callback and loss handling, then parks between active polls. The ordinary wgpu poll still queries the device and maintains its queue under the same read lock. Other waits, cleanup, application scheduling, and presentation remain possible contributors. The changed tail establishes no uniquely identified non-device-lock bottleneck. The prior conservative same-clock analysis retained seven ambiguous late frames.

## Build and hardware evidence

The stock `fb9c399a96658331cb76b843a1e565b503e94fdd` package archive SHA-256 is `b8f707ca5812ce2137b49feee2888b50de53825e160aa9ccb0e30f55b02a62de`. The experiment ref `71ec40d36b972b447cfbe6e680c978a3ef41ded5` adds a local same-version CubeCL WGPU 0.11.4 override with patch SHA-256 `34fcccd5175fe2052e24c58165965934848fdb6d3b25dd284ab900948111cb70`. Exact GUI and lock hashes for both arms are in `pair-analysis.json`. The optimized candidate and GPU contract were built together with `taskset -c 2,3 nice -n 10 env -u RUSTC_WRAPPER -u RUSTC_WORKSPACE_WRAPPER cargo build --offline --locked --release -p pancake -p pancake-gui --bin pancake-gui --test gpu_shared_poll_contract --message-format=json-render-diagnostics` on Rust 1.100.0.

The targeted CPU probe reproduced the original native Wait selection and passed all three checks after replacing it with nonblocking Poll, including the real park/unpark handoff and live token drainage. The attachment, five CPU runtime and thirteen CPU stack job checks were green on the initial experiment ref `b95e34d`. The strict workspace Clippy command completed successfully. The exact rebased optimized `71ec40d` physical contract ELF SHA-256 `9b2eb017727eea67d6b1a0b90850a0b9bb715cc61230be803f62f2089bb6d262` ran 1/1 in 0.52 s on the RTX 3090 at PCI 01:00.0 with driver 580.178.04. Its own shared and indexed routes retained the same queue and device, returned the expected convolution pixels, and reported `DeviceLost` through normal CubeCL sync after destroying the isolated test's device. It exited cleanly and removed its cache and runtime directory. Physical lifecycle was outside the completed scope after this candidate missed the viewer gate.

## Evidence retention

The one ordered pair ran from 23:19:55 to 23:28:01 +08:00 on 18 November 2026. The full owner-only captures, exact executable, compile artifacts, native proof and source patch are retained under `artifacts/p10-viewer-poll-park-fb9c399/`. The verified capture archives contain 2,034 allowlisted, byte-checked files each. Their only images are the 16×16 calibrated witnesses and presented RGB patches. The archives exclude inherited environment, credential fields, ambient process dumps, profile caches, full media and temporary duplicates. `capture-archives.sha256` binds the archives, `raw-receipts.sha256` binds the native report, verifier and completion ledger, and `archive-verification.json` records the verification. Both input roles released the borrowed T50 files after the final postflight rehashes.
