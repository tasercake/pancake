//! Validate preview reuse and eviction with actual decoded RGB samples.

use pancake::image_io::RgbImage;

use super::{Cache, ImageKey, Preview};

/// Display cache reuses one immutable preview and evicts the oldest entry.
///
/// # Errors
///
/// Returns an invalid preview or a violated cache boundary.
#[test]
fn retains_no_more_than_four_display_images() -> Result<(), String> {
    let mut cache = Cache::for_runtime(None);
    let raster = RgbImage::new(1, 1, vec![0.2, 0.4, 0.6])?;
    for index in 0_usize..4 {
        let _preview = cache.get(ImageKey::Result(index, false), || Preview::new(&raster))?;
    }
    let _preview = cache.get(ImageKey::Result(0, false), || {
        Err("new preview for cached result".into())
    })?;
    let _preview = cache.get(ImageKey::Result(4, false), || Preview::new(&raster))?;
    let _preview = cache.get(ImageKey::Result(0, false), || {
        Err("evicted recently used preview".into())
    })?;
    if cache
        .get(ImageKey::Result(1, false), || {
            Err("preview was evicted".into())
        })
        .is_ok()
    {
        return Err("oldest preview was retained over the four-image limit".into());
    }
    Ok(())
}

/// A filmstrip thumbnail retains image orientation and bounded RGB samples.
/// # Errors
/// Returns a sample, geometry or renderer-buffer mismatch.
#[test]
fn thumbnail_samples_a_real_wide_frame_with_bounded_storage() -> Result<(), String> {
    let width = 480;
    let height = 240;
    let mut samples = Vec::with_capacity(480_usize.saturating_mul(240).saturating_mul(3));
    for _ in 0..height {
        for x in 0..width {
            let sample = if x == 0 {
                [1.0, 0.0, 0.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            samples.extend(sample);
        }
    }
    let frame = RgbImage::new(width, height, samples)?;
    let thumbnail = super::Thumbnail::new(&frame)?.into_image();
    let raster = thumbnail
        .to_rgba8()
        .ok_or("Missing filmstrip thumbnail raster.")?;
    let first = raster
        .as_slice()
        .first()
        .ok_or("Missing thumbnail sample.")?;
    let last = raster
        .as_slice()
        .last()
        .ok_or("Missing thumbnail sample.")?;
    if (
        raster.width(),
        raster.height(),
        first.r,
        first.b,
        last.r,
        last.b,
    ) != (160, 80, 255, 0, 0, 255)
    {
        return Err(
            "The thumbnail lost real source samples, proportions, or its size limit.".into(),
        );
    }
    Ok(())
}
