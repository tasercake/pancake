//! Hardware-specific runtime qualification on the production selector.

use cubecl as _;
use pancake::config::DevicePreference;
use pancake::runtime::{
    Backend, RuntimeOverride, SelectedRuntime, prepare_wgpu_runtime, select_runtime,
};
use wgpu as _;

include!("common/runtime_assertions.rs");

/// A wgpu selection completes a real probe before returning and retains the
/// same shared setup for renderer and worker reuse.
///
/// # Errors
///
/// Returns hardware setup, kernel or synchronization errors.
#[test]
fn wgpu_runtime_qualifies_hardware_and_reuses_setup() -> Result<(), String> {
    let first = prepare_wgpu_runtime(None, 1024)?;
    let second = prepare_wgpu_runtime(None, 1024)?;
    require(
        first.backend() == Backend::Gpu,
        "wgpu selected the wrong class",
    )?;
    require(
        first.device() == second.device(),
        "wgpu registered another device for a second job",
    )?;
    require(first.physical_gpu(), "wgpu returned a software adapter")?;
    require(
        first.fallback_reason().is_none(),
        "strict wgpu request fell back",
    )?;
    let setup = first
        .wgpu_setup()
        .ok_or("wgpu setup was not returned for interop")?;
    let reused = second.wgpu_setup().ok_or("wgpu setup was not retained")?;
    require(
        setup.device == reused.device,
        "renderer and compute would use different devices",
    )?;
    require(
        setup.queue == reused.queue,
        "renderer and compute would use different queues",
    )?;
    let expected_api = if cfg!(target_os = "macos") {
        "wgpu-metal"
    } else if cfg!(target_os = "windows") {
        "wgpu-dx12"
    } else {
        "wgpu-vulkan"
    };
    require(
        first.runtime_name() == expected_api,
        "reported backend does not match selected API",
    )?;
    let output = pancake::blur_horizontal_u16(first.device(), 3, 1, &[16, 0, 12])?;
    require(
        [12, 7, 9] == output.as_slice(),
        "qualified wgpu compute returned wrong samples",
    )?;
    first.synchronize().map_err(|error| error.to_string())
}

/// Exercise the explicitly selected hardware runtime. The test defaults to
/// wgpu. The dedicated CUDA and native Metal lanes pass an explicit selector.
///
/// # Errors
///
/// Returns invalid selector, setup, kernel, synchronization or receipt errors.
#[test]
fn explicit_gpu_override_completes_on_named_runtime() -> Result<(), String> {
    let requested = std::env::var("PANCAKE_TEST_GPU_RUNTIME")
        .map_or(Ok(RuntimeOverride::Wgpu), |value| {
            value.parse::<RuntimeOverride>().map_err(str::to_owned)
        })?;
    let runtime = select_runtime(DevicePreference::Gpu, Some(0), 1024, Some(requested))?;
    require(
        runtime.physical_gpu(),
        "the explicit accelerator was not physical",
    )?;
    require(
        runtime.runtime_name().starts_with(requested.as_str()),
        "runtime override was ignored",
    )?;
    require(
        !runtime.device_name().is_empty(),
        "physical runtime omitted its device name",
    )?;
    require(
        runtime.fallback_reason().is_none(),
        "strict GPU request fell back",
    )?;
    let output = pancake::blur_horizontal_u16(runtime.device(), 3, 1, &[0, 8, 16])?;
    require(
        [2, 8, 14] == output.as_slice(),
        "selected accelerator returned wrong samples",
    )?;
    runtime.synchronize().map_err(|error| error.to_string())?;
    write_receipt(&runtime)
}

/// Automatic mode uses a qualified real device and identifies setup fallback.
///
/// A separate software-only execution can explicitly require the CPU outcome.
///
/// # Errors
///
/// Returns runtime setup, execution, synchronization or receipt errors.
#[test]
fn auto_reports_the_qualified_device() -> Result<(), String> {
    let runtime = select_runtime(DevicePreference::Auto, None, 1024, None)?;
    if std::env::var_os("PANCAKE_TEST_REQUIRE_AUTO_CPU").is_some() {
        require(
            runtime.backend() == Backend::Cpu,
            "software-only lane qualified a GPU",
        )?;
    }
    if runtime.backend() == Backend::Cpu {
        require(
            runtime.fallback_reason().is_some(),
            "automatic CPU use lost its diagnostic",
        )?;
    } else {
        require(
            runtime.physical_gpu(),
            "automatic GPU use accepted software",
        )?;
        require(
            runtime.fallback_reason().is_none(),
            "qualified GPU reported a fallback",
        )?;
    }
    let outcome = pancake::blur_horizontal_u16(runtime.device(), 3, 1, &[0, 8, 16])?;
    require(
        outcome == [2, 8, 14],
        "automatic device returned wrong samples",
    )?;
    runtime.synchronize().map_err(|error| error.to_string())?;
    write_receipt(&runtime)
}

/// Optionally write factual hardware identity for the independent receipt.
///
/// # Errors
///
/// Returns a filesystem error when an explicit receipt cannot be written.
fn write_receipt(runtime: &SelectedRuntime) -> Result<(), String> {
    if let Some(path) = std::env::var_os("PANCAKE_TEST_RUNTIME_RECEIPT") {
        let adapter = runtime.wgpu_setup().map(|setup| setup.adapter.get_info());
        std::fs::write(
            path,
            format!(
                "runtime={}
backend={}
device={}
fingerprint={}
device_pci_address={:?}
physical_gpu={}
maximum_allocation_bytes={}
capacity_bytes={:?}
fallback_reason={:?}
wgpu_adapter={adapter:#?}
",
                runtime.runtime_name(),
                runtime.backend().as_str(),
                runtime.device_name(),
                runtime.fingerprint(),
                runtime.device_pci_address(),
                runtime.physical_gpu(),
                runtime.max_allocation_bytes(),
                runtime.memory_capacity_bytes(),
                runtime.fallback_reason(),
            ),
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}
