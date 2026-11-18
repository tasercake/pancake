//! Preserve the developed RAW RGB16 boundary across packing changes.

use std::error::Error;

use image::metadata::Orientation;
use rawler::imgop::develop::Intermediate;
use rawler::pixarray::Color2D;

use super::{developed_raster, raster_to_rgb};

/// Asymmetric RGB triples with explicit expected unsigned code values.
const SAMPLE_CODES: [(f32, u16); 18] = [
    (-0.25, 0),
    (-0.0, 0),
    (0.0, 0),
    (f32::from_bits(1), 0),
    (0.5 / 65_535.0, 0),
    (1.5 / 65_535.0, 1),
    (0.25, 16_383),
    (0.5, 32_767),
    (0.75, 49_151),
    (1.0, u16::MAX),
    (1.25, u16::MAX),
    (f32::INFINITY, u16::MAX),
    (f32::NEG_INFINITY, 0),
    (f32::NAN, 0),
    (32_769.5 / 65_535.0, 32_769),
    (0.125, 8_191),
    (0.625, 40_959),
    (0.875, 57_343),
];

/// Make the same fully developed pixels for both quantization paths.
fn fixture() -> Intermediate {
    let values = SAMPLE_CODES.map(|(sample, _)| sample);
    Intermediate::ThreeColor(Color2D::new_with(values.as_chunks::<3>().0.to_vec(), 3, 2))
}

/// Preserve truncation, saturation, component order and RGB16 image type.
/// # Errors
/// Returns a missing raster, incorrect image type, or numerical mismatch.
#[test]
fn packed_rgb16_matches_rawler_quantization_and_known_codes() -> Result<(), Box<dyn Error>> {
    let baseline = fixture()
        .to_dynamic_image()
        .ok_or("missing Rawler raster")?;
    let packed = developed_raster(fixture()).ok_or("missing packed raster")?;
    let expected = baseline.as_rgb16().ok_or("expected Rawler RGB16")?;
    let actual = packed.as_rgb16().ok_or("expected packed RGB16")?;
    if expected.dimensions() != actual.dimensions()
        || actual.as_raw() != expected.as_raw()
        || !actual
            .as_raw()
            .iter()
            .copied()
            .eq(SAMPLE_CODES.map(|(_, code)| code))
    {
        return Err(format!("incorrect packed RGB16 codes: {:?}", actual.as_raw()).into());
    }
    Ok(())
}

/// Preserve all EXIF orientations and the final working f32 values bit for bit.
/// # Errors
/// Returns invalid orientation, raster conversion, or working-frame mismatch.
#[test]
fn packed_raw_preserves_oriented_working_samples() -> Result<(), Box<dyn Error>> {
    for code in 1_u8..=8 {
        let orientation = Orientation::from_exif(code).ok_or("invalid EXIF test orientation")?;
        let mut expected = fixture()
            .to_dynamic_image()
            .ok_or("missing Rawler raster")?;
        let mut actual = developed_raster(fixture()).ok_or("missing packed raster")?;
        expected.apply_orientation(orientation);
        actual.apply_orientation(orientation);

        let expected = raster_to_rgb(expected, None, None)?;
        let actual = raster_to_rgb(actual, None, None)?;
        if (
            expected.width(),
            expected.height(),
            expected.sample_encoding(),
        ) != (actual.width(), actual.height(), actual.sample_encoding())
            || !expected
                .samples()
                .iter()
                .copied()
                .map(f32::to_bits)
                .eq(actual.samples().iter().copied().map(f32::to_bits))
        {
            return Err(format!("working RAW samples differ for EXIF {code}").into());
        }
    }
    Ok(())
}
