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
    advanced_fusion_cases::check_hybrid_level_bounds(&device)?;
    advanced_fusion_cases::check_hybrid_frame_launches(&device)?;
    advanced_fusion_cases::check_exposure_batches(&device)?;
    advanced_fusion_cases::check_exposure_precision(&device)?;
    advanced_fusion_cases::check_exposure_specialization(&device)
}

/// Keep selected detail at two spatial scales when sources have equal focus.
///
/// # Errors
/// Returns a numerical, geometry, or runtime failure.
#[test]
fn cpu_hybrid_retains_selected_fine_detail_across_scales() -> Result<(), String> {
    const STRIP_WIDTH: usize = 64;
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    let second_difference = |&[left, _, _, center, _, _, right, _, _]: &[f32; 9]| {
        2.0_f64.mul_add(-f64::from(center), f64::from(left) + f64::from(right))
    };
    let mut failures = Vec::new();

    // Phase opposition gives equal focus magnitude and cancels the signed
    // detail in the weighted mean. Two- and four-pixel periods disappear
    // after one and two Gaussian reductions respectively, away from edges.
    let patterns: [&[f32]; 2] = [&[0.0, 1.0], &[0.0, 0.5, 1.0, 0.5]];
    for pattern in patterns {
        let first: Vec<_> = pattern
            .iter()
            .copied()
            .cycle()
            .take(STRIP_WIDTH)
            .flat_map(|sample| [sample; 3])
            .collect();
        let opposite: Vec<_> = first.iter().map(|sample| 1.0 - sample).collect();
        let mut accumulator = AdvancedAccumulator::new(
            &device,
            STRIP_WIDTH,
            1,
            AdvancedMethod::Hybrid,
            6,
            3,
            BUDGET,
        )?;
        accumulator.add_frame(&first)?;
        accumulator.add_frame(&opposite)?;
        let output = accumulator.finish()?;
        if output.len() != first.len() {
            return Err("Hybrid changed the dimensions of the detail strip".to_owned());
        }

        let (mut source_power, mut fused_power, mut covariance) = (0.0_f64, 0.0_f64, 0.0_f64);
        // Center x=24..40 stays clear of both image edges through three reductions.
        for (source, fused) in first
            .array_windows::<9>()
            .zip(output.array_windows::<9>())
            .step_by(3)
            .skip(23)
            .take(16)
        {
            let source = second_difference(source);
            let fused = second_difference(fused);
            source_power = source.mul_add(source, source_power);
            fused_power = fused.mul_add(fused, fused_power);
            covariance = fused.mul_add(source, covariance);
        }

        // The public numerical focus oracle requires 65% of true Laplacian
        // variance. Whole periods have zero mean interior second difference.
        // Positive covariance also verifies deterministic earlier-source detail.
        if source_power <= 0.0_f64 || !fused_power.is_finite() {
            return Err("Hybrid detail fixture has invalid second differences".to_owned());
        }
        let retained = fused_power / source_power;
        if retained < 0.65_f64 || covariance <= 0.0_f64 {
            failures.push(format!(
                "{}-pixel period retained {retained:.3} of detail power (covariance {covariance:.3})",
                pattern.len()
            ));
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join(", "))
    }
}

/// In-phase blue detail strengthens the luminance contrast of a red pattern.
///
/// # Errors
/// Returns a device error or an incorrect focus winner.
#[test]
fn cpu_laplacian_prefers_in_phase_blue_detail() -> Result<(), String> {
    let (width, height) = (9_usize, 7_usize);
    let mut red = Vec::new();
    let mut red_blue = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let sample = if x % 2 == y % 2 { 0.25_f32 } else { 0.75 };
            red.extend([sample, 0.5, 0.5]);
            red_blue.extend([sample, 0.5, sample]);
        }
    }

    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    for (first, second) in [(&red, &red_blue), (&red_blue, &red)] {
        // With no reductions, the winning focus coefficient is the input RGB.
        let mut accumulator = AdvancedAccumulator::new(
            &device,
            width,
            height,
            AdvancedMethod::Laplacian,
            3,
            0,
            BUDGET,
        )?;
        accumulator.add_frame(first)?;
        accumulator.add_frame(second)?;
        let output = accumulator.finish()?;
        if output != red_blue {
            return Err(format!(
                "expected the stronger red-and-blue detail, got first pixel {:?}",
                output.get(..3)
            ));
        }
    }
    Ok(())
}

/// Reject malformed caller samples before accepting the next complete frame.
/// # Errors
/// Returns a numerical, validation, or runtime error.
#[test]
fn cpu_advanced_host_frame_validation_preserves_recovery() -> Result<(), String> {
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    let mut accumulator =
        AdvancedAccumulator::new(&device, 3, 1, AdvancedMethod::Hybrid, 3, 2, BUDGET)?;
    let valid = [0.625; 9];
    let mut nonfinite = valid;
    *nonfinite
        .first_mut()
        .ok_or("expected a nonempty RGB frame")? = f32::NAN;
    if !matches!(accumulator.add_frame(&[0.0]), Err(error) if error == "sample count does not match packed RGB dimensions")
        || !matches!(accumulator.add_frame(&nonfinite), Err(error) if error == "image contains a nonfinite sample")
    {
        return Err("advanced fusion did not reject malformed host samples".to_owned());
    }
    accumulator.add_frame(&valid)?;
    if accumulator.finish()? != valid {
        return Err("rejected samples changed the accepted Hybrid result".to_owned());
    }
    Ok(())
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

/// Preserve the same two-frame Hybrid result after exchanging image axes.
///
/// # Errors
/// Returns incorrect fusion geometry or samples, or a runtime failure.
#[test]
fn cpu_hybrid_commutes_with_transposition() -> Result<(), String> {
    const SIDE: usize = 8;
    const ROW_SAMPLES: usize = SIDE * 3;
    const ONE_RGB16_CODE: f32 = 1.0_f32 / 65_535.0_f32;

    // Each opposing pair has equal focus magnitude but different signed
    // detail. The extra bright row varies the low-frequency tone as well.
    let stripe = [0.25_f32, 0.75, 0.25, 0.75, 0.75, 0.25, 0.75, 0.25];
    let row_stripes: Vec<_> = stripe
        .into_iter()
        .flat_map(|value| [value; ROW_SAMPLES])
        .collect();
    let column_stripes: Vec<_> = std::iter::repeat_n(stripe, SIDE)
        .flatten()
        .flat_map(|value| [value; 3])
        .collect();
    let device = cubecl::Device::Cpu(cubecl::cpu::CpuDevice);
    let fuse = |first: &[f32]| -> Result<Vec<f32>, String> {
        let second: Vec<_> = first.iter().map(|value| 1.0_f32 - value).collect();
        let mut accumulator =
            AdvancedAccumulator::new(&device, SIDE, SIDE, AdvancedMethod::Hybrid, 3, 1, BUDGET)?;
        accumulator.add_frame(first)?;
        accumulator.add_frame(&second)?;
        let fused = accumulator.finish()?;
        if fused.len() != first.len() {
            return Err("Hybrid changed the square output extent".to_owned());
        }
        for input in [first, second.as_slice()] {
            if fused
                .iter()
                .zip(input)
                .all(|(actual, input)| (actual - input).abs() <= ONE_RGB16_CODE)
            {
                return Err("Hybrid did not combine the two focus sources".to_owned());
            }
        }
        Ok(fused)
    };
    let horizontal = fuse(&row_stripes)?;
    let vertical = fuse(&column_stripes)?;
    let (horizontal_rows, _) = horizontal.as_chunks::<ROW_SAMPLES>();
    let (vertical_rows, _) = vertical.as_chunks::<ROW_SAMPLES>();
    let mut max_error = 0.0_f32;
    for (y, row) in horizontal_rows.iter().enumerate() {
        let (pixels, _) = row.as_chunks::<3>();
        for (x, pixel) in pixels.iter().enumerate() {
            let row = vertical_rows.get(x).ok_or("missing transposed row")?;
            let (transposed, _) = row.as_chunks::<3>();
            let expected = transposed.get(y).ok_or("missing transposed pixel")?;
            for (actual, expected) in pixel.iter().zip(expected) {
                max_error = max_error.max((actual - expected).abs());
            }
        }
    }
    if max_error > ONE_RGB16_CODE {
        return Err(format!(
            "transposing the focus sources changed Hybrid by {max_error:.6}"
        ));
    }
    Ok(())
}
