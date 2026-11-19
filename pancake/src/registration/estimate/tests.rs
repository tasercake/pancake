//! Native numerical contracts for registration parameter conversion.

use super::{Affine, Coordinates, Extent, Model, Parameters, is_plausible};

/// A Euclidean candidate rotates in pixel-center coordinates without scaling.
///
/// # Errors
///
/// Returns invalid geometry or a failure of the independent rigid-map checks.
#[test]
fn euclidean_parameters_produce_a_rigid_map() -> Result<(), String> {
    const CENTER: [f64; 2] = [64.0, 48.0];
    const TRANSLATED_CENTER: [f64; 2] = [67.25, 46.5];

    // 24² + 143² = 145², so these integer ratios describe a known unit rotation.
    let expected_cosine = 143.0_f64 / 145.0_f64;
    let expected_sine = 24.0_f64 / 145.0_f64;
    let angle = 24.0_f32.atan2(143.0_f32);
    let extent = Extent::new(129, 97)?;
    let transform = Coordinates::new(extent).convert(
        Model::Euclidean,
        Parameters([3.25, -1.5, angle * 64.0, 0.0, 0.0, 0.0]),
    )?;
    let [[a, b, tx], [c, d, ty]] = transform.matrix().map(|row| row.map(f64::from));

    let gram_errors = [
        a.mul_add(a, c * c) - 1.0_f64,
        b.mul_add(b, d * d) - 1.0_f64,
        a.mul_add(b, c * d),
        a.mul_add(d, -(b * c)) - 1.0_f64,
    ];
    if gram_errors
        .into_iter()
        .any(|error| error.abs() > 0.000_002_f64)
    {
        return Err(format!(
            "Euclidean matrix is not a rigid rotation: {transform:?}"
        ));
    }

    // Verify the native center and two basis points against independently rotated offsets.
    let [center_x, center_y] = CENTER;
    let [translated_x, translated_y] = TRANSLATED_CENTER;
    let points: [([f64; 2], [f64; 2]); 3] = [
        (CENTER, TRANSLATED_CENTER),
        (
            [96.0, 48.0],
            [
                32.0_f64.mul_add(expected_cosine, translated_x),
                32.0_f64.mul_add(expected_sine, translated_y),
            ],
        ),
        (
            [64.0, 72.0],
            [
                24.0_f64.mul_add(-expected_sine, translated_x),
                24.0_f64.mul_add(expected_cosine, translated_y),
            ],
        ),
    ];
    for ([x, y], [u, v]) in points {
        let mapped_x = a.mul_add(x, b.mul_add(y, tx));
        let mapped_y = c.mul_add(x, d.mul_add(y, ty));
        // Recover the original position using the independently specified transpose.
        let local_x = mapped_x - translated_x;
        let local_y = mapped_y - translated_y;
        let recovered_x =
            expected_cosine.mul_add(local_x, expected_sine.mul_add(local_y, center_x));
        let recovered_y =
            (-expected_sine).mul_add(local_x, expected_cosine.mul_add(local_y, center_y));
        if [mapped_x - u, mapped_y - v, recovered_x - x, recovered_y - y]
            .into_iter()
            .any(|error| error.abs() > 0.000_02_f64)
        {
            return Err(format!(
                "Euclidean map did not rotate and recover [{x}, {y}]: {transform:?}"
            ));
        }
    }
    Ok(())
}

/// Reject translation beyond the entire scale of the search canvas on either axis.
///
/// # Errors
///
/// Returns invalid fixture geometry or a failed motion plausibility check.
#[test]
fn plausibility_rejects_translation_outside_search_radius() -> Result<(), String> {
    const RADIUS: f32 = 100.0;
    let translation = |x, y| Affine::from_matrix([[1.0, 0.0, x], [0.0, 1.0, y]]);
    let ordinary = translation(20.0, -25.0)?;
    if !is_plausible(ordinary, RADIUS) {
        return Err("ordinary rigid translation was rejected".to_owned());
    }
    for [x, y] in [[125.0_f32, 0.0_f32], [0.0_f32, -125.0_f32]] {
        if is_plausible(translation(x, y)?, RADIUS) {
            return Err(format!(
                "translation [{x}, {y}] exceeded the search radius {RADIUS} but was accepted"
            ));
        }
    }
    Ok(())
}
