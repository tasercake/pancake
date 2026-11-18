//! Run independent registration oracles through a physical GPU.

include!("common/registration_cases.rs");

/// Recover known registration and crop coordinates on a physical wgpu backend.
///
/// # Errors
///
/// Returns a device or numerical error, including software fallback.
#[test]
fn gpu_registration_geometry() -> Result<(), String> {
    let selector = cubecl::wgpu::WgpuDevice::default();
    let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let setup = cubecl::wgpu::try_init_setup::<cubecl::wgpu::AutoGraphicsApi>(&selector, options)
        .map_err(|error| error.to_string())?;
    let adapter = setup.adapter.get_info();
    if !matches!(
        adapter.device_type,
        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
    ) {
        return Err(format!(
            "registration qualification needs a physical GPU: {adapter:?}"
        ));
    }
    let device = cubecl::Device::Wgpu(selector);
    crop_and_warp_preserve_rgb_coordinates(&device)?;
    registration_recovers_subpixel_translation(&device)?;
    featureless_pair_has_zero_confidence(&device)?;
    registration_recovers_wide_shift(&device)
}
