//! Physical-GPU qualification, separate from host sanitizer execution.

#[path = "common/numerical_cases.rs"]
pub mod numerical_cases;

use wgpu as _;

/// Physical GPU must execute the same kernel and preserve independent values.
///
/// # Errors
/// Returns acquisition, runtime, numerical or ownership failures.
#[test]
fn gpu_variance_and_reuse() -> Result<(), String> {
    let selector = cubecl::wgpu::WgpuDevice::default();
    let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let setup = cubecl::wgpu::try_init_setup::<cubecl::wgpu::AutoGraphicsApi>(&selector, options)
        .map_err(|error| error.to_string())?;
    if setup.adapter.get_info().device_type == wgpu::DeviceType::Cpu {
        return Err("physical GPU qualification cannot use a software adapter".to_owned());
    }
    let device = cubecl::Device::Wgpu(selector);
    numerical_cases::check_variance(&device)?;
    numerical_cases::check_budget_and_reuse(&device, "gpu")?;
    std::fs::write(
        "target/p0-numerical-gpu-adapter.txt",
        format!(
            "backend={:?}\nadapter={:#?}\n",
            setup.backend,
            setup.adapter.get_info()
        ),
    )
    .map_err(|error| error.to_string())
}
