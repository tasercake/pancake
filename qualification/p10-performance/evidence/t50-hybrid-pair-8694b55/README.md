# Exact combined 8694b55 T50 Hybrid diagnostic

All 52 shared generated files, 2,775,174,405 bytes under sealed materialization SHA-256 `317f4b0c9be5837cd7082227bf700ee232e7e995471ee44be8488cbf3511603c`, were reauthenticated after truth reads. Independent TIFF decode confirmed genuine 5998×3998 RGB16, low-bit variation, 50 exact finite identity transforms, symmetric one-pixel crop and all 50 known focal-plane regions in each result. Both byte streams and decoded hash streams matched their corresponding prior `12f2151` outputs exactly. [Previous exact output identities](../t50-12f2151-preflight/pair-001/summary.json)

| --- | ---: | ---: |
| Retained before deletion, TIFF bytes | 78,347,120 | 143,881,378 |
| Mean absolute error versus generated sharp truth, u16 | 979.595 | 843.328 |
| Vertical-second-difference variance ratio in 50 truth regions | 1.1663986–1.1664015 | 0.8667194–0.8918330 |
| Observed process-tree RSS peak, bytes | 6,701,006,848 | 987,627,520 |
| Max RSS observation gap | 26.475 ms | 9.677 ms |
| Observed attributable GPU memory peak, bytes | 4,315,938,816 | 2,518,323,200 |
| Max GPU observation gap | 27.563 ms | 78.153 ms |

Whole-output paired RGB16 MAE was 1792.133 codes, RMS 2207.626 and maximum 19611. Candidate truth MAE was lower and its measured local detail was attenuated. Equal quality remains unresolved. The independent raw-trace reduction reproduced both per-metric cadences and peaks. The reference RSS trace and both GPU traces exceeded the 20 ms cap, so neither paired memory ratio qualifies. The 34 scheduled timing jobs remain separate.

The two output TIFFs were rehashed and removed, totaling 222,228,498 bytes. The shared stage remains with its owner for the other reader. [Cleanup receipt](output-cleanup.json)
