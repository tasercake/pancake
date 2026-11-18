//! Physical-GPU qualification never falls back to software or skips hardware failures.

include!("common/kernel_cases.rs");


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
