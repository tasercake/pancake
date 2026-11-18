//! CPU integration tests exercise exactly the same fusion kernels as physical GPUs.

#[path = "common/fusion_cases.rs"]
pub mod cases;

use pancake::fusion::{FocusAccumulator, FocusMethod, resize};
use wgpu as _;

/// Execute real kernel launches and read back their numerical results.
/// # Errors
/// Returns runtime, shape or numerical contract failures.
#[test]
fn cpu_fusion_and_resize() -> Result<(), String> {
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    cases::focus_contract(&device)?;
    cases::resize_contract(&device)
}

/// Invalid shape, resource budget and sample count are recoverable errors.
/// # Errors
/// Returns a description if invalid data reached a runtime or was accepted.
#[test]
fn invalid_requests_do_not_produce_a_result() -> Result<(), String> {
    let unavailable = cubecl::Device::Hip(cubecl::device::AmdDevice::default());
    let method = FocusMethod::WeightedAverage;
    for (width, height, kernel, budget, diagnostic) in [
        (0, 1, 3, 4096, "dimensions"),
        (usize::MAX, 2, 3, 4096, "area"),
        (3, 1, 0, 4096, "kernel size"),
        (3, 1, 3, 1, "working-data budget"),
    ] {
        let result = FocusAccumulator::new(&unavailable, width, height, method, kernel, budget);
        if !matches!(result, Err(error) if error.contains(diagnostic)) {
            return Err(format!("invalid fusion request must report {diagnostic}"));
        }
    }
    let oversize = resize(&unavailable, [3, 1], [5, 1], &[0.0; 9], 1);
    if !matches!(oversize, Err(error) if error.contains("working-data budget")) {
        return Err("resize must reject an over-budget request before runtime setup".to_owned());
    }
    let cpu = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    let mut accumulator = FocusAccumulator::new(&cpu, 3, 1, method, 3, 4096)?;
    if !matches!(accumulator.add_frame(&[0.2; 8]), Err(error) if error.contains("sample count"))
        || !matches!(accumulator.add_frame(&[f32::NAN; 9]), Err(error) if error.contains("nonfinite"))
    {
        return Err("invalid frame accepted".to_owned());
    }
    if !matches!(accumulator.finish(), Err(error) if error.contains("no frames")) {
        return Err("empty accumulator produced an image".to_owned());
    }
    Ok(())
}
