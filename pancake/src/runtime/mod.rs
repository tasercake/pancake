//! Runtime selection and qualification for the shared image engine.

use std::fmt::{self, Display, Formatter};
use std::panic::{UnwindSafe, catch_unwind};
use std::str::FromStr;
use std::sync::{Arc, Once};

use cubecl::Device;
use cubecl::device::{DeviceUnavailable, MetalDevice};
use cubecl::server::ServerError;
use cubecl::wgpu::WgpuSetup;

use crate::{config::DevicePreference, row_variance_u16};

mod portable;
pub use portable::prepare_wgpu_runtime;

/// Logical byte budget for the two-sample variance probe.
const PROBE_BUDGET_BYTES: usize = 44;

/// Numerical execution class recorded after a successful job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    /// Native CPU runtime.
    Cpu,
    /// Qualified physical accelerator.
    Gpu,
}

impl Backend {
    /// Stable completion-event value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }
}

/// Explicit accelerator API for reproducible device qualification and runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeOverride {
    /// wgpu on the platform's primary graphics API.
    Wgpu,
    /// Native NVIDIA CUDA.
    Cuda,
    /// Native Apple Metal.
    Metal,
    /// Native AMD HIP.
    Hip,
}

impl RuntimeOverride {
    /// Stable spelling accepted by a runtime override setting.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wgpu => "wgpu",
            Self::Cuda => "cuda",
            Self::Metal => "metal",
            Self::Hip => "hip",
        }
    }
}

impl FromStr for RuntimeOverride {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "wgpu" => Ok(Self::Wgpu),
            "cuda" => Ok(Self::Cuda),
            "metal" => Ok(Self::Metal),
            "hip" => Ok(Self::Hip),
            _ => Err("expected wgpu, cuda, metal, or hip"),
        }
    }
}

/// A selected runtime failed after work was scheduled.
#[derive(Debug)]
pub enum RuntimeExecutionError {
    /// The driver rejected the device after an unrecoverable fault.
    DeviceLost(String),
    /// Another execution failure, which cannot trigger automatic CPU fallback.
    Other(String),
}

impl RuntimeExecutionError {
    /// Preserve upstream device-poisoning semantics through nested errors.
    #[must_use]
    pub fn from_server_error(error: &ServerError) -> Self {
        if error.is_device_poisoned() {
            Self::DeviceLost(error.to_string())
        } else {
            Self::Other(error.to_string())
        }
    }
}

impl Display for RuntimeExecutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceLost(reason) => write!(formatter, "compute device lost: {reason}"),
            Self::Other(reason) => write!(formatter, "compute execution failed: {reason}"),
        }
    }
}

impl std::error::Error for RuntimeExecutionError {}

/// A device that has already produced a checked, completed qualification result.
///
/// A job still needs to finish on this device before reporting completion.
#[derive(Clone, Debug)]
pub struct SelectedRuntime {
    /// Device used by all numerical kernels.
    device: Device,
    /// CPU or physical GPU.
    backend: Backend,
    /// Concrete execution API, including the underlying API on wgpu.
    runtime_name: &'static str,
    /// Model from the acquired compute client.
    device_name: String,
    /// Compilation identity from the acquired compute client.
    fingerprint: String,
    /// PCI identity reported by the backend, if available.
    device_pci_address: Option<String>,
    /// Maximum single resource size exposed by the selected runtime.
    max_allocation_bytes: u64,
    /// Total memory limit when reported by the runtime.
    memory_capacity_bytes: Option<u64>,
    /// Adapter, device and queue shared with the renderer when used.
    wgpu_setup: Option<Arc<WgpuSetup>>,
    /// Recorded only when automatic selection could not qualify a GPU.
    fallback_reason: Option<String>,
}

impl SelectedRuntime {
    /// Numerical device used by the worker.
    #[must_use]
    pub const fn device(&self) -> &Device {
        &self.device
    }

    /// Backend class after qualification.
    #[must_use]
    pub const fn backend(&self) -> Backend {
        self.backend
    }

    /// Whether the selected compute runtime runs on a physical accelerator.
    #[must_use]
    pub const fn physical_gpu(&self) -> bool {
        matches!(self.backend, Backend::Gpu)
    }

    /// Concrete execution API, such as `wgpu-vulkan` or `cuda`.
    #[must_use]
    pub const fn runtime_name(&self) -> &'static str {
        self.runtime_name
    }

    /// Model reported by the compute client.
    #[must_use]
    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    /// Runtime compilation identity.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// PCI address matching external hardware telemetry when available.
    #[must_use]
    pub fn device_pci_address(&self) -> Option<&str> {
        self.device_pci_address.as_deref()
    }

    /// Per-resource byte limit before runtime allocation.
    #[must_use]
    pub const fn max_allocation_bytes(&self) -> u64 {
        self.max_allocation_bytes
    }

    /// Total runtime memory capacity if the runtime can report it.
    #[must_use]
    pub const fn memory_capacity_bytes(&self) -> Option<u64> {
        self.memory_capacity_bytes
    }

    /// Setup whose device and queue wgpu compute and rendering share.
    #[must_use]
    pub fn wgpu_setup(&self) -> Option<&WgpuSetup> {
        self.wgpu_setup.as_deref()
    }

    /// GPU qualification failure that prompted automatic CPU use.
    #[must_use]
    pub fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }

    /// Wait for submitted work on the calling worker's stream to settle.
    ///
    /// # Errors
    ///
    /// Reports execution failure, including unrecoverable device loss.
    pub fn synchronize(&self) -> Result<(), RuntimeExecutionError> {
        cubecl::future::block_on(self.device.client().sync())
            .map_err(|error| RuntimeExecutionError::from_server_error(&error))
    }

    /// Check a workload against the total byte capacity if it is reported.
    ///
    /// # Errors
    ///
    /// Returns an error if the request cannot fit in the capacity.
    fn check_capacity(&self, budget_bytes: usize) -> Result<(), String> {
        if let Some(capacity) = self.memory_capacity_bytes {
            let required = u64::try_from(budget_bytes).map_err(|error| error.to_string())?;
            if required > capacity {
                return Err(format!(
                    "{} has a {capacity}-byte memory limit below the {required}-byte job budget",
                    self.runtime_name
                ));
            }
        }
        Ok(())
    }
}

/// Select a numerical runtime after a small real kernel completes.
///
/// A forced CPU job never asks the graphics system for adapters. Explicit GPU,
/// API and index requests fail if the named path cannot qualify. Only an
/// unforced automatic job may fall back before it starts.
///
/// # Errors
///
/// Reports conflicting settings, invalid index or budget, or a runtime that
/// is unavailable or fails its kernel probe.
pub fn select_runtime(
    preference: DevicePreference,
    gpu_index: Option<usize>,
    budget_bytes: usize,
    requested: Option<RuntimeOverride>,
) -> Result<SelectedRuntime, String> {
    check_request(gpu_index, budget_bytes)?;
    if preference == DevicePreference::Cpu {
        if let Some(runtime) = requested {
            return Err(format!(
                "the requested {} runtime conflicts with explicit CPU",
                runtime.as_str()
            ));
        }
        if gpu_index.is_some() {
            return Err("a GPU index conflicts with explicit CPU".to_owned());
        }
        return cpu_runtime(budget_bytes);
    }

    if let Some(runtime) = requested {
        return accelerator_runtime(runtime, gpu_index, budget_bytes);
    }

    match prepare_wgpu_runtime(gpu_index, budget_bytes) {
        Ok(runtime) => Ok(runtime),
        Err(wgpu_error) if gpu_index.is_none() => {
            let alternate = if cfg!(target_os = "macos") {
                RuntimeOverride::Metal
            } else {
                RuntimeOverride::Cuda
            };
            match accelerator_runtime(alternate, None, budget_bytes) {
                Ok(runtime) => Ok(runtime),
                Err(native_error) if preference == DevicePreference::Auto => {
                    let mut cpu = cpu_runtime(budget_bytes)?;
                    cpu.fallback_reason = Some(format!("{wgpu_error}; {native_error}"));
                    Ok(cpu)
                }
                Err(native_error) => Err(format!("{wgpu_error}; {native_error}")),
            }
        }
        Err(error) => Err(error),
    }
}

/// Refuse requests whose identity or minimum probe cannot be represented.
///
/// # Errors
///
/// Returns an error for an out-of-range index or insufficient probe budget.
fn check_request(gpu_index: Option<usize>, budget_bytes: usize) -> Result<(), String> {
    if let Some(index) = gpu_index
        && index > usize::from(u16::MAX)
    {
        return Err(format!(
            "GPU index {index} exceeds the runtime's index range"
        ));
    }
    if budget_bytes < PROBE_BUDGET_BYTES {
        return Err(format!(
            "the {budget_bytes}-byte job budget cannot hold the {PROBE_BUDGET_BYTES}-byte runtime probe"
        ));
    }
    Ok(())
}

/// Honor the task's compute cache root once, before creating any client.
fn configure_cache() {
    /// Single cache root per process, shared by renderer and jobs.
    static CONFIGURE: Once = Once::new();
    CONFIGURE.call_once(|| {
        if let Some(path) =
            std::env::var_os("PANCAKE_COMPUTE_CACHE_DIR").filter(|value| !value.is_empty())
        {
            cubecl::environment::set_root(path);
        }
    });
}

/// Select native CPU and execute its exact-value probe.
///
/// # Errors
///
/// Returns a native CPU setup, probe or capacity failure.
fn cpu_runtime(budget_bytes: usize) -> Result<SelectedRuntime, String> {
    qualify_native("cpu", Backend::Cpu, Device::cpu, budget_bytes)
}

/// Select the exact accelerator API and index requested.
///
/// # Errors
///
/// Returns an accelerator setup, probe or capacity failure.
fn accelerator_runtime(
    runtime: RuntimeOverride,
    gpu_index: Option<usize>,
    budget_bytes: usize,
) -> Result<SelectedRuntime, String> {
    let identify: fn(Option<usize>) -> Result<Device, DeviceUnavailable> = match runtime {
        RuntimeOverride::Wgpu => return prepare_wgpu_runtime(gpu_index, budget_bytes),
        RuntimeOverride::Cuda => |id| Device::cuda(id.unwrap_or(0)),
        RuntimeOverride::Hip => |id| Device::rocm(id.unwrap_or(0)),
        RuntimeOverride::Metal => |id| {
            id.map_or_else(
                || Device::metal(MetalDevice::DefaultDevice),
                |index| {
                    Device::metal(MetalDevice::DiscreteGpu(index))
                        .or_else(|_| Device::metal(MetalDevice::IntegratedGpu(index)))
                },
            )
        },
    };
    qualify_native(
        runtime.as_str(),
        Backend::Gpu,
        || identify(gpu_index),
        budget_bytes,
    )
}

/// Translate unwinds from native initialization into errors.
///
/// # Errors
///
/// Reports unavailable devices, initialization errors, probe errors or capacity limits.
fn qualify_native(
    name: &'static str,
    backend: Backend,
    get_device: impl FnOnce() -> Result<Device, DeviceUnavailable> + UnwindSafe,
    budget_bytes: usize,
) -> Result<SelectedRuntime, String> {
    configure_cache();
    catch_unwind(|| {
        let device = get_device().map_err(|error| format!("{name} unavailable: {error}"))?;
        let result = qualify_device(device, backend, name, None)?;
        result.check_capacity(budget_bytes)?;
        Ok(result)
    })
    .map_err(|payload| {
        let cause = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("native runtime failed during initialization");
        format!("{name} unavailable: {cause}")
    })?
}

/// Complete and check the production variance kernel on one concrete client.
///
/// # Errors
///
/// Reports a nonphysical accelerator, execution failure or unexpected variance.
fn qualify_device(
    device: Device,
    backend: Backend,
    runtime_name: &'static str,
    wgpu_setup: Option<Arc<WgpuSetup>>,
) -> Result<SelectedRuntime, String> {
    let client = device.client();
    let properties = client.properties();
    if backend == Backend::Gpu && properties.identity.physical.is_none() {
        return Err(format!(
            "{runtime_name} did not identify a physical accelerator"
        ));
    }
    let variance = row_variance_u16(&device, 2, 1, &[1, 9], PROBE_BUDGET_BYTES)
        .map_err(|error| format!("{runtime_name} qualification failed: {error}"))?;
    if variance.as_slice() != [16.0_f32] {
        return Err(format!(
            "{runtime_name} produced an incorrect qualification result"
        ));
    }
    Ok(SelectedRuntime {
        device,
        backend,
        runtime_name,
        device_name: properties.identity.name.clone(),
        fingerprint: properties.identity.fingerprint.clone(),
        device_pci_address: properties
            .identity
            .physical
            .as_ref()
            .and_then(|device| device.pci_address)
            .map(|address| address.to_string()),
        max_allocation_bytes: properties.memory.max_page_size,
        memory_capacity_bytes: properties.memory.max_memory(),
        wgpu_setup,
        fallback_reason: None,
    })
}
