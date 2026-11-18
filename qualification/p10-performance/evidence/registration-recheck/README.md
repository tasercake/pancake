# P5 repairs at scale

On 2026-11-18, locked release `5e102447ad95c641eb773d9fdf94f2bc3757b81d`
was verified on Linux x86-64. Its CLI SHA-256 was
`3ac6abaecc1bbd5743efd2abc6879a43d4b17294514b7bc209a05227d613ccbb`
and the Cargo lock SHA-256 was
`012cda6d161124eef8ecc3c5c6a4fed8db881e5f7606f4b9d7aee5f766f1a879`.
Both preflights used the unchanged workloads and frozen profile flags.
There are no elapsed target measurements in these receipts.

## Complete generated T50 on physical Vulkan

The RTX 3090, PCI `0000:01:00.0`, driver 580.178.04, completed the Hybrid
profile in CubeCL wgpu-Vulkan on all 50 exact native RGB16 TIFFs. All 50
reference-to-source transforms were exact identity. The output was genuine
3998 × 5998 RGB16 with the expected symmetric one-pixel interpolation margin.
The previous [false motion and 15.14% crop](../scale-functional/README.md)
did not recur. The TIFF SHA-256 was
`c3cb518761fb620a91012bb6e12047a494c93af988237c474ba9076b13e93fd7`.

Independent render truth was decoded from the exact frozen sharp-truth TIFF.
The centered overlap covered every output pixel and all 71,940,012 channel
samples. Its absolute error mean was 843.33, RMS 1093.21, and max 17902 in
uint16 code units. All 50 known depth regions were retained. Their median
absolute error was 829.91. The vertical second-difference variance ratio
relative to sharp truth ranged from 0.8667 to 0.8918, median 0.8691.
The minimum rose from 0.0228 in the initial geometry-defective diagnostic.
These values describe the real decoded candidate. A matched reference
performance and image-quality campaign remains pending.

All 52 generated source and truth TIFFs were rehashed and removed under the
second materialization’s ownership receipt. The candidate TIFF was removed
after inspection.

## Paired real E9 output inspection on CPU

Across the full like-shaped output pair, mean absolute difference was
2823.78, RMS 3481.49, and max 19150 in uint16 code units. 99.65% of channel
samples differed. Three independently decoded 48 × 48 upper-left, center,
and lower-right regions had mean absolute differences of 3706.15, 4118.22,
and 3452.26. The frozen contract provides no real-image code-identity
threshold for those quantities. Equal-quality review remains open.
Both exports were deleted and all nine original hashes matched before and
after the checks.
