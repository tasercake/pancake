//! Source channel layout contracts at the real image decode boundary.

use std::fs;
use std::io;

use image::{GrayImage, ImageFormat};
use pancake::image_io::decode;

/// Tag a 2D numeric raster and keep the layout of native grayscale and channel arrays.
///
/// # Errors
///
/// Returns a file, decoder or unexpected source layout error.
#[test]
fn identifies_two_dimensional_arrays_without_marking_native_grayscale()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    for (shape, data, expected) in [
        ("(1, 2)", &[1_u8, 17][..], true),
        ("(1, 2, 1)", &[1_u8, 17][..], false),
        ("(1, 2, 3)", &[1_u8, 1, 1, 17, 17, 17][..], false),
    ] {
        let dictionary = format!("{{'descr': '|u1', 'fortran_order': False, 'shape': {shape}, }}");
        let padding = 64_usize.saturating_sub(dictionary.len().saturating_add(11) % 64);
        let dictionary = format!("{dictionary}{}\n", " ".repeat(padding));
        let path = directory.path().join("numeric.npy");
        let mut container = b"\x93NUMPY\x01\x00".to_vec();
        container.extend(u16::try_from(dictionary.len())?.to_le_bytes());
        container.extend_from_slice(dictionary.as_bytes());
        container.extend_from_slice(data);
        fs::write(&path, container)?;
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
