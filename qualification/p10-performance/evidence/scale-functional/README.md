# Exact Linux functional scale preflight

| Profile | Actual RGB16 output height × width | Registration and quality |
|---|---:|---|
| D150 Hybrid CPU | 1524 × 2126 | All 150 native JPEGs were consumed. All 150 transforms were finite, 62 identity, and max translation norm was 93.46 px. Representative registered comparison against the reference remains open. |
| R50 Hybrid CPU | 3392 × 5088 | All 50 Canon CR3 inputs were consumed. All transforms were identity. Pinned LibRaw visible geometry is 3412 × 5118, revealing a 20 × 30 extent difference. Canon crop and renderer parity remain open. |
| T50 Hybrid physical Vulkan | 3620 × 5626 | All 50 native generated TIFFs were consumed. One stationary frame produced a false 262.28 px translation norm. Output lost 15.14% of the correct 4000 × 6000 stationary canvas. Geometry blocks equal-quality comparison. |

The T50 exception was exactly `focus_006.tif`, with the following actual
reference-to-source matrix. Every other transform was identity.

```text
[ 0.99995726347   0.00029137847  184.614578247 ]
[-0.00029137847   0.99995726347 -186.298477173 ]
```

The independently decoded generated sharp truth and the candidate shared
46 sampled central depth-strip regions once mapped by the reported
symmetric crop assumption. Median absolute error was 832.96 of 65535.
Median vertical second-difference variance ratio was 0.869 and the
minimum was 0.0228. These descriptive values establish actual output
inspection. Matching per-method quality and complete release gates remain open.

The per-workload receipts retain exact commands, hashes, software versions,
physical/CPU identities, geometry and output statistics.
