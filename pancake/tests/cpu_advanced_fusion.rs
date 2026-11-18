//! Independent CPU contracts for multiscale focus and exposure fusion.

use pancake::fusion::advanced::{
    AdvancedAccumulator, AdvancedMethod, automatic_parameters, slab_ranges,
};
use wgpu as _;

#[path = "common/advanced_fusion_cases.rs"]
pub mod advanced_fusion_cases;

use advanced_fusion_cases::{BUDGET, HEIGHT, WIDTH};

/// Run the same independent cases as the physical GPU.
///
/// # Errors
/// Returns a numerical or compute error.
#[test]
fn cpu_advanced_fusion() -> Result<(), String> {
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    advanced_fusion_cases::check_constant(&device)?;
    advanced_fusion_cases::check_focus(&device)?;
    advanced_fusion_cases::check_exposure_batches(&device)?;
    advanced_fusion_cases::check_exposure_precision(&device)
}

/// Invalid dimensions and budget fail without touching the unavailable runtime.
///
/// # Panics
/// Panics if a malformed request is accepted or reports a different error.
#[test]
fn cpu_fusion_resource_limits_precede_runtime_access() {
    let unavailable = cubecl::Device::Hip(cubecl::device::AmdDevice::default());
    let zero = AdvancedAccumulator::new(
        &unavailable,
        0,
        HEIGHT,
        AdvancedMethod::Hybrid,
        3,
        4,
        BUDGET,
    );
    assert!(
        matches!(zero, Err(error) if error.contains("dimensions")),
        "empty image was accepted"
    );
    let tiny = AdvancedAccumulator::new(
        &unavailable,
        WIDTH,
        HEIGHT,
        AdvancedMethod::Exposure,
        3,
        4,
        1,
    );
    assert!(
        matches!(tiny, Err(error) if error.contains("budget")),
        "fusion exceeded its requested budget"
    );
}

/// Slabs cover ordered inputs and retain overlap, including a short final slab.
///
/// # Errors
/// Returns a slab or automatic-parameter validation error.
#[test]
fn cpu_slab_geometry_and_scale_aware_parameters() -> Result<(), String> {
    if slab_ranges(9, 4, 1)? != [0..4, 3..7, 6..9] {
        return Err("incorrect overlapping slabs".to_owned());
    }
    if slab_ranges(9, 0, 0)? != std::iter::once(0..9).collect::<Vec<_>>() {
        return Err("disabled slabs changed frame order".to_owned());
    }
    if !matches!(slab_ranges(9, 4, 4), Err(error) if error.contains("overlap")) {
        return Err("nonadvancing overlap was accepted".to_owned());
    }
    if !matches!(slab_ranges(1, 1, 0), Err(error) if error.contains("size")) {
        return Err("invalid one-frame slab size was accepted".to_owned());
    }
    let small = automatic_parameters(WIDTH, HEIGHT, 9)?;
    let large = automatic_parameters(6000, 4000, 50)?;
    if small.kernel_size >= large.kernel_size || small.pyramid_levels >= large.pyramid_levels {
        return Err("automatic tuning ignored stack dimensions".to_owned());
    }
    Ok(())
}
