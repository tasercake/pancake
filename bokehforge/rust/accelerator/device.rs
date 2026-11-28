use std::panic::catch_unwind;
use std::sync::OnceLock;

use cubecl::{Device, client::Client};

pub(super) fn client(device: &str) -> Result<Client, String> {
    match device {
        "cpu" => {
            static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
            cached(&CLIENT, "CPU", || {
                Device::cpu().map_err(|error| error.to_string())
            })
        }
        #[cfg(any(target_os = "linux", target_os = "windows"))]
        "cuda" => {
            static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
            cached(&CLIENT, "CUDA", cuda_device)
        }
        #[cfg(target_os = "macos")]
        "mps" | "metal" => {
            static CLIENT: OnceLock<Result<Client, String>> = OnceLock::new();
            cached(&CLIENT, "Metal", metal_device)
        }
        _ => Err(format!("{device} is unavailable on this platform")),
    }
}

fn cached(
    cache: &OnceLock<Result<Client, String>>,
    name: &str,
    select: fn() -> Result<Device, String>,
) -> Result<Client, String> {
    cache
        .get_or_init(|| {
            catch_unwind(|| select().map(|device| device.client()))
                .map_err(|_| format!("CubeCL could not initialize the {name} device"))?
        })
        .clone()
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn cuda_device() -> Result<Device, String> {
    // SAFETY: cudarc checks whether it can load the NVIDIA driver library.
    // This keeps a missing driver out of CubeCL's panicking loader path.
    if !unsafe { cudarc::driver::sys::is_culib_present() } {
        return Err("the NVIDIA CUDA driver library was not found".into());
    }
    Device::cuda(0).map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn metal_device() -> Result<Device, String> {
    use cubecl::device::WgpuDeviceKind;
    use cubecl::metal::device::{default_device, register_device};
    use objc2_metal::{MTLDevice, MTLGPUFamily};

    let device = default_device().ok_or("no Metal GPU was found")?;
    let selected = if device.supportsFamily(MTLGPUFamily::Metal3) {
        Device::metal(register_device(device))
    } else {
        // WGPU's native MSL backend also validates Mac2 Metal adapters, such
        // as Apple Paravirtual GPUs, that do not report the Metal3 family.
        Device::metal_msl(WgpuDeviceKind::DefaultDevice)
    };
    selected.map_err(|error| error.to_string())
}
