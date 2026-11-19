//! Run independent registration oracles through the production CPU backend.

include!("common/registration_cases.rs");

/// Recover known translation and crop coordinates, and reject featureless motion.
///
/// # Errors
///
/// Returns a device or numerical error.
#[test]
fn cpu_registration_geometry() -> Result<(), String> {
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    crop_and_warp_preserve_rgb_coordinates(&device)?;
    registration_recovers_subpixel_translation(&device)?;
    featureless_pair_has_zero_confidence(&device)?;
    registration_recovers_wide_shift(&device)
}

/// Recover a real centered 3% expansion and cap the supported fit at 1%.
/// # Errors
/// Returns a device, geometry, confidence or scale error.
#[test]
fn cpu_breathing_clamps_a_supported_expansion() -> Result<(), String> {
    const WIDTH: u16 = 321;
    const HEIGHT: u16 = 257;
    const CENTER: [f32; 2] = [160.0, 128.0];
    const SCALE: f32 = 1.03;
    const INVERSE: f32 = 1.0 / SCALE;
    const BUDGET: usize = 16_777_216;
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    let mut reference = Vec::new();
    let mut source = Vec::new();
    for y in (0..HEIGHT).map(f32::from) {
        for x in (0..WIDTH).map(f32::from) {
            reference.extend([scene_sample(x / 2.5, y / 2.5); 3]);
            let original_x = INVERSE.mul_add(x, (1.0 - INVERSE) * CENTER[0]);
            let original_y = INVERSE.mul_add(y, (1.0 - INVERSE) * CENTER[1]);
            source.extend([scene_sample(original_x / 2.5, original_y / 2.5); 3]);
        }
    }
    let prepare = |samples| {
        let [width, height] = [usize::from(WIDTH), usize::from(HEIGHT)];
        thumbnail(
            &device,
            Frame::new(width, height, samples)?,
            width,
            height,
            BUDGET,
        )
    };
    let reference = prepare(&reference)?;
    let source = prepare(&source)?;
    for (maximum, expected, tolerance) in [(0.05, SCALE, 0.002), (0.01, 1.01, 0.000_01)] {
        let estimated = estimate_pair(
            &device,
            &reference,
            &source,
            Model::Breathing,
            maximum,
            true,
        )?;
        let [[xx, xy, tx], [yx, yy, ty]] = estimated.transform.matrix();
        let center_x = xx.mul_add(CENTER[0], xy.mul_add(CENTER[1], tx));
        let center_y = yx.mul_add(CENTER[0], yy.mul_add(CENTER[1], ty));
        if !estimated.confidence.is_finite()
            || estimated.confidence <= 0.0
            || (center_x - CENTER[0])
                .abs()
                .max((center_y - CENTER[1]).abs())
                > 0.5
        {
            return Err(format!(
                "Breathing lost the supported centered fit at {maximum}: {estimated:?}"
            ));
        }
        if (xx - expected).abs().max((yy - expected).abs()) > tolerance {
            return Err(format!(
                "Breathing scale at limit {maximum} must be {expected}: {estimated:?}"
            ));
        }
    }
    Ok(())
}

/// A translated signed shear is reversible in both composition orders.
///
/// # Errors
///
/// Returns a matrix or composition that disagrees with the independent inverse.
#[test]
fn cpu_affine_inverse_reverses_signed_shear() -> Result<(), String> {
    // The linear map has determinant one. Inverting the two linear equations
    // gives x = 3X/4 - Y/2 - 13/4 and y = X/2 + Y + 1/2.
    let forward = Affine::from_matrix([[1.0, 0.5, 3.0], [-0.5, 0.75, -2.0]])?;
    let expected = Affine::from_matrix([[0.75, -0.5, -3.25], [0.5, 1.0, 0.5]])?;
    let inverse = forward.inverse()?;
    let round_trips = [inverse.compose(forward)?, forward.compose(inverse)?];
    if inverse != expected || round_trips != [Affine::IDENTITY; 2] {
        return Err(format!(
            "signed shear inverse did not recover coordinates: {inverse:?}, {round_trips:?}"
        ));
    }
    Ok(())
}

/// Scale displacement is measured from the last pixel center.
///
/// # Errors
///
/// Returns a crop error or a mismatch with the independently derived bounds.
#[test]
fn cpu_scaled_crop_uses_the_last_pixel_center() -> Result<(), String> {
    // The last of 20 centers is x=19. Its 0.988-pixel displacement needs one
    // rounded-up pixel plus one for interpolation. The y axis only needs
    // the interpolation margin because it is unchanged.
    let scale = Affine::from_matrix([[1.052, 0.0, 0.0], [0.0, 1.0, 0.0]])?;
    let crop = symmetric_crop(20, 10, &[scale])?;
    let expected = Bounds::new(2, 1, 16, 8)?;
    if crop != expected {
        return Err(format!(
            "scaled last-pixel crop expected {expected:?}, got {crop:?}"
        ));
    }
    Ok(())
}

/// A correlated source cannot establish motion against a nearly flat reference.
///
/// # Errors
///
/// Returns a device error or an incorrect registration decision.
#[test]
fn cpu_registration_requires_detail_in_the_reference() -> Result<(), String> {
    let (width, height) = (113_u16, 95_u16);
    let (dx, dy) = (3.0_f32, -3.0_f32);
    let (reference, source) = scene_pair(width, height, dx, dy, 1.0);
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    let prepare = |samples| {
        let w = usize::from(width);
        let h = usize::from(height);
        thumbnail(&device, Frame::new(w, h, samples)?, w, h, 2_097_152)
    };
    let estimate = |reference, source| {
        estimate_pair(
            &device,
            &prepare(reference)?,
            &prepare(source)?,
            Model::Translation,
            0.03,
            false,
        )
    };
    let control = estimate(&reference, &source)?;
    let [[_, _, tx], [_, _, ty]] = control.transform.matrix();
    if (tx - dx).abs() > 0.14 || (ty - dy).abs() > 0.14 || control.confidence <= 0.5 {
        return Err(format!(
            "expected textured motion [{dx}, {dy}], got {control:?}"
        ));
    }

    // Both exposures retain the same translated structure, but the reference
    // has only one quarter of the moving image's contrast.
    let near_flat = reference
        .iter()
        .map(|sample| sample * 0.002)
        .collect::<Vec<_>>();
    let brighter = source
        .iter()
        .map(|sample| sample * 0.008)
        .collect::<Vec<_>>();
    let result = estimate(&near_flat, &brighter)?;
    if result.transform != Affine::IDENTITY || result.confidence.abs() >= f32::EPSILON {
        return Err(format!(
            "near-flat reference claimed unsupported motion: {result:?}"
        ));
    }
    Ok(())
}

/// An elongated reference canvas keeps aspect ratio within the thumbnail working budget.
///
/// # Errors
/// Returns a device error or an admission decision inconsistent with the packed proxy dimensions.
#[test]
fn cpu_panorama_thumbnail_respects_proxy_budget() -> Result<(), String> {
    // On an 8192 × 160 canvas, the bounded proxy is 3686 × 72 pixels.
    // Four working f32 samples per proxy pixel, two copies of the 12 input
    // samples and 32 fixed f32 scalars require exactly 4,246,496 bytes.
    const PROXY_BUDGET: usize = 4_246_496;
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    let pixels = [0.5_f32; 12];
    let source = Frame::new(2, 2, &pixels)?;
    let below_budget = thumbnail(&device, source, 8192, 160, 4_246_495);
    if below_budget.err().as_deref() != Some("kernel working-data budget exceeded") {
        return Err("the panorama proxy must reject a budget one byte too small".to_owned());
    }
    let _proxy = thumbnail(&device, source, 8192, 160, PROXY_BUDGET).map_err(|error| {
        format!("the bounded panorama proxy must fit {PROXY_BUDGET} bytes: {error}")
    })?;
    Ok(())
}

/// Preserve every output sample when the final registration tile is partial.
/// # Errors
/// Returns an allocation, device, or sample mismatch error.
#[test]
fn cpu_registration_preserves_partial_final_tile() -> Result<(), String> {
    let (width, height) = (301, 293);
    let pixels = (0_u16..=u16::MAX)
        .cycle()
        .take(264_579)
        .map(f32::from)
        .collect::<Vec<_>>();
    let frame = Frame::new(width, height, &pixels)?;
    let bounds = Bounds::new(0, 0, width, height)?;
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    let result = warp(&device, frame, Affine::IDENTITY, bounds, 8_388_608)?;
    if result != pixels {
        return Err("tiled registration did not preserve the complete RGB frame".to_owned());
    }
    Ok(())
}
