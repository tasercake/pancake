//! Independent delivery checks using real staged encoders.

use std::{error::Error, fs};

use pancake::image_io::RgbImage;

/// Failure staging any batch member preserves every old export.
///
/// # Errors
///
/// Returns a real setup, export, or file-retention failure.
#[test]
fn failed_batch_keeps_previous_files() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path();
    let image = RgbImage::new(2, 1, vec![0.25, 0.5, 0.75, 0.5, 0.25, 1.0])?;
    super::batch(path, &image, &[], 95)?;
    let png = path.join("Pancake.png");
    let tiff = path.join("Pancake.tif");
    let jpeg = path.join("Pancake.jpg");
    let previous_png = fs::read(&png)?;
    let previous_tiff = fs::read(&tiff)?;
    fs::remove_file(&jpeg)?;
    fs::create_dir(&jpeg)?;
    if super::batch(path, &image, &[], 95).is_ok() {
        return Err("a conflicting output directory was accepted".into());
    }
    if fs::read(&png)? != previous_png || fs::read(&tiff)? != previous_tiff {
        return Err("failed batch changed another destination".into());
    }
    Ok(())
}
