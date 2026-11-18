//! Runtime routing contracts for the strict native CPU selection.

use cubecl as _;
use pancake::config::DevicePreference;
use pancake::runtime::{Backend, RuntimeOverride, select_runtime};
use wgpu as _;

include!("common/runtime_assertions.rs");

/// Ensure a strict CPU request identifies native execution and reaches a real
/// completed kernel. The CPU request must leave graphics uninitialized.
///
/// # Errors
///
/// Returns a CPU setup, kernel or synchronization error.
#[test]
fn explicit_cpu_qualifies_a_real_shared_kernel() -> Result<(), String> {
    let runtime = select_runtime(DevicePreference::Cpu, None, 1024, None)?;
    require(
        runtime.backend() == Backend::Cpu,
        "CPU request changed backend",
    )?;
    require(
        runtime.runtime_name() == "cpu",
        "CPU request changed runtime",
    )?;
    require(
        !runtime.physical_gpu(),
        "CPU was classified as a physical GPU",
    )?;
    require(
        !runtime.device_name().is_empty(),
        "runtime omitted its device name",
    )?;
    require(
        runtime.wgpu_setup().is_none(),
        "CPU initialized a graphics device",
    )?;
    require(
        runtime.fallback_reason().is_none(),
        "CPU request reported fallback",
    )?;
    let pixels = pancake::blur_horizontal_u16(runtime.device(), 3, 1, &[0, 8, 16])?;
    require(
        pixels == [2, 8, 14],
        "selected CPU failed a known-value computation",
    )?;
    runtime.synchronize().map_err(|error| error.to_string())
}

/// Reject incompatible device settings before acquiring hardware.
///
/// # Errors
///
/// Reports a missing or misleading rejection.
#[test]
fn incompatible_and_overflowing_device_requests_fail() -> Result<(), String> {
    let conflict = select_runtime(
        DevicePreference::Cpu,
        None,
        1024,
        Some(RuntimeOverride::Wgpu),
    )
    .err();
    require(
        conflict.is_some_and(|reason| reason.contains("CPU") && reason.contains("wgpu")),
        "the error must identify the contradictory CPU and wgpu settings",
    )?;
    let invalid_index = select_runtime(
        DevicePreference::Gpu,
        Some(usize::MAX),
        1024,
        Some(RuntimeOverride::Wgpu),
    )
    .err();
    require(
        invalid_index.is_some_and(|reason| reason.contains("index")),
        "an index too large for the runtime must fail before acquisition",
    )?;
    let budget = select_runtime(DevicePreference::Cpu, None, 19, None).err();
    require(
        budget.is_some_and(|reason| reason.contains("budget")),
        "a budget too small for qualification must fail before acquisition",
    )?;
    Ok(())
}
