//! CPU contracts for portable variance and bounded working storage.

#[path = "common/numerical_cases.rs"]
pub mod numerical_cases;

use slint as _;
use wgpu as _;

/// CPU uses the same authored kernels as the GPU.
///
/// # Errors
/// Returns runtime, numerical or ownership failures.
#[test]
fn cpu_variance_and_reuse() -> Result<(), String> {
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    numerical_cases::check_variance(&device)?;
    numerical_cases::check_budget_and_reuse(&device, "cpu")
}

/// Budgets reject work before initializing even an unavailable runtime.
///
/// # Panics
/// Panics if a rejected request reaches a runtime or returns the wrong error.
#[test]
fn budgets_fail_before_runtime_initialization() {
    let unavailable = cubecl::Device::Hip(cubecl::device::AmdDevice::default());
    assert_eq!(
        pancake::blur_horizontal_u16_bounded(&unavailable, 5, 1, &[0; 5], 99),
        Err("kernel working-data budget exceeded".to_owned())
    );
    assert_eq!(
        pancake::row_variance_u16(&unavailable, 5, 1, &[0; 5], 109),
        Err("kernel working-data budget exceeded".to_owned())
    );
}
