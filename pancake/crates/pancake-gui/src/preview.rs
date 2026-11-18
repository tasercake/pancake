//! Bounded display sampling from the shared full-precision RGB image.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use num_traits::ToPrimitive as _;
use pancake::image_io::RgbImage;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// A decoded display raster is bounded separately from authoritative pixels.
const MAX_PREVIEW_AXIS: usize = 2_048;

/// Downsample into a bounded color preview, retaining native geometry metadata.
#[derive(Clone, Debug)]
pub struct Preview {
    /// Pixel raster consumed only by the renderer.
    pub picture: Image,
    /// Native image width.
    pub width: u32,
    /// Native image height.
    pub height: u32,
}

impl Preview {
    /// Copy an oriented source image to bounded display storage.
    ///
    /// # Errors
    ///
    /// Returns an error if the frame's geometry or sample access is inconsistent.
    pub fn new(frame: &RgbImage) -> Result<Self, String> {
        let (width, height) = (frame.width(), frame.height());
        let max_axis = width.max(height);
        let scale = max_axis.div_ceil(MAX_PREVIEW_AXIS).max(1);
        let (display_width, display_height) = (width.div_ceil(scale), height.div_ceil(scale));
        let convert =
            |axis| u32::try_from(axis).map_err(|error| format!("Preview dimension: {error}"));

        let mut raster =
            SharedPixelBuffer::<Rgba8Pixel>::new(convert(display_width)?, convert(display_height)?);
        let (pixels, remainder) = frame.samples().as_chunks::<3>();
        if !remainder.is_empty() {
            return Err("Preview source has incomplete RGB pixels.".into());
        }

        for (y, row) in raster
            .make_mut_slice()
            .chunks_exact_mut(display_width)
            .enumerate()
        {
            let source_start = y
                .checked_mul(scale)
                .and_then(|index| index.checked_mul(width))
                .ok_or("Preview row exceeded source geometry.")?;
            for (x, pixel) in row.iter_mut().enumerate() {
                let index = x
                    .checked_mul(scale)
                    .and_then(|index| source_start.checked_add(index))
                    .ok_or("Preview column exceeded source geometry.")?;
                let [red, green, blue] = *pixels.get(index).ok_or("Preview sample was missing.")?;
                *pixel = Rgba8Pixel::new(channel(red)?, channel(green)?, channel(blue)?, 255);
            }
        }
        Ok(Self {
            picture: Image::from_rgba8(raster),
            width: convert(width)?,
            height: convert(height)?,
        })
    }
}

/// Identity of an unchanged source or an immutable result representation.
#[derive(Debug, Eq, PartialEq)]
pub enum ImageKey {
    /// The identity of a regular source and its actual current filesystem stamp.
    Source {
        /// Absolute source path from the ordered catalog.
        path: PathBuf,
        /// Most recent content modification.
        modified: SystemTime,
        /// Current byte count.
        bytes: u64,
    },
    /// Retained stack ordinal paired with its manual-crop revision.
    Result(usize, bool),
}

impl ImageKey {
    /// Recheck a real source before reusing any already-produced preview.
    ///
    /// # Errors
    ///
    /// Returns an inaccessible source or a metadata inspection error.
    pub fn source(path: &Path) -> Result<Self, String> {
        let metadata = path
            .metadata()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let modified = metadata.modified().map_err(|error| error.to_string())?;
        Ok(Self::Source {
            path: path.to_path_buf(),
            modified,
            bytes: metadata.len(),
        })
    }
}

/// Four display-only images at at most 16 MiB apiece.
#[derive(Debug, Default)]
pub struct Cache {
    /// Least recently accessed previews first.
    entries: VecDeque<(ImageKey, Preview)>,
}

impl Cache {
    /// Retrieve a prior preview or render exactly one current frame.
    ///
    /// # Errors
    ///
    /// Returns a failed preview build or inconsistent cache access.
    pub fn get(
        &mut self,
        key: ImageKey,
        build: impl FnOnce() -> Result<Preview, String>,
    ) -> Result<Preview, String> {
        if let Some(index) = self
            .entries
            .iter()
            .position(|(identity, _)| *identity == key)
        {
            let entry = self
                .entries
                .remove(index)
                .ok_or("Preview was evicted during selection.")?;
            let picture = entry.1.clone();
            self.entries.push_back(entry);
            return Ok(picture);
        }
        let picture = build()?;
        if self.entries.len() == 4 {
            let _evicted = self.entries.pop_front();
        }
        self.entries.push_back((key, picture.clone()));
        Ok(picture)
    }

    /// Remove previews from the replaced source and result collections.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Quantize a finite color component for display.
///
/// # Errors
///
/// Rejects nonfinite samples instead of synthesizing a pixel value.
fn channel(sample: f32) -> Result<u8, String> {
    sample
        .clamp(0.0, 1.0)
        .mul_add(255.0, 0.5)
        .floor()
        .to_u8()
        .ok_or_else(|| "Preview received a nonfinite color component.".into())
}

#[cfg(test)]
mod tests;
