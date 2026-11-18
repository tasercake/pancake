//! Application image input contracts with real temporary source files.

use std::error::Error;
use std::fs;
use std::io;

use pancake::image_io::{RgbImage, decode};

/// Test results include recoverable fixture construction and application errors.
type TestResult = Result<(), Box<dyn Error>>;

/// An oversized IHDR with a correct CRC and no compressed data.
const OVERSIZED_PNG: [u8; 33] = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 1, 0, 0, 0, 1, 0, 0, 8, 2, 0,
    0, 0, 0xe3, 0xe6, 0xa7, 0xb4,
];

/// Corrupt paths are individually diagnosed without returning a decoded frame.
#[test]
fn malformed_file_diagnostics_identify_each_input() -> TestResult {
    let directory = tempfile::tempdir()?;
    for extension in ["png", "dng", "npy", "unknown"] {
        let path = directory.path().join(format!("invalid‐frame.{extension}"));
        let content = b"not a valid encoded image\x00\xff";
        fs::write(&path, content)?;
        let error = decode(&path).err().ok_or_else(|| {
            io::Error::other(format!(
                "malformed {extension} was accepted as a valid frame"
            ))
        })?;
        assert!(
            error.contains(&*path.to_string_lossy()),
            "missing source path: {error}"
        );
        assert_eq!(fs::read(path)?, content, "decoding must preserve the input");
    }
    Ok(())
}

/// The pixel limit is applied to the dimensions before requiring a raster.
#[test]
fn oversized_png_is_rejected_before_pixels_are_read() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("oversized.png");
    fs::write(&path, OVERSIZED_PNG)?;
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("oversized frame was decoded"))?;
    assert!(
        error.contains("limit") && error.contains("oversized.png"),
        "must reject oversized dimensions before decoding compressed data: {error}"
    );
    Ok(())
}

/// Shared frames validate RGB count, positive shape and finite samples.
#[test]
fn raster_constructor_validates_shape_and_samples() -> TestResult {
    let image = RgbImage::new(2, 1, vec![0.0_f32, 0.25, 0.5, 0.75, 1.0, 1.25])?;
    assert_eq!((image.width(), image.height()), (2, 1), "shape survives");
    assert_eq!(
        image
            .samples()
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        [0.0_f32, 0.25, 0.5, 0.75, 1.0, 1.25].map(f32::to_bits),
        "sample bits survive"
    );
    assert!(
        RgbImage::new(0, 1, Vec::new()).is_err(),
        "zero width must fail"
    );
    assert!(
        RgbImage::new(1, 0, Vec::new()).is_err(),
        "zero height must fail"
    );
    assert!(
        RgbImage::new(usize::MAX, 2, Vec::new()).is_err(),
        "overflowing area must fail"
    );
    assert!(
        RgbImage::new(1, 1, vec![0.0_f32]).is_err(),
        "sample undercount must fail"
    );
    assert!(
        RgbImage::new(1, 1, vec![0.0_f32, 1.0, f32::NAN]).is_err(),
        "nonfinite samples must fail"
    );
    Ok(())
}
