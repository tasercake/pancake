//! Physical-GPU integration tests exercise the same fusion kernels as native CPU.

use std::env;

#[path = "common/fusion_cases.rs"]
pub mod cases;

/// Execute weighted fusion, depth selection and resize on a physical GPU.
/// # Errors
/// Returns device acquisition, runtime or numerical contract failures.
#[test]
fn physical_gpu_fusion_and_resize() -> Result<(), String> {
    let choice = match env::var("PANCAKE_TEST_GPU_RUNTIME") {
        Ok(choice) => choice,
        Err(env::VarError::NotPresent) => "wgpu".to_owned(),
        Err(error) => return Err(error.to_string()),
    };
    let (device, identity) = match choice.as_str() {
        "wgpu" => physical_wgpu()?,
        "cuda" => native(cubecl::Device::cuda(0))?,
        "metal" => native(cubecl::Device::metal(cubecl::device::MetalDevice::default()))?,
        other => {
            return Err(format!(
                "unknown physical GPU qualification runtime {other:?}"
            ));
        }
    };
    cases::focus_contract(&device)?;
    cases::resize_contract(&device)?;
    std::fs::create_dir_all("target").map_err(|error| error.to_string())?;
    std::fs::write("target/p3-fusion-gpu-adapter.txt", identity).map_err(|error| error.to_string())
}

/// Select a wgpu device and verify its physical adapter classification.
/// # Errors
/// Returns setup failure or a nonphysical-adapter error.
fn physical_wgpu() -> Result<(cubecl::Device, String), String> {
    let selector = cubecl::wgpu::WgpuDevice::default();
    let options = cubecl::wgpu::RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let setup = cubecl::wgpu::try_init_setup::<cubecl::wgpu::AutoGraphicsApi>(&selector, options)
        .map_err(|error| error.to_string())?;
    let info = setup.adapter.get_info();
    if !matches!(
        info.device_type,
        wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
    ) {
        return Err("physical GPU qualification cannot use a nonphysical adapter".to_owned());
    }
    Ok((
        cubecl::Device::Wgpu(selector),
        format!("backend={:?}\nadapter={info:#?}\n", setup.backend),
    ))
}

/// Record the native runtime's queried identity before launching the oracle.
/// # Errors
/// Returns a diagnostic if the named runtime/device is unavailable.
fn native(
    device: Result<cubecl::Device, cubecl::device::DeviceUnavailable>,
) -> Result<(cubecl::Device, String), String> {
    let device = device.map_err(|error| error.to_string())?;
    let client = device.client();
    let identity = format!(
        "backend={:?}\ndevice={:#?}\n",
        device.runtime(),
        client.properties().identity
    );
    Ok((device, identity))
}
