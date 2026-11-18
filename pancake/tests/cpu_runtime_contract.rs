//! Runtime routing contracts for the strict native CPU selection.

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

/// Runtime selection reports the real CPU capacity and performs no stack work on rejection.
///
/// # Errors
/// Returns absent CPU properties or mismatched capacity and index diagnostics.
#[test]
fn cpu_budget_and_explicit_index_respect_the_qualified_runtime() -> Result<(), String> {
    let cpu = select_runtime(DevicePreference::Cpu, None, 44, None)?;
    let capacity = cpu
        .memory_capacity_bytes()
        .ok_or("native CPU omitted its capacity")?;
    let excessive = usize::try_from(capacity)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or("CPU memory capacity leaves no one-byte-over test value")?;
    let above_capacity = select_runtime(DevicePreference::Cpu, None, excessive, None)
        .err()
        .ok_or("CPU accepted a job budget above its reported capacity")?;
    require(
        above_capacity.contains(&format!("{capacity}-byte memory limit"))
            && above_capacity.contains(&format!("{excessive}-byte job budget")),
        "capacity rejection lost the real available and requested byte counts",
    )?;
    let incompatible = select_runtime(DevicePreference::Cpu, Some(0), 44, None)
        .err()
        .ok_or("explicit CPU accepted a GPU index")?;
    require(
        incompatible.contains("GPU index") && incompatible.contains("explicit CPU"),
        "CPU-index conflict lacked useful diagnostics",
    )
}

/// Public selectors round-trip the exact runtime and device spellings.
///
/// # Errors
/// Returns changed spellings or accepted noncanonical input.
#[test]
fn preferred_runtime_names_round_trip_for_cli_receipts() -> Result<(), String> {
    for (name, runtime) in [
        ("wgpu", RuntimeOverride::Wgpu),
        ("cuda", RuntimeOverride::Cuda),
        ("metal", RuntimeOverride::Metal),
        ("hip", RuntimeOverride::Hip),
    ] {
        require(
            name.parse::<RuntimeOverride>() == Ok(runtime) && runtime.as_str() == name,
            "a backend selector changed its public runtime name",
        )?;
    }
    require(
        Backend::Cpu.as_str() == "cpu"
            && Backend::Gpu.as_str() == "gpu"
            && "CPU".parse::<RuntimeOverride>().is_err(),
        "backend or noncanonical runtime names were reported incorrectly",
    )
}

/// A skipped kernel is an ordinary execution error, never a poisoned device.
///
/// # Errors
/// Returns an incorrect runtime failure classification.
#[test]
fn skipped_work_does_not_claim_device_loss() -> Result<(), String> {
    use pancake::runtime::RuntimeExecutionError;
    let underlying = cubecl::server::ServerError::Skipped;
    let outcome = RuntimeExecutionError::from_server_error(&underlying);
    require(
        matches!(&outcome, RuntimeExecutionError::Other(_))
            && outcome.to_string().contains("work was skipped"),
        "dependency failure became a misleading device fault",
    )
}
