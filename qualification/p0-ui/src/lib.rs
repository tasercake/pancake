//! Bounded ownership primitives for the native qualification harness.

// Runtime dependencies are consumed by the companion native executable.
use cubecl as _;
use pancake as _;
use slint as _;
use slint_interpreter as _;
use wgpu as _;

/// Build one bounded high-bit checker plane without retaining a frame collection.
///
/// # Errors
///
/// Rejects empty dimensions or more than 2048x1536 samples.
pub fn make_source(width: usize, height: usize, inverted: bool) -> Result<Vec<u16>, String> {
    let area = width
        .checked_mul(height)
        .filter(|area| *area <= 3_145_728)
        .ok_or("invalid working image dimensions")?;
    if area == 0 {
        return Err("invalid working image dimensions".to_owned());
    }
    let mut pixels = Vec::with_capacity(area);
    for row in 0..height {
        for column in 0..width {
            let white = (((row / 128) & 1) != ((column / 128) & 1)) ^ inverted;
            pixels.push(if white { 65_535 } else { 0 });
        }
    }
    Ok(pixels)
}

/// Downsample a complete u16 plane to an explicit RGBA8 preview staging buffer.
///
/// # Errors
///
/// Rejects empty/overflowed dimensions, wrong sample count or a zero step.
pub fn preview_rgba(
    pixels: &[u16],
    width: usize,
    height: usize,
    step: usize,
) -> Result<Vec<u8>, String> {
    let area = width
        .checked_mul(height)
        .ok_or("working image area overflow")?;
    if pixels.len() != area {
        return Err("pixel count does not match working image".to_owned());
    }
    if area == 0 || step == 0 {
        return Err("invalid preview dimensions".to_owned());
    }
    let bytes = width
        .div_ceil(step)
        .checked_mul(height.div_ceil(step))
        .and_then(|value| value.checked_mul(4))
        .ok_or("preview area overflow")?;
    let mut output = Vec::with_capacity(bytes);
    for row in (0..height).step_by(step) {
        for column in (0..width).step_by(step) {
            let index = row
                .checked_mul(width)
                .and_then(|base| base.checked_add(column))
                .ok_or("preview index overflow")?;
            let pixel = pixels.get(index).ok_or("missing preview sample")?;
            let value = u8::try_from(pixel / 257).map_err(|error| error.to_string())?;
            output.extend_from_slice(&[value, value, value, 255]);
        }
    }
    Ok(output)
}

/// Two-slot ownership cache; eviction drops the oldest retained resource.
#[derive(Debug)]
pub struct SlotCache<T> {
    /// Retained resources in insertion order.
    items: std::collections::VecDeque<T>,
    /// Largest retained count actually observed.
    peak: usize,
}

impl<T> Default for SlotCache<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SlotCache<T> {
    /// Construct an empty fixed-capacity cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: std::collections::VecDeque::with_capacity(2),
            peak: 0,
        }
    }

    /// Retain a resource, releasing the oldest slot before adding a third.
    pub fn insert(&mut self, item: T) {
        self.make_room();
        self.items.push_back(item);
        self.peak = self.peak.max(self.items.len());
    }

    /// Release the oldest full slot before allocating its replacement.
    pub fn make_room(&mut self) {
        if self.items.len() == 2 {
            drop(self.items.pop_front());
        }
    }

    /// Number of resources retained now.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the cache owns no resources.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Maximum count observed while inserting resources.
    #[must_use]
    pub const fn peak(&self) -> usize {
        self.peak
    }
}

#[cfg(test)]
mod tests {
    use super::{SlotCache, make_source, preview_rgba};
    use std::{cell::Cell, rc::Rc};

    /// Resource whose drop gives an independent ownership observation.
    #[derive(Debug)]
    struct Resource {
        /// Count resources which have actually been released.
        drops: Rc<Cell<usize>>,
    }

    impl Drop for Resource {
        fn drop(&mut self) {
            self.drops.set(self.drops.get().saturating_add(1));
        }
    }

    /// The cache must hold two resources and release every replaced/closed slot.
    ///
    /// # Panics
    ///
    /// Panics if resource count or release behavior violates the fixed bound.
    #[test]
    fn cache_evicts_and_releases_at_fixed_capacity() {
        let drops = Rc::new(Cell::new(0));
        let mut cache = SlotCache::new();
        assert!(cache.is_empty(), "new cache must be empty");
        for _ in 0_usize..5 {
            cache.insert(Resource {
                drops: Rc::clone(&drops),
            });
        }
        assert_eq!(cache.len(), 2, "cache capacity must remain two");
        assert_eq!(
            cache.peak(),
            2,
            "recorded cache high-water count must be bounded"
        );
        assert_eq!(drops.get(), 3, "eviction must release the oldest resources");
        drop(cache);
        assert_eq!(drops.get(), 5, "closing must release retained resources");
    }

    /// Generated image and preview conversion have independent known values.
    ///
    /// # Panics
    ///
    /// Panics if image dimensions, pattern or preview bytes are incorrect.
    #[test]
    fn known_image_and_preview_bytes() {
        assert_eq!(
            make_source(0, 1, false),
            Err("invalid working image dimensions".to_owned()),
            "empty dimensions"
        );
        assert_eq!(
            make_source(2, 1, false),
            Ok(vec![0, 0]),
            "first checker square"
        );
        assert_eq!(
            make_source(2, 1, true),
            Ok(vec![65_535, 65_535]),
            "inverted checker square"
        );
        assert_eq!(
            preview_rgba(&[0, 32_768, 65_535, 257], 2, 2, 1),
            Ok(vec![
                0, 0, 0, 255, 127, 127, 127, 255, 255, 255, 255, 255, 1, 1, 1, 255
            ]),
            "u16 preview conversion"
        );
        assert_eq!(
            preview_rgba(&[0], 2, 2, 1),
            Err("pixel count does not match working image".to_owned()),
            "bad count"
        );
    }
}
