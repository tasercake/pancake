# Hybrid coherence on exact held-out D150 pixels

The packaged control is the locked `287828aac07ffad2252785b73a52a59fc07b3409` release. The Debian archive matched its independent P9 verification receipt, including all 69,551,694 bytes and SHA256 `34a63edea55c411a6082924c2fbe2c50094eea98753b1f75378459da78e4db99`. The read-only CLI extraction matched SHA256 `874c8683623131b5619858a9cba61cb6219bc2eae6d6bdb30ed1bc5d8634ce8e`. [Package control receipt](packaged-control.json)

The experiment at `a51e697` applies the previously tested coherent fine-level changes to that exact package source. The only product files that differ from the control are `fusion/advanced/levels.rs` and one visibility line for the existing luma helper in `fusion/advanced/weights.rs`. The candidate was built offline on the pinned Rust 1.100.0 locked graph using two jobs and disabled incremental compilation. The resulting CLI SHA256 is `4bfe990f106ae7a6fc366ae3e87451cc74b3f2886d1bff2cd4c74c1c609c95ec`. [Build log](optical-build.log)

## Reproduce the controlled optical inputs

The earlier [raw-only held-out plan](../d150-band-focus/heldout-plan.json) fixes both 96×96 witness regions. Their corners are (670,675) and (1505,1045) in first-frame coordinates. Original indexes 36 through 75 are one original D150 slab and include both corroborated source details. Using the same independent map artifact SHA `b5b862bf0ffa3b9a630373082f606ad8a8912d8d71e642d1b667a7928f58133a`, the one-core preparation inputs reproduced all 40 previously registered TIFF hashes exactly. We authenticated all 150 borrowed original JPEGs before and after preparation and returned the read lease. The shared resampling is Lanczos4 with replicated boundaries and RGB16 TIFF. Both products receive identical samples from identical TIFF bytes.

Use the frozen reference Python environment for the comparison script and acquire the authenticated original frames with the sealed D150 recipe. Decompress the independent registration artifact at `pancake/docs/d150-geometry-audit/subpixel-diagnostic/native-local-track.json.gz`. Run `verify_optical.py.txt prepare ORIGINALS MAP_JSON NEW_OUTPUT_DIRECTORY` from the repo root to recreate the exact input digests. Then run `taskset -c 0-3 python3 verify_optical.py.txt compare CONTROL_CLI CANDIDATE_CLI OUTPUT_DIRECTORY CANDIDATE_SHA256`. The replay uses four logical CPU cores and existing real Hybrid job tools. It disables internal alignment and crop and uses kernel 6, levels 8, slab 40 overlap 4 and RGB16. The framed scope is a one-slab optical diagnostic. It supplies no timing or memory gate credit.

## Observed optical comparison

| Native held-out region | Packaged signed projection | Coherence signed projection | Historical pinned signed projection | Packaged unsigned P90 | Coherence unsigned P90 |
| --- | ---: | ---: | ---: | ---: | ---: |
| (670,675) | 0.308888 | 0.341482 | 0.392158 | 0.046601 | 0.041979 |
| (1505,1045) | 0.237393 | 0.247185 | 0.298540 | 0.052345 | 0.045544 |

The earlier live pinned reference used the exact same registered pixels. The candidate raises the projection on the real signed phase by 10.6% and 4.1%. Signed phase correlation rises from 0.182080 to 0.226110 and from 0.138480 to 0.170086. The unsigned luma Laplacian P90 falls 9.9% and 13.0%. Signed detail reaches 87.1% and 82.8% of the pinned reference in these regions. Visual review showed less fine clutter with the major ridges preserved in both 96×96 patches. These results show a mix of better source coherence and lower overall contrast, with the full-image quality and frozen targets unresolved. [Native comparison](comparison.json)

The archived octave method also reproduces the packaged control's earlier signed values. The first region gains at 0.125–0.25 cycles/pixel from 0.4040 to 0.4240, with a slight decline at 0.25–0.5 from 0.2181 to 0.2142. The second region gains in the highest octave from 0.1136 to 0.1432 and falls at 0.0625–0.125 from 0.1845 to 0.1384 and at 0.125–0.25 from 0.1683 to 0.1511. The candidate remains isolated pending broader optical and runtime verification. [Octave comparison](octaves.json)

## Cleanup

The original JPEG lease was read-only and returned before the actual comparison. After recording all results, the 40 registered TIFFs, two outputs, two phase witnesses, visual review panel and temporary package extraction were removed. The original package remains in its P9 staging directory. The exact candidate executable is retained in the local performance artifacts. [Cleanup and executable hash](cleanup.json)
