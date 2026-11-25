# Batched CPU witness diagnostic

The earlier passive witness scored 108 cell, map, lattice and direction combinations through 108 individual launches and readbacks per eligible D150 edge. This experiment sends the 54 independent records in each direction through one launch and one readback. The records share the existing extent, stride and 64-sample lattice shape. Each lane reads its own two origin coordinates from the metadata tail. It uses the same map coefficient order, bilinear sampler, accumulation sequence and 5-output record. The two direction launches exchange the original device buffers. The host consumes the records in the same order and applies the same cross-fit and spatial-rank decisions.

One direction's explicit temporary payload is 468 bytes of metadata, 1,296 bytes of coefficients and 1,080 bytes of moments. The image thumbnails already reside on the device. The probe checks record count and output length. These payload sizes exclude driver allocation, cached binaries and resident thumbnails.

The test-first control used the unchanged production CubeCL kernel and the serial route. The known signed translation produced the expected supported map and the moment comparison agreed, then the test failed the new dispatch limit with 54 calls per direction. The packed overlay passed both tests. Across each of the three fixtures it returned 540 moment values with exactly the same bits as the individual launches. The signed translation with a photometric gain and offset survived. A horizontal aperture and texture supported on only one side still abstained. The explicit call path made two real dispatches/readbacks for each complete edge.

| Synchronous warm moment dispatch and readback | Samples | Median | Range |
| --- | ---: | ---: | ---: |
| Individual records | 21 | 16.816 ms | 11.685–19.474 ms |
| One packed launch per direction | 21 | 0.591 ms | 0.355–0.626 ms |

The measurement alternated serial-first and packed-first using the same synthetic 96 × 96 pair, queries and Rust executable after both paths warmed. Timed work included per-call metadata, coefficients, output allocation, dispatch, readback and moment parsing. The median within-pair ratio was 0.0310. This development-profile diagnostic used logical CPUs 12 and 13 on a Ryzen 9 7950X3D running Linux 6.8.0-147-generic. It ran without the exclusive benchmark lease. It measures the arithmetic witness in isolation and excludes decode, initial thumbnail upload, proposal estimation and pose decisions.

The synthetic diagnostic isolates the kernel layout from any real scene. A second explicit CPU test below exercises the same layout on all six previously authenticated proxy comparisons.

[Prior real D150 witness](../witness-pilot/README.md) · [Source, lock and runtime identifiers](identities.json) · [Paired observations](synthetic-cost.json) · [Initial failing log](red.log.gz) · [Passing log](green.log.gz)

## Authenticated D150 proxy replay

The existing acquisition tool verified the pinned public ZIP against the frozen manifest and extracted only the same nine JPEGs. The product decoder and luma prefilter prepared the normal 384 × 288 proxy. The test used the archived two directional candidates and identity, with complete overlap for all sampled taps. Across six edges the packed route produced all 3,240 float32 moment values with identical bits to serial. Every archived cross-fit loss, cell result and rank footprint reproduced exactly. The genuine `580→581` step retained inverse-reverse support in eight of nine cells and rank two. The false jump on `654→655` had zero direct wins and one inverse-reverse win, so the witness abstained globally.

Across 42 alternating-order warmed observations, the serial route used 108 launches/readbacks per edge and had a median of 18.632 ms (12.871–20.240 ms). The packed route used two launches/readbacks and had a median of 0.597 ms (0.358–0.638 ms). The median within-pair ratio was 0.0305. A two-job development-profile build and the diagnostic ran on CPUs 12 and 13 as above. The Cargo test took 8.92 s, including preparation and all 42 diagnostic pairs. These observations measure reuse of existing proposals only. Reverse-hypothesis generation cost, whole-stack acceptance and frozen performance/quality remain open. No GPU was used. All nine originals were independently rehashed and removed. No release source was changed.

[Actual results](real-packed.json.gz) · [Bit parity and timing summary](real-summary.json) · [Acquisition](real-acquisition.json.gz) · [Cleanup and exact test identity](real-cleanup.json) · [Passing test](real.log.gz)

## Replay

Use an isolated checkout of `86d4df2d43a52b89eff4b4ba5fad94a3e68161a4` and point `proof` to the absolute path of this evidence directory. Use a new output path for the diagnostic report. The individual-record test is expected to exit 101 at its dispatch count check. Reverse that overlay before applying the second one.

```sh
repo=$(pwd)
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 RAYON_NUM_THREADS=2
gzip -dc "$proof/serial-control-overlay.patch.gz" | git apply
(cd "$repo/pancake" && taskset -c 12,13 cargo test --locked --lib packed_witness_exact_decisions -- --test-threads=1)
gzip -dc "$proof/serial-control-overlay.patch.gz" | git apply --reverse

gzip -dc "$proof/packed-overlay.patch.gz" | git apply
export PANCAKE_PACKED_WITNESS_REPORT="$report"
(cd "$repo/pancake" && taskset -c 12,13 cargo test --locked --lib packed_witness_ -- --test-threads=1)
gzip -dc "$proof/packed-overlay.patch.gz" | git apply --reverse
```

For the real control, first clear either synthetic overlay and make an owned scratch directory. The acquisition option is the existing pinned nine-file recipe, and its verification consumes the public ZIP before deleting it. Reproduce only when authorized to access the frozen originals and the stated CPU allocation.

```sh
motion="$repo/pancake/docs/d150-geometry-audit/motion-admission"
scratch="$repo/artifacts/d150-batched-witness"
mkdir -p "$scratch"
taskset -c 12,13 python3 "$motion/acquire-probe.py.txt" \
  --audit "$motion/audit.json" --output "$scratch/real-input"

gzip -dc "$proof/real-overlay.patch.gz" | git apply
export PANCAKE_WITNESS_ACQUISITION="$scratch/real-input/acquisition.json"
export PANCAKE_WITNESS_DIRECTIONS="$motion/direction-replay.json"
export PANCAKE_WITNESS_OUTPUT="$scratch/real-packed.json"
(cd "$repo/pancake" && taskset -c 12,13 cargo test --locked --lib packed_witness_real_d150_proxy_diagnostic -- --test-threads=1)
gzip -dc "$proof/real-overlay.patch.gz" | git apply --reverse
gzip -dc "$proof/audit_real.py.gz" | python3
```

Rehash the nine extracted originals against the acquisition receipt before removing them.
