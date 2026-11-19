# Independent E9 GPU image review

The first measured warm E9 pair supports adequate equal sampled image quality on the physical RTX 3090. I examined the actual TIFFs side by side at one source pixel per display pixel and enlarged the three original 48 × 48 windows by 4× nearest neighbor. Both images used the same fixed RGB conversion, rounding the unsigned 16-bit channels onto evenly spaced 8-bit display codes. Measurements used the original unsigned 16-bit arrays.

I rehashed all nine ordered Waterloo Balloons PNGs against the recorded frozen inputs. I verified the actual warm TIFF metadata, one true RGB16 page at 510 × 337, and its encoded and decoded hashes against both the warm and matching separate diagnostic receipts. Each diagnostic reports nine identity maps. The reference reports a one-pixel crop on each side. The Pancake output dimensions and corresponding features agree with the same symmetric crop on the first-frame grid. Pancake completed on CUDA at PCI 0000:01:00.0. The corrected pinned reference completed on the OpenCL device whose UUID matches the same RTX 3090. Neither diagnostic reports a backend fallback.

- Upper-left (103, 60) preserves the balloon contour and nearby bright cloud, with slightly darker and less saturated envelope tones in Pancake.
- Center (231, 144) preserves the tree silhouette and muted cloud contrast. Tone differences are slight.
- Lower-right (358, 228) preserves lettering and equipment detail through the orange glare. Pancake is modestly brighter and warmer here, with mean nominal luma 679 unsigned 16-bit codes higher.
- At native scale, both retain the readable lettering, balloon and cloud edges, rays of the sun and foreground forms. I saw no new seam, halo, double edge or visible representative detail loss. All three fixed windows have zero clipped channel samples.

Whole-frame mean absolute difference is 515.08 unsigned 16-bit codes. Pancake reaches the upper endpoint in 1,414 samples (0.2742%) compared with 683 (0.1325%). The 731 additional saturated samples cluster around the sun and nearby clouds, with matching reference values from 61,270 to 65,519. Pancake also reaches black in 364 blue-channel samples (0.0706%), chiefly in the right balloon artwork, with matching reference values from 266 to 1,199. The reference has zero exact black samples. These are genuine exposure differences and the additional clipped samples showed no discernible loss of texture at native display scale.

The assessment covers the exact observed image pair. The GPU warm median ratio of 0.521 still misses the frozen 0.50 limit, and GPU memory remains pending.
