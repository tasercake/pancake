// Independent registration and resampling contracts.

use pancake::registration::{
    Affine, Bounds, Estimate, Frame, Model, estimate_pair, symmetric_crop, thumbnail, warp,
};

/// A coordinate ramp exposes split-axis interpolation, crop origins and border policy.
///
/// # Errors
///
/// Returns a device, geometry or numerical contract failure.
fn crop_and_warp_preserve_rgb_coordinates(device: &cubecl::Device) -> Result<(), String> {
    let ramp: Vec<f32> = (0_u16..60_u16).map(f32::from).collect();
    let frame = Frame::new(5, 4, &ramp)?;
    let bounds = Bounds::new(1, 1, 2, 2)?;
    let transform = Affine::from_matrix([[1.0, 0.0, 0.5], [0.0, 1.0, -0.25]])?;
    let result = warp(device, frame, transform, bounds, 8192)?;
    let expected = [
        15.75, 16.75, 17.75, 18.75, 19.75, 20.75, 30.75, 31.75, 32.75, 33.75, 34.75, 35.75,
    ];
    if result != expected {
        return Err(format!(
            "crop coordinates produced {result:?}, expected {expected:?}"
        ));
    }
    let crop = symmetric_crop(25, 23, &[Affine::IDENTITY, transform])?;
    if crop != Bounds::new(2, 2, 21, 19)? {
        return Err(format!(
            "symmetric crop lacks expected interpolation margin: {crop:?}"
        ));
    }
    Ok(())
}

/// Recover a known displacement from two analytical samplings of the same scene.
///
/// # Errors
///
/// Returns a device, geometry, or numerical contract failure.
fn registration_recovers_subpixel_translation(device: &cubecl::Device) -> Result<(), String> {
    let (width, height) = (113_u16, 95_u16);
    let (reference, source) = scene_pair(width, height, 3.25, -1.5, 1.0);
    let make = |pixels| {
        let w = usize::from(width);
        let h = usize::from(height);
        thumbnail(device, Frame::new(w, h, pixels)?, w, h, 2_097_152)
    };
    let reference = make(&reference)?;
    let source = make(&source)?;
    let Estimate {
        transform,
        confidence,
    } = estimate_pair(device, &reference, &source, Model::Translation, 0.03, false)?;
    let [[xx, xy, tx], [yx, yy, ty]] = transform.matrix();
    if [(xx - 1.0).abs(), xy.abs(), yx.abs(), (yy - 1.0).abs()]
        .into_iter()
        .any(|error| error > f32::EPSILON)
    {
        return Err(format!(
            "translation changed its linear part: {transform:?}"
        ));
    }
    if (tx - 3.25).abs() > 0.14 || (ty + 1.5).abs() > 0.14 || confidence <= 0.5 {
        return Err(format!(
            "expected [3.25, -1.5], found [{tx}, {ty}] at {confidence} confidence"
        ));
    }
    Ok(())
}

/// A featureless pair cannot justify any estimated motion.
///
/// # Errors
///
/// Returns a device, geometry, or numerical contract failure.
fn featureless_pair_has_zero_confidence(device: &cubecl::Device) -> Result<(), String> {
    let samples = vec![0.75_f32; 17 * 19 * 3];
    let frame = Frame::new(17, 19, &samples)?;
    let prepared = thumbnail(device, frame, 17, 19, 65536)?;
    let estimate = estimate_pair(device, &prepared, &prepared, Model::Breathing, 0.04, true)?;
    if estimate.transform != Affine::IDENTITY || estimate.confidence.abs() >= f32::EPSILON {
        return Err(format!(
            "featureless pair suggested unsupported motion: {estimate:?}"
        ));
    }
    Ok(())
}

/// Sample a smooth non-periodic scene at a fixed subpixel translation.
fn scene_pair(
    width: u16,
    height: u16,
    dx: f32,
    dy: f32,
    texture_scale: f32,
) -> (Vec<f32>, Vec<f32>) {
    let sample = |x: f32, y: f32| {
        let weight_x = (0.017 * y).mul_add(y, 0.11 * x).sin();
        let weight_y = (0.011 * x).mul_add(x, 0.16 * y).cos();
        let coarse = 0.27_f32.mul_add(y, 0.31 * x).cos();
        let texture = 0.19_f32.mul_add(-y, 0.13 * x).sin();
        (0.22 * weight_x).mul_add(
            weight_y,
            0.14_f32.mul_add(coarse, 0.06_f32.mul_add(texture, 0.5)),
        )
    };
    let mut reference = Vec::new();
    let mut source = Vec::new();
    for y in (0..height).map(f32::from) {
        for x in (0..width).map(f32::from) {
            reference.extend([sample(x / texture_scale, y / texture_scale); 3]);
            source.extend([sample((x - dx) / texture_scale, (y - dy) / texture_scale); 3]);
        }
    }
    (reference, source)
}

/// A textured source can move beyond the local search radius.
///
/// # Errors
///
/// Returns a device error or an incorrect coarse translation.
fn registration_recovers_wide_shift(device: &cubecl::Device) -> Result<(), String> {
    let (width, height) = (321_u16, 257_u16);
    let (dx, dy) = (37.25_f32, -25.5_f32);
    let (reference, source) = scene_pair(width, height, dx, dy, 2.5);
    let make = |pixels| {
        let w = usize::from(width);
        let h = usize::from(height);
        thumbnail(device, Frame::new(w, h, pixels)?, w, h, 16_777_216)
    };
    let estimate = estimate_pair(
        device,
        &make(&reference)?,
        &make(&source)?,
        Model::Translation,
        0.03,
        true,
    )?;
    let [[_, _, tx], [_, _, ty]] = estimate.transform.matrix();
    if (tx - dx).abs() > 0.5 || (ty - dy).abs() > 0.5 {
        return Err(format!(
            "wide shift: expected [{dx}, {dy}], found [{tx}, {ty}]"
        ));
    }
    Ok(())
}
