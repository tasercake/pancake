//! Check that retained crop coordinates preserve exact source samples.

use crate::image_io::RgbImage;
use crate::registration::Bounds;

/// Apply an absolute crop to a frame that already has a reference origin.
///
/// # Errors
///
/// Returns invalid fixtures or a changed original/sample subset.
#[test]
fn copies_only_the_retained_absolute_crop() -> Result<(), String> {
    let initial: Vec<f32> = (0_u16..36).map(f32::from).collect();
    let image = RgbImage::new(4, 3, initial.clone())?;
    let original = Bounds::new(4, 10, 4, 3)?;
    let target = Bounds::new(5, 11, 2, 2)?;
    let cropped = super::copy_crop(&image, original, target)?;
    let expected: Vec<f32> = (15_u16..21).chain(27..33).map(f32::from).collect();
    if cropped.width() != 2 || cropped.height() != 2 || cropped.samples() != expected {
        return Err("cropped image did not preserve the exact rectangle".into());
    }
    if image.samples() != initial {
        return Err("crop changed the immutable source".into());
    }
    if super::copy_crop(&image, original, Bounds::new(3, 11, 2, 2)?).is_ok() {
        return Err("crop outside the retained source was accepted".into());
    }
    Ok(())
}
