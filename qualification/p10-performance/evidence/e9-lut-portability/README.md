# E9 coefficient table registration qualification

This replay covers the exact shared-table candidate
`a919045e8004e138df9fcc05650d9e87e50e0e36` and the unchanged stock source
`75c3213a8e4c1d91298c4ba954102fd295eaa114`. The locked candidate release
CLI SHA-256 is
`95923ea36d753a348352b7eba91a1d14e08322617d16c5239f7b357eb6682671`.
The native build contains linked CubeCL CUDA, CubeCL wgpu and Vulkan HAL
symbols. The frozen target descriptor ran that exact CLI.

The independent comparison links each source with the same CUDA-enabled
profile, lock and Rust 1.100.0. Both comparison test programs include the same
synthetic fixture and output logic derived from the prior native CPU probe.
They differ at the two private calls that receive the candidate table handle.
Each test exercises identity, signed-shift, and flat ambiguity at FFT sides
64 and 128. It verifies real window, FFT, and cross-phase dispatch
counts, estimates, scored affine hypotheses and all inverse-of-forward f32
surface channels. Comparison is within each named physical backend, on one
RTX 3090 at PCI `0000:01:00.0`. The published evidence retains backend
provenance, case outputs as hashes, and the raw f32 comparisons.

## Results

The four comparison runs used NVIDIA driver 580.178.04 and the same
physical RTX 3090. Stock and candidate shared the Vulkan fingerprint
`spirv_4318_8708` and the CUDA fingerprint `ptx_sm86`. Each backend matched
all 122,880 f32 channels across six inverse-of-forward surfaces, with
zero maximum absolute difference and identical scores and decisions.
Each fixture submitted two window, six FFT, and one cross-phase launch.
[Physical parity audit and per-fixture digests](physical-audit.json)

| Test executable | Source | SHA-256 |
| --- | --- | --- |
| Stock | `75c3213` | `31d25181cbefbe220c12b47d8198cc078cebe0b625d09928e8e3f90fab815a0c` |
| Coefficient table | `a919045` | `6efaf56c0add1a6fdd1652c4cdde25cf5c52cea6f7d747de4b42a9b16d3f0432` |
