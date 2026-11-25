# Vulkan ordinal regression on the physical RTX 3090

The saved `0a9ae85` mutation at `src/runtime/portable.rs:38:85` changes
the leading Vulkan branch from `gpu_index.unwrap_or(0) == 0` to
`!= 0`. Its exact archived patch SHA-256 is
`6611ef0d4a09ab11607368613526d57467bb581b67a7b16e9a47d094db1e4f9f`.
The checked build applied that original patch and restored the product
source afterward.

The test-only revision `8df18fdc7d542e3f979da182141cf77302dc7473`
extends the existing native `gpu_runtime_contract` target. It qualifies
and runs real work on explicit ordinal zero, enumerates the physical
roster for the same wgpu API, then asks for its first absent ordinal.
The legitimate outcome is the exact absent-ordinal error. If selection
unexpectedly succeeds, the test runs the real checked blur on the
selected physical backend and includes its identity and output in the
failing assertion.

## Exact execution

The pinned NVIDIA-only ICD listed one physical discrete adapter,
an RTX 3090 at PCI `0000:01:00.0`, UUID
`GPU-c7900bb5-e779-c07b-a37d-4adff841c8ac`, driver `580.178.04`.
All four processes used physical `wgpu-vulkan`, fingerprint
`spirv_4318_8708`, for their positive ordinal-zero probe with no
fallback. Stock rejected the first absent ordinal, 1. The exact mutant
ran that request on the first RTX and returned the real blur result
`[2, 8, 14]`, then failed the ordinal assertion. The retained stock
executable rejected ordinal 1 again after source restoration.

| Process | Exit |
| --- | ---: |
| Existing ordinal-zero positive control | 0 |
| Stock absent-ordinal contract | 0 |
| Exact mutant absent-ordinal contract | 101 |
| Restored stock absent-ordinal contract | 0 |

The original wrapper exited 1 after all four processes completed. Its
last fixed-string check omitted escaping from Rust's rendered error
string. The offline audit decoded that string and verified the actual
process results, receipts and preflight. Both the wrapper exit and
unaltered logs are retained. No device rerun was needed. All device work
was terminal at 2026-11-18 18:31:02 UTC.

## Build identity and scope

Both release test artifacts used Rust 1.100.0, two direct Cargo jobs on
CPUs 8 and 9, and the locked offline default application graph. The
Cargo.lock SHA-256 is
`f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`.

| Artifact | SHA-256 | GNU Build ID |
| --- | --- | --- |
| Stock | `6620b6534490707a7239143eadf6dbd3436804534d64bef71906a9d45274567e` | `1ac8783a0a54d94d80d15d36ae920fa663d9fe22` |
| Mutant | `d898960ed45da1ff94e42ced9374e4fbd2d70ad00857a95a4e7af86f778e5ce3` | `3b011facc386d3fd8f8cc60217d7c19e0488cacf` |

The native replay proves the physical Vulkan correctness regression.
The canonical CPU-only mutation campaign remains a separate gate.
The evidence requires no CLI rebuild, performance samples, real
image fixtures, or other backend execution.
