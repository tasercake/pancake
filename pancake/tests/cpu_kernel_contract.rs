//! CPU execution and malformed-input contracts for shared kernel launch/readback.

include!("common/kernel_cases.rs");

/// CPU must execute the same kernel used by the GPU.
///
/// # Errors
///
/// Returns runtime failures or a numerical contract mismatch.
#[test]
fn cpu_blur_matches_independent_values() -> Result<(), String> {
    let device = Device::Cpu(cubecl::cpu::CpuDevice);
    check_odd_plane(&device)?;
    check_boundaries(&device)
}

/// Invalid shapes must fail before allocating or starting any runtime.
///
/// # Panics
///
/// Panics if an invalid shape fails to return its specified recoverable error.
#[test]
fn invalid_shapes_are_recoverable() {
    let device = Device::Cpu(cubecl::cpu::CpuDevice);
    for (width, height, input, error) in [
        (0, 1, &[][..], "image dimensions must be nonzero"),
        (1, 0, &[][..], "image dimensions must be nonzero"),
        (usize::MAX, 2, &[][..], "image area exceeds u32 addressing"),
        (
            2,
            2,
            &[1, 2, 3][..],
            "pixel count does not match image dimensions",
        ),
    ] {
        assert_eq!(
            blur_horizontal_u16(&device, width, height, input),
            Err(error.to_owned()),
            "invalid shape must produce a recoverable error"
        );
    }
}
