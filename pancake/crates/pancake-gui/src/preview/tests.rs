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
    let mut cache = Cache::default();
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
