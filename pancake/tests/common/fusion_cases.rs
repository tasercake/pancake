//! Independent numerical expectations for portable focus fusion and resizing.

use pancake::fusion::{FocusAccumulator, FocusMethod, resize, sharpness};

/// Ensure distinct focus planes are both retained and ties follow input order.
/// # Errors
/// Returns launch, readback or numerical contract failures.
pub fn focus_contract(device: &cubecl::Device) -> Result<(), String> {
    let first = [0.0_f32, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
    let second = [1.0_f32, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
    let weighted = combine(device, FocusMethod::WeightedAverage, &first, &second)?;
    if weighted != [0.5; 9] {
        return Err(format!(
            "equal Laplacian weight did not average sources: {weighted:?}"
        ));
    }
    let depth = combine(device, FocusMethod::DepthMap, &first, &second)?;
    if depth != first {
        return Err(format!("depth ties did not keep first source: {depth:?}"));
    }
    let flat = [0.4_f32; 9];
    for method in [FocusMethod::WeightedAverage, FocusMethod::DepthMap] {
        let detailed = combine(device, method, &flat, &first)?;
        if detailed != first {
            return Err(format!(
                "{method:?} discarded the focused source: {detailed:?}"
            ));
        }
        let zero = combine(device, method, &flat, &[0.7; 9])?;
        if zero != flat {
            return Err(format!(
                "{method:?} lost the first source in a flat region: {zero:?}"
            ));
        }
        let gradient = [
            0.10001, 0.23003, 0.37007, 0.20021, 0.34003, 0.51007, 0.70001, 0.80003, 0.87007,
        ];
        let identity = combine(device, method, &gradient, &gradient)?;
        if identity != gradient {
            return Err(format!(
                "{method:?} changed the identical high-precision sources"
            ));
        }
    }
    complementary_planes(device)?;
    sharpness_contract(device)
}

/// Check both focus regions of an independently constructed checker scene.
/// # Errors
/// Returns a runtime error or a boundary pixel that lost focus detail.
fn complementary_planes(device: &cubecl::Device) -> Result<(), String> {
    let scene = (0_usize..33)
        .map(|column| {
            let detail = if column % 2 == 0 { 0.25 } else { 0.75 };
            [detail, detail, detail]
        })
        .collect::<Vec<_>>();
    let left = scene
        .iter()
        .enumerate()
        .flat_map(
            |(column, pixel)| {
                if column < 16 { *pixel } else { [0.5; 3] }
            },
        )
        .collect::<Vec<_>>();
    let right = scene
        .iter()
        .enumerate()
        .flat_map(
            |(column, pixel)| {
                if column >= 16 { *pixel } else { [0.5; 3] }
            },
        )
        .collect::<Vec<_>>();
    for method in [FocusMethod::WeightedAverage, FocusMethod::DepthMap] {
        let mut accumulator = FocusAccumulator::new(device, scene.len(), 1, method, 3, 4096)?;
        accumulator.add_frame(&left)?;
        accumulator.add_frame(&right)?;
        let output = accumulator.finish()?;
        let (pixels, _) = output.as_chunks::<3>();
        for (column, (pixel, expected)) in pixels.iter().zip(&scene).enumerate() {
            if column <= 7 || column >= 24 {
                close(pixel, expected).map_err(|error| {
                    format!("{method:?} focus at column {column}: {error}; {pixels:?}")
                })?;
            }
        }
    }
    Ok(())
}

/// Compare image sharpness to independent analytical squared Laplacian energy.
/// # Errors
/// Returns a runtime error or a result that differs from the analytic energy.
fn sharpness_contract(device: &cubecl::Device) -> Result<(), String> {
    let impulse = [0.0_f32, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0];
    let measured = sharpness(device, 3, 1, &impulse, 4096)?;
    close(&[measured], &[4.0])?;

    // Reflect-101 at both borders gives Laplacians [-4, 2, 2, 0].
    // Their squared energies sum to 24 over the four pixels.
    let corner = [1.0_f32, 0.0, 0.0, 0.0].map(|sample| [sample; 3]);
    let measured = sharpness(device, 2, 2, corner.as_flattened(), 4096)?;
    close(&[measured], &[6.0])
        .map_err(|_| format!("2×2 corner impulse energy expected 6, got {measured}"))?;

    let flat = sharpness(device, 3, 1, &[0.27; 9], 4096)?;
    close(&[flat], &[0.0])
}

/// Verify pixel-center interpolation, reduction footprints and channel order.
/// # Errors
/// Returns launch, readback or numerical contract failures.
pub fn resize_contract(device: &cubecl::Device) -> Result<(), String> {
    let source = [0.0_f32, 0.2, 0.4, 1.0, 0.2, 0.4, 0.0, 0.2, 0.4];
    let expanded = resize(device, [3, 1], [5, 1], &source, 4096)?;
    let expected = [
        0.0, 0.2, 0.4, 0.4, 0.2, 0.4, 1.0, 0.2, 0.4, 0.4, 0.2, 0.4, 0.0, 0.2, 0.4,
    ];
    close(&expanded, &expected)?;
    let reduced = resize(device, [3, 1], [1, 1], &source, 4096)?;
    close(&reduced, &[1.0 / 3.0, 0.2, 0.4])?;
    let tall = resize(device, [1, 1], [1, 3], &[0.11, 0.55, 1.2], 4096)?;
    close(&tall, &[0.11, 0.55, 1.2, 0.11, 0.55, 1.2, 0.11, 0.55, 1.2])?;
    let reduced_y = resize(device, [1, 3], [1, 1], &source, 4096)?;
    close(&reduced_y, &[1.0 / 3.0, 0.2, 0.4])?;
    let identical = resize(device, [3, 1], [3, 1], &source, 4096)?;
    close(&identical, &source)?;
    let grid = [
        0.0_f32, 0.0, 0.0, 1.0, 1.0, 1.0, 0.5, 0.5, 0.5, 0.25, 0.25, 0.25,
    ];
    let mixed = resize(device, [2, 2], [3, 1], &grid, 4096)?;
    close(
        &mixed,
        &[
            0.25, 0.25, 0.25, 0.4375, 0.4375, 0.4375, 0.625, 0.625, 0.625,
        ],
    )
}

/// Verify exactly two inputs against the same supported runtime.
/// # Errors
/// Returns a shape, budget, launch, or readback error.
fn combine(
    device: &cubecl::Device,
    method: FocusMethod,
    first: &[f32],
    second: &[f32],
) -> Result<Vec<f32>, String> {
    let mut accumulator = FocusAccumulator::new(device, 3, 1, method, 3, 4096)?;
    accumulator.add_frame(first)?;
    accumulator.add_frame(second)?;
    accumulator.finish()
}

/// Compare element counts and samples with an absolute f32 rounding budget.
/// # Errors
/// Returns a detailed error for an incorrect shape or sample.
fn close(actual: &[f32], expected: &[f32]) -> Result<(), String> {
    if actual.len() != expected.len()
        || actual
            .iter()
            .zip(expected)
            .any(|(a, b)| (*a - *b).abs() > 0.000_001)
    {
        return Err(format!("resize expected {expected:?}, got {actual:?}"));
    }
    Ok(())
}
