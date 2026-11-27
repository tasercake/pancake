//! Bounded display sampling from the shared full-precision RGB image.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use num_traits::ToPrimitive as _;
use pancake::image_io::RgbImage;
use pancake::runtime::SelectedRuntime;
use slint::wgpu_30::wgpu;
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
        let raster = sample_raster(frame, MAX_PREVIEW_AXIS)?;
        Ok(Self {
            picture: Image::from_rgba8(raster),
            width: display_dimension(frame.width())?,
            height: display_dimension(frame.height())?,
        })
    }
}

/// One bounded filmstrip raster, transferable to the native event thread.
#[derive(Debug)]
pub(super) struct Thumbnail {
    /// Oriented display samples with no connection to renderer resources.
    raster: SharedPixelBuffer<Rgba8Pixel>,
}

impl Thumbnail {
    /// Downsample the full-precision decoded frame for the filmstrip.
    /// # Errors
    /// Returns a sample or source geometry mismatch.
    pub(super) fn new(frame: &RgbImage) -> Result<Self, String> {
        Ok(Self {
            raster: sample_raster(frame, 160)?,
        })
    }

    /// Attach the native renderer image on the view thread.
    pub(super) fn into_image(self) -> Image {
        Image::from_rgba8(self.raster)
    }
}

/// Convert one supported raster dimension to the toolkit representation.
/// # Errors
/// Returns a representation error for an oversized axis.
fn display_dimension(axis: usize) -> Result<u32, String> {
    u32::try_from(axis).map_err(|error| format!("Preview dimension: {error}"))
}

/// Produce a separate bounded display raster, preserving the original samples.
/// # Errors
/// Returns a source shape or sample conversion error.
fn sample_raster(
    frame: &RgbImage,
    max_display_axis: usize,
) -> Result<SharedPixelBuffer<Rgba8Pixel>, String> {
    let (width, height) = (frame.width(), frame.height());
    let scale = width.max(height).div_ceil(max_display_axis).max(1);
    let (display_width, display_height) = (width.div_ceil(scale), height.div_ceil(scale));
    let mut raster = SharedPixelBuffer::<Rgba8Pixel>::new(
        display_dimension(display_width)?,
        display_dimension(display_height)?,
    );
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
    Ok(raster)
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
///
/// The viewer can additionally reference its two displayed images and stage
/// one new raster. These lifetimes are independent of full-precision results.
#[derive(Debug, Default)]
pub struct Cache {
    /// Least recently accessed previews first.
    entries: VecDeque<(ImageKey, Preview)>,
    /// Shared renderer handles for the lifetime of this workspace.
    device: Option<PreviewDevice>,
}

impl Cache {
    /// Cache textures on the active Slint device, or rasters on software.
    #[must_use]
    pub fn for_runtime(runtime: Option<&SelectedRuntime>) -> Self {
        let device = runtime
            .and_then(SelectedRuntime::wgpu_setup)
            .map(|setup| PreviewDevice {
                device: setup.device.clone(),
                queue: setup.queue.clone(),
            });
        Self {
            device,
            ..Self::default()
        }
    }

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
        let mut picture = build()?;
        if let Some(device) = &self.device {
            picture.picture = device.retain(&picture.picture)?;
        }
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

/// Device-local resources never outlive their owning viewer session.
#[derive(Debug)]
struct PreviewDevice {
    /// The exact device Slint uses for its presentation surfaces.
    device: wgpu::Device,
    /// The matching queue, so resource creation precedes any frame that samples it.
    queue: wgpu::Queue,
}

impl PreviewDevice {
    /// Retain the same bounded display samples as one native Slint texture.
    ///
    /// # Errors
    ///
    /// Returns an invalid raster or a texture import error.
    fn retain(&self, image: &Image) -> Result<Image, String> {
        let raster = image.to_rgba8().ok_or("Preview raster is unavailable.")?;
        let row_bytes = raster
            .width()
            .checked_mul(4_u32)
            .ok_or("Preview upload row is too large.")?;
        let size = wgpu::Extent3d {
            width: raster.width(),
            height: raster.height(),
            depth_or_array_layers: 1,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Pancake Stacker viewer preview"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.queue.write_texture(
            texture.as_image_copy(),
            raster.as_bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row_bytes),
                rows_per_image: None,
            },
            size,
        );
        Image::try_from(texture).map_err(|error| error.to_string())
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
