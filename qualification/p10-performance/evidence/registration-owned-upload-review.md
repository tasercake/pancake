# Registration upload ownership review

Reviewed `07816a24a62f96bc3d3252e95fcfd2b4b5950269` against `287828aac07ffad2252785b73a52a59fc07b3409`. The candidate changes only the registration upload helper and its reviewed source hash. No source blocker was found. The lock remains `f523404bf033b0532dc46ce71aa515dc398f56c32e6aee1c3529cf90a27018dc`. The Miri platform source and dependency inventory validator and `git diff --check` passed. Review work was source and receipt inspection.

## Byte representation and ownership

The pinned CubeCL 0.11.4 environment implementation represents the empty native allocation by a non-null, aligned sentinel. `Bytes::from_bytes_vec(Vec::new())` enforces `align_of::<u128>()` while the capacity and length are zero. `extend_from_byte_slice` reserves aligned capacity, rounds the backing allocation up, copies the borrowed byte slice into that storage, and sets logical length to the copied bytes only. The first growth makes one payload allocation. The prior borrowed upload made two temporary `Vec<u8>` copies and then allocated native-aligned `Bytes` due to the `Vec<u8>` layout's recorded alignment. Both API paths deliver the same native allocation property and logical byte length to the backend.

The pinned `CubeElement for f32` implements `as_bytes` as a bytemuck slice cast. Both versions therefore copy exactly the same native-endian float representations, with no numerical conversion or serialization. The CPU readback decoder still uses native byte order. The shipped x86-64 and Apple ARM hardware use little endian. Native bitwise equivalence to the previous path does not add a cross-endian portability claim. Both client APIs produce the same contiguous `[byte_len]` copy descriptor with byte element size 1, allocate on the same stream and move the transfer descriptor and bytes into the same initialize/write closure. The typed tensor retains its contiguous `[sample_count]` shape and F32 metadata.

The CPU scheduler owns `Bytes` until its host copy reaches the managed resource. The wgpu scheduler owns it until its mapped or queue-staging copy. That includes the normal wgpu Metal selection. The native Metal server waits for the prior stream and copies the same slice into its buffer. CUDA and retained HIP use the common host-transfer path and the same pinned staging decisions for native bytes. A fence-backed drop queue or capture window retains the buffer until the driver finishes with it. AOT and JIT kernel selection, code generation, binding metadata, and capture rules are unaffected by the choice of host buffer constructor.

Frame dimensions, the validated packed RGB sample count, registration byte-budget admission, finite readback and affine layout are unchanged. Registration still keeps the input on the same client across tiled warps and retains the same `ResidentRgb` handle for the admitted identity path. The previous conservative admission calculation remains in place.

## Read-back of retained evidence

The Linux x86-64 GDB probe uses a 6000×4000 RGB f32 frame (288,000,000 logical bytes) and checks the production CPU warp's sample bits. Both binaries exited normally. Independent parsing of the symbolized mmap stacks found three registration staging mappings at the baseline and one in the candidate, each 288,002,048 bytes including the allocator/page overhead. Counting the borrowed probe input gives four and two total mappings. This measurement excludes later device or CPU managed-resource copies and is not a peak memory or timing result.

The recorded CPU P5 report contains 63 distinct selected IDs, all complete and passed, with zero failed or unsupported. Runner provenance points to the clean exact `07816a2` target and suite checkout. Physical T50 and native platform behavior still require their own execution evidence.

| Retained input | SHA256 |
| --- | --- |
| Stock GDB stdout | `810ca32454f53ae70c9f3b689fbbdd27e98dbbea980e725a92fad693ace640ed` |
| Candidate GDB stdout | `0fdb0a03fe28ac094f3bf674214833104416a470bb98e80739451592f18fd592` |
