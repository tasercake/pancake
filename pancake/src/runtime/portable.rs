//! Physical wgpu adapters and reusable renderer/compute setup.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use cubecl::Device;
use cubecl::device::{WgpuBackend, WgpuDevice, WgpuDeviceKind};
use cubecl::wgpu::{AutoGraphicsApi, RuntimeOptions, try_init_setup};

use super::{Backend, SelectedRuntime, check_request, configure_cache, qualify_device};

/// Successful setups indexed by their concrete API and selector.
///
/// Serializing registration prevents two workers from initializing the same
/// `CubeCL` client ID. There is at most one setup per detected physical adapter.
static SETUPS: LazyLock<Mutex<HashMap<WgpuDevice, SelectedRuntime>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Prepare and qualify a physical wgpu adapter for renderer/worker interop.
///
/// Enumerated indices start with discrete GPUs and continue with integrated
/// GPUs, in driver order within each class. An unspecified index permits
/// trying the next adapter if an earlier one cannot meet the request.
///
/// # Errors
///
/// Reports a missing physical adapter, an invalid budget or index, or a
/// driver/setup/kernel error if no suitable device can pass qualification.
pub fn prepare_wgpu_runtime(
    gpu_index: Option<usize>,
    budget_bytes: usize,
) -> Result<SelectedRuntime, String> {
    check_request(gpu_index, budget_bytes)?;
    let api = primary_api();
    // The leading discrete Vulkan selector is the first entry of the full
    // inventory. Its setup already discovers and verifies the physical adapter.
    // Most indexed and automatic jobs can finish that single acquisition.
    let preferred_failure = if api == WgpuBackend::Vulkan && gpu_index.unwrap_or(0) == 0 {
        let selector = WgpuDevice::new(WgpuDeviceKind::DiscreteGpu(0)).on(api);
        match acquire(&selector, budget_bytes) {
            Ok(runtime) => return Ok(runtime),
            Err(error) => Some((selector, error)),
        }
    } else {
        None
    };
    let acquire_selected = |selector: &WgpuDevice| match &preferred_failure {
        Some((failed, error)) if failed == selector => Err(error.clone()),
        _ => acquire(selector, budget_bytes),
    };
    let adapters = physical_adapters(api).collect::<Vec<_>>();
    if let Some(index) = gpu_index {
        let selector = adapters.get(index).ok_or_else(|| {
            format!(
                "wgpu {api:?} GPU index {index} is absent ({} physical adapters)",
                adapters.len()
            )
        })?;
        return acquire_selected(selector);
    }

    let mut last_failure = format!("wgpu {api:?} found no physical GPU");
    for selector in &adapters {
        match acquire_selected(selector) {
            Ok(runtime) => return Ok(runtime),
            Err(error) => last_failure = error,
        }
    }
    Err(last_failure)
}

/// Pin the API so Vulkan, D3D12 and Apple Metal cannot be confused.
const fn primary_api() -> WgpuBackend {
    if cfg!(target_os = "macos") {
        WgpuBackend::Metal
    } else if cfg!(target_os = "windows") {
        WgpuBackend::Dx12
    } else {
        WgpuBackend::Vulkan
    }
}

/// Discover physical adapters without opening a display or enumerating a
/// native CUDA, HIP, or Metal runtime.
fn physical_adapters(api: WgpuBackend) -> impl Iterator<Item = WgpuDevice> {
    [
        WgpuDeviceKind::DiscreteGpu(0),
        WgpuDeviceKind::IntegratedGpu(0),
    ]
    .into_iter()
    .flat_map(move |kind| {
        let selector = Device::Wgpu(WgpuDevice::new(kind).on(api));
        Device::enumerate(selector.to_id())
    })
    .filter_map(|id| match Device::from_id(id) {
        Device::Wgpu(selector) => Some(selector),
        _ => None,
    })
}

/// Qualify each selector once per process, then validate each job's budget.
///
/// # Errors
///
/// Reports registration, driver, probe or capacity failures.
fn acquire(selector: &WgpuDevice, budget_bytes: usize) -> Result<SelectedRuntime, String> {
    let mut setups = SETUPS.lock().map_err(|error| error.to_string())?;
    let runtime = if let Some(previous) = setups.get(selector) {
        previous.clone()
    } else {
        let runtime = initial_setup(selector)?;
        let _previous = setups.insert(selector.clone(), runtime.clone());
        runtime
    };
    drop(setups);
    runtime.check_job_budget(budget_bytes)?;
    Ok(runtime)
}

/// Acquire wgpu setup and verify the adapter feeding `CubeCL`.
///
/// # Errors
///
/// Reports driver/setup errors, a nonphysical adapter or a failed kernel probe.
fn initial_setup(selector: &WgpuDevice) -> Result<SelectedRuntime, String> {
    configure_cache();
    let options = RuntimeOptions::try_default().map_err(|error| error.to_string())?;
    let setup = try_init_setup::<AutoGraphicsApi>(selector, options)
        .map_err(|error| format!("wgpu {selector:?} unavailable: {error}"))?;
    let adapter = setup.adapter.get_info();
    if !matches!(
        adapter.device_type,
        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
    ) {
        return Err(format!(
            "wgpu cannot qualify {:?} as a physical GPU",
            adapter.name
        ));
    }
    let name = match setup.backend {
        wgpu::Backend::Metal => "wgpu-metal",
        wgpu::Backend::Dx12 => "wgpu-dx12",
        wgpu::Backend::Vulkan => "wgpu-vulkan",
        other => return Err(format!("unexpected wgpu graphics API {other:?}")),
    };
    qualify_device(
        Device::Wgpu(selector.clone()),
        Backend::Gpu,
        name,
        Some(Arc::new(setup)),
    )
}
