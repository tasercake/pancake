//! Numerical contracts for one image kernel shared by CPU and GPU runtimes.

use cubecl::Device;
use pancake::blur_horizontal_u16;
// Keep the linked frontend in this native qualification dependency graph.
use slint as _;

/// Check independent hand-calculated values, clamped borders and full u16 range.
///
/// # Errors
///
/// Returns runtime failures or a numerical contract mismatch.
fn check_odd_plane(device: &Device) -> Result<(), String> {
    let input = [0, 4, 8, 12, 16, 65_535, 65_535, 0, 1, 65_534, 1, 2, 3, 4, 5];
    let expected = [
        1, 4, 8, 12, 15, 65_535, 49_151, 16_384, 16_384, 49_150, 1, 2, 3, 4, 4,
    ];
    let actual = blur_horizontal_u16(device, 5, 3, &input)?;
    if actual != expected {
        return Err(format!(
            "clamped u16 blur mismatch: {actual:?} != {expected:?}"
        ));
    }
    Ok(())
}

/// Check workgroup tails and one-pixel-wide borders without a scalar twin kernel.
///
/// # Errors
///
/// Returns runtime failures or a boundary contract mismatch.
fn check_boundaries(device: &Device) -> Result<(), String> {
    if blur_horizontal_u16(device, 1, 3, &[0, 32_768, 65_535])? != [0, 32_768, 65_535] {
        return Err("one-pixel-wide images must be unchanged".to_owned());
    }
    let constant = vec![65_535; 771];
    if blur_horizontal_u16(device, 257, 3, &constant)? != constant {
        return Err("an odd workgroup tail must preserve a constant image".to_owned());
    }
    Ok(())
}

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

/// GPU availability is a qualification requirement, so failures are not skipped.
///
/// # Errors
///
/// Returns acquisition, artifact-write, runtime or numerical contract failures.
#[test]
fn wgpu_blur_matches_independent_values() -> Result<(), String> {
    let selector = cubecl::wgpu::WgpuDevice::default();
    let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let setup = cubecl::wgpu::try_init_setup::<cubecl::wgpu::AutoGraphicsApi>(&selector, options)
        .map_err(|error| error.to_string())?;
    let adapter = setup.adapter.get_info();
    if adapter.device_type == wgpu::DeviceType::Cpu {
        return Err("GPU qualification must not silently use a software adapter".to_owned());
    }
    std::fs::write(
        "target/p0-kernel-adapter.txt",
        format!(
            "backend = {:?}\nadapter = {adapter:#?}\ncpu = {:#?}\n",
            setup.backend,
            Device::Cpu(cubecl::cpu::CpuDevice)
                .client()
                .properties()
                .identity
        ),
    )
    .map_err(|error| error.to_string())?;
    let device = Device::Wgpu(selector);
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
