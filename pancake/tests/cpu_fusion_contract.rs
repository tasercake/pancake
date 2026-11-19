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

/// Check how wider focus windows blend two separated details, including their reflected edges.
/// # Errors
/// Returns a launch, readback or numerical mismatch error.
#[test]
fn weighted_focus_uses_the_requested_window() -> Result<(), String> {
    // The squared Laplacians of these rows are [2.25, 2.25, .5625, 0, 0, 0, 0]
    // and [0, 0, 0, 0, .0625, .25, .25] with reflected edges. The reference
    // below folds those arrays against the identity, five-tap binomial and
    // seven-tap Gaussian (sigma 1.4) stencils, then computes the weighted means.
    let first = [0.125_f32, 0.875, 0.125, 0.125, 0.125, 0.125, 0.125];
    let second = [0.625_f32, 0.625, 0.625, 0.625, 0.625, 0.375, 0.625];
    let one: [f64; 7] = [0.125, 0.875, 0.125, 0.125, 0.625, 0.375, 0.625];
    let five: [f64; 7] = [
        0.125,
        0.875,
        0.127_127_660,
        0.175,
        0.496_428_571,
        0.375,
        0.625,
    ];
    let seven: [f64; 7] = [
        0.125,
        0.874_703_393,
        0.132_032_427,
        0.175,
        0.356_964_188,
        0.353_057_510,
        0.625,
    ];
    let first_rgb = first.map(|value| [value; 3]);
    let second_rgb = second.map(|value| [value; 3]);
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    for (window, expected) in [(1, one), (4, five), (5, five), (7, seven)] {
        let mut accumulator = FocusAccumulator::new(
            &device,
            first.len(),
            1,
            FocusMethod::WeightedAverage,
            window,
            4096,
        )?;
        accumulator.add_frame(first_rgb.as_flattened())?;
        accumulator.add_frame(second_rgb.as_flattened())?;
        let output = accumulator.finish()?;
        if output.len() != first_rgb.as_flattened().len() {
            return Err(format!(
                "focus window {window} returned {} samples",
                output.len()
            ));
        }
        let (pixels, _) = output.as_chunks::<3>();
        for (column, (pixel, reference)) in pixels.iter().zip(expected).enumerate() {
            if pixel.iter().any(|&sample| {
                !sample.is_finite() || (f64::from(sample) - reference).abs() > 1e-6_f64
            }) {
                return Err(format!(
                    "focus window {window} column {column} expected {reference}, got {pixel:?}"
                ));
            }
        }
    }
    Ok(())
}

/// Depth-map center selection compares normalized focus across box scales.
/// # Errors
/// Returns a native launch, readback, or selected source mismatch.
#[test]
fn depth_map_compares_normalized_focus_scales_in_both_axes() -> Result<(), String> {
    // Each isolated impulse supplies one unit of modified-Laplacian energy.
    // At the center all three box widths see the centered impulse's energy,
    // while only the widest box sees both remote impulses. The normalized
    // focus scores are 1/5 + 1/3 + 1/11 and 2/11, respectively.
    let centered: [[f32; 3]; 13] =
        std::array::from_fn(|index| [if index == 6 { 0.375 } else { 0.125 }; 3]);
    let remote: [[f32; 3]; 13] = std::array::from_fn(|index| {
        [if matches!(index, 2 | 10) {
            0.375
        } else {
            0.125
        }; 3]
    });
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    let [row, column] = [(13, 1), (1, 13)].map(|(width, height)| {
        let mut accumulator =
            FocusAccumulator::new(&device, width, height, FocusMethod::DepthMap, 3, 4096)?;
        accumulator.add_frame(remote.as_flattened())?;
        accumulator.add_frame(centered.as_flattened())?;
        let output = accumulator.finish()?;
        if output.len() != centered.as_flattened().len() {
            return Err(format!(
                "depth-map output shape changed for {width} by {height}"
            ));
        }
        let (pixels, _) = output.as_chunks::<3>();
        pixels
            .get(6)
            .copied()
            .ok_or_else(|| "missing focus center".to_owned())
    });
    let (row, column) = (row?, column?);
    let expected = [0.375_f32.to_bits(); 3];
    if row.map(f32::to_bits) == expected && column.map(f32::to_bits) == expected {
        return Ok(());
    }
    Err(format!(
        "depth map selected row {row:?} and column {column:?}, expected the centered source"
    ))
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
