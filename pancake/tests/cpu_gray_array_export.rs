//! Grayscale array export keeps the original channel layout at the file boundary.

use std::{fs, io};

use image::{ColorType, GrayImage, ImageFormat};
use pancake::config::BitDepth;
use pancake::image_io::{decode, export};

/// A real 2D float `NumPy` image exports integer pixels as one channel.
///
/// # Errors
/// Returns a fixture, codec, source immutability, or pixel representation failure.
#[test]
fn npy_gray_array_preserves_single_channel_and_depth() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("grayscale.npy");
    let header = "{'descr': '<f4', 'fortran_order': False, 'shape': (2, 2), }";
    let padding = 64_usize.saturating_sub(header.len().saturating_add(11) % 64);
    let header = format!("{header}{}\n", " ".repeat(padding));
    let mut npy = b"\x93NUMPY\x01\x00".to_vec();
    npy.extend(u16::try_from(header.len())?.to_le_bytes());
    npy.extend_from_slice(header.as_bytes());
    for sample in [13.0_f32, 201.0, 88.0, 255.0] {
        npy.extend(sample.to_le_bytes());
    }
    fs::write(&source, &npy)?;
    let frame = decode(&source)?;
    let sources = [source.clone()];
    for extension in ["png", "tiff"] {
        let eight = directory.path().join(format!("gray8.{extension}"));
        export(&eight, &frame, BitDepth::Eight, 100, &sources)?;
        let image = image::open(&eight)?;
        if image.color() != ColorType::L8 || image.into_luma8().into_raw() != [13, 201, 88, 255] {
            return Err(io::Error::other(format!(
                "{extension} must preserve exact 8-bit grayscale samples"
            ))
            .into());
        }

        let sixteen = directory.path().join(format!("gray16.{extension}"));
        export(&sixteen, &frame, BitDepth::Sixteen, 100, &sources)?;
        let image = image::open(&sixteen)?;
        if image.color() != ColorType::L16
            || image.into_luma16().into_raw() != [3341, 51657, 22616, 65535]
        {
            return Err(io::Error::other(format!(
                "{extension} must preserve exact 16-bit grayscale samples"
            ))
            .into());
        }
    }
    let jpeg = directory.path().join("gray.jpg");
    export(&jpeg, &frame, BitDepth::Eight, 100, &sources)?;
    let image = image::open(&jpeg)?;
    if image.color() != ColorType::L8 || (image.width(), image.height()) != (2, 2) {
        return Err(io::Error::other("JPEG must retain grayscale structure").into());
    }
    let exr = directory.path().join("frame.exr");
    export(&exr, &frame, BitDepth::Sixteen, 100, &sources)?;
    if image::open(&exr)?.color() != ColorType::Rgb32F {
        return Err(io::Error::other("EXR must retain float RGB samples").into());
    }
    if fs::read(&source)? != npy {
        return Err(io::Error::other("export altered its NumPy input").into());
    }
    Ok(())
}

/// Grayscale from native image files still follows the application's RGB export policy.
///
/// # Errors
/// Returns a fixture, codec, or output layout failure.
#[test]
fn native_grayscale_still_exports_rgb() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("mono.png");
    let native = GrayImage::from_raw(2, 1, vec![13, 201])
        .ok_or_else(|| io::Error::other("invalid monochrome fixture"))?;
    native.save_with_format(&source, ImageFormat::Png)?;
    let frame = decode(&source)?;
    let output = directory.path().join("frame.png");
    export(&output, &frame, BitDepth::Eight, 95, &[source])?;
    let image = image::open(output)?;
    if image.color() != ColorType::Rgb8
        || image.into_rgb8().into_raw() != [13, 13, 13, 201, 201, 201]
    {
        return Err(io::Error::other("native grayscale PNG export must remain RGB").into());
    }
    Ok(())
}
