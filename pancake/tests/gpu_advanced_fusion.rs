//! Physical wgpu execution of the multiscale focus and exposure contracts.

#[path = "common/advanced_fusion_cases.rs"]
pub mod advanced_fusion_cases;

/// Run the shared independent oracles on the registered physical adapter.
///
/// # Errors
/// Returns a setup, runtime, numerical or evidence-write failure.
#[test]
fn gpu_advanced_fusion() -> Result<(), String> {
    let selector = cubecl::wgpu::WgpuDevice::default();
    let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let setup = cubecl::wgpu::try_init_setup::<cubecl::wgpu::AutoGraphicsApi>(&selector, options)
        .map_err(|error| error.to_string())?;
    let info = setup.adapter.get_info();
    if !matches!(
        info.device_type,
        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
    ) {
        return Err(format!(
            "advanced fusion requires a physical adapter, found {info:?}"
        ));
    }
    let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let selected =
        cubecl::wgpu::try_init_device(setup, options).map_err(|error| error.to_string())?;
    let device = cubecl::Device::Wgpu(selected);
    advanced_fusion_cases::check_constant(&device)?;
    advanced_fusion_cases::check_focus(&device)?;
    advanced_fusion_cases::check_exposure_batches(&device)?;
    advanced_fusion_cases::check_exposure_precision(&device)?;
    let memory = &device.client().properties_shared().memory;
    std::fs::write(
        "target/advanced-fusion-gpu.txt",
        format!("{info:#?}\n{memory:#?}\n"),
    )
    .map_err(|error| error.to_string())
}
