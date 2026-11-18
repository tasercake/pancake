//! Source channel layout contracts at the real image decode boundary.

use std::fs;
use std::io;
use std::path::Path;

use image::{ColorType, GrayImage, ImageFormat};
use pancake::config::BitDepth;
use pancake::image_io::{decode, export};

/// A native layout contract failure or fixture setup error.
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Write a version one scalar or channel array with the supplied bytes.
///
/// # Errors
/// Returns a header-length or write failure.
fn write_array(path: &Path, dtype: &str, shape: &str, bytes: &[u8]) -> TestResult {
    let dictionary = format!("{{'descr': '{dtype}', 'fortran_order': False, 'shape': {shape}, }}");
    let padding = 63_usize.saturating_sub(dictionary.len().saturating_add(10) % 64);
    let dictionary = format!("{dictionary}{}\n", " ".repeat(padding));
    let mut container = b"\x93NUMPY\x01\x00".to_vec();
    container.extend(u16::try_from(dictionary.len())?.to_le_bytes());
    container.extend_from_slice(dictionary.as_bytes());
    container.extend_from_slice(bytes);
    fs::write(path, container)?;
    Ok(())
}

/// Tag a 2D numeric raster and keep the layout of native grayscale and channel arrays.
///
/// # Errors
///
/// Returns a file, decoder or unexpected source layout error.
#[test]
fn identifies_two_dimensional_arrays_without_marking_native_grayscale() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (shape, data, expected) in [
        ("(1, 2)", &[1_u8, 17][..], true),
        ("(1, 2, 1)", &[1_u8, 17][..], false),
        ("(1, 2, 3)", &[1_u8, 1, 1, 17, 17, 17][..], false),
    ] {
        let path = directory.path().join("numeric.npy");
        write_array(&path, "|u1", shape, data)?;
        let frame = decode(&path)?;
        if frame.is_grayscale_array() != expected {
            return Err(io::Error::other(format!("unexpected array metadata for {shape}")).into());
        }
        let samples = [1_u8, 1, 1, 17, 17, 17].map(|sample| f32::from(sample) / 255.0);
        if !frame
            .samples()
            .iter()
            .copied()
            .zip(samples)
            .all(|(sample, expected)| (sample - expected).abs() < 1e-6)
        {
            return Err(io::Error::other("array metadata altered RGB processing samples").into());
        }
    }
    let native = GrayImage::from_raw(2, 1, vec![1, 17])
        .ok_or_else(|| io::Error::other("invalid grayscale fixture"))?;
    let path = directory.path().join("native.png");
    native.save_with_format(&path, ImageFormat::Png)?;
    if decode(&path)?.is_grayscale_array() {
        return Err(io::Error::other("native grayscale must export RGB by default").into());
    }
    Ok(())
}

/// Export two-dimensional arrays with genuine one-channel samples in integer formats.
///
/// # Errors
/// Returns a source setup, output codec, or channel precision failure.
#[test]
fn grayscale_array_exports_keep_native_sample_layout() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("gray.npy");
    let original = [0_u16, 32768, 65535, 257];
    let bytes: Vec<u8> = original.into_iter().flat_map(u16::to_le_bytes).collect();
    write_array(&source, "<u2", "(2, 2)", &bytes)?;
    let frame = decode(&source)?;
    for name in ["gray.png", "gray.tif"] {
        let path = directory.path().join(name);
        export(&path, &frame, BitDepth::Sixteen, 90, &[])?;
        let restored = image::open(&path)?;
        if restored.color() != ColorType::L16 || restored.into_luma16().into_raw() != original {
            return Err(io::Error::other(format!("{name} changed 16-bit gray samples")).into());
        }
        export(&path, &frame, BitDepth::Eight, 90, &[])?;
        let restored = image::open(&path)?;
        if restored.color() != ColorType::L8 || restored.into_luma8().into_raw() != [0, 128, 255, 1]
        {
            return Err(
                io::Error::other(format!("{name} changed rounded 8-bit gray samples")).into(),
            );
        }
    }
    let jpeg = directory.path().join("gray.jpg");
    export(&jpeg, &frame, BitDepth::Eight, 100, &[])?;
    let restored = image::open(&jpeg)?;
    if restored.color() != ColorType::L8 || (restored.width(), restored.height()) != (2, 2) {
        return Err(io::Error::other("JPEG lost grayscale layout or geometry").into());
    }
    let exr = directory.path().join("gray.exr");
    export(&exr, &frame, BitDepth::Sixteen, 90, &[])?;
    let restored = image::open(&exr)?;
    if restored.color() != ColorType::Rgb32F
        || !restored
            .into_rgb32f()
            .into_raw()
            .into_iter()
            .map(f32::to_bits)
            .eq(frame.samples().iter().copied().map(f32::to_bits))
    {
        return Err(io::Error::other("grayscale array EXR lost RGB float sample precision").into());
    }
    Ok(())
}
