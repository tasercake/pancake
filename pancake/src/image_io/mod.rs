//! Bounded image decoding and full-precision RGB frames shared by both front ends.

mod export;
mod npy;
mod raw;

pub use export::{ExportFormat, PreparedExport, export, prepare_export};

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use image::{ColorType, DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};

/// Per-axis limit applied to supported format headers before raster decode.
const MAX_AXIS: u32 = 32_768;

/// Maximum full-resolution pixels in one imported frame.
const MAX_PIXELS: usize = 50_000_000;

/// Decoder memory bound, independent of job-level frame and working-data budgets.
const MAX_DECODE_BYTES: u64 = 768 * 1024 * 1024;

/// Encoded source sample representation before conversion to working f32.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleEncoding {
    /// Unsigned eight-bit samples.
    U8,
    /// Unsigned sixteen-bit samples, including fully developed RAW input.
    U16,
    /// Float samples, with the format-specific working scale applied.
    Float,
}

/// An owned full-precision RGB frame in validated row-major order.
///
/// Integer sources use normalized code values with the source transfer intact.
/// Float sources retain finite HDR samples. Display/export quantization is separate.
#[derive(Debug)]
pub struct RgbImage {
    /// Number of columns.
    width: usize,
    /// Number of rows.
    height: usize,
    /// Interleaved red, green and blue working samples.
    samples: Vec<f32>,
    /// Precision of the source raster.
    sample_encoding: SampleEncoding,
    /// Embedded profile, retained as metadata without implicit color conversion.
    icc_profile: Option<Vec<u8>>,
}

impl RgbImage {
    /// Construct a fully validated RGB frame from processing samples.
    ///
    /// # Errors
    ///
    /// Rejects invalid dimensions, a mismatched sample count and nonfinite samples.
    pub fn new(width: usize, height: usize, samples: Vec<f32>) -> Result<Self, String> {
        validate_samples(width, height, &samples)?;
        Ok(Self {
            width,
            height,
            samples,
            sample_encoding: SampleEncoding::Float,
            icc_profile: None,
        })
    }

    /// The width in pixels after orientation.
    #[must_use]
    pub const fn width(&self) -> usize {
        self.width
    }

    /// The height in pixels after orientation.
    #[must_use]
    pub const fn height(&self) -> usize {
        self.height
    }

    /// Contiguous RGB triples in row-major order.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Transfer ownership of the interleaved RGB samples.
    #[must_use]
    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    /// Encoded source precision, for consumers that choose export precision.
    #[must_use]
    pub const fn sample_encoding(&self) -> SampleEncoding {
        self.sample_encoding
    }

    /// The original embedded ICC profile when the format supplied one.
    #[must_use]
    pub fn icc_profile(&self) -> Option<&[u8]> {
        self.icc_profile.as_deref()
    }
}

/// Decode one complete image while retaining source precision in the working samples.
///
/// Supported inputs include standard raster formats, `NumPy` grayscale/RGB arrays
/// and fully developed DNG and CR3 camera RAW.
///
/// # Errors
///
/// Returns a source-path diagnostic for unsupported, malformed, nonfinite or
/// oversized images and for filesystem errors.
pub fn decode(path: &Path) -> Result<RgbImage, String> {
    decode_file(path).map_err(|error| format!("could not decode input {}: {error}", path.display()))
}

/// Select the actual format decoder for a regular file.
///
/// # Errors
///
/// Returns regular-file, filesystem, or format-specific errors.
fn decode_file(path: &Path) -> Result<RgbImage, String> {
    if !fs::metadata(path)
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("expected a regular image file".to_owned());
    }
    let input = File::open(path).map_err(|error| error.to_string())?;
    let extension = path.extension().and_then(OsStr::to_str);
    match extension.map(str::to_ascii_lowercase).as_deref() {
        Some("dng" | "cr3") => raw::decode(input, path),
        Some("npy") => npy::decode(BufReader::new(input)),
        _ => decode_raster(input),
    }
}

/// Apply header limits and orientation before converting native raster samples.
///
/// # Errors
///
/// Returns header-limit, malformed-raster, or conversion errors.
fn decode_raster(input: File) -> Result<RgbImage, String> {
    let mut reader = ImageReader::new(BufReader::new(input))
        .with_guessed_format()
        .map_err(|error| error.to_string())?;
    let format = reader.format();
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_AXIS);
    limits.max_image_height = Some(MAX_AXIS);
    reader.limits(limits.clone());
    let mut decoder = reader.into_decoder().map_err(|error| error.to_string())?;
    let (width, height) = decoder.dimensions();
    let _count = checked_pixel_count(
        usize::try_from(width).map_err(|error| error.to_string())?,
        usize::try_from(height).map_err(|error| error.to_string())?,
    )?;
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
        .reserve(decoder.total_bytes())
        .map_err(|error| error.to_string())?;
    decoder
        .set_limits(limits)
        .map_err(|error| error.to_string())?;
    let orientation = decoder.orientation().map_err(|error| error.to_string())?;
    let profile = decoder.icc_profile().map_err(|error| error.to_string())?;
    let mut image = DynamicImage::from_decoder(decoder).map_err(|error| error.to_string())?;
    image.apply_orientation(orientation);
    raster_to_rgb(image, format, profile)
}

/// Return a checked pixel count before allocating a buffer or reading samples.
///
/// # Errors
///
/// Rejects empty, oversized, or unaddressable dimensions.
fn checked_pixel_count(width: usize, height: usize) -> Result<usize, String> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| "image area overflowed".to_owned())?;
    let axis_limit = usize::try_from(MAX_AXIS).map_err(|error| error.to_string())?;
    if width > axis_limit || height > axis_limit || pixels > MAX_PIXELS {
        return Err(format!(
            "dimensions {width}×{height} exceed the image limit of {MAX_PIXELS} pixels and {MAX_AXIS} per axis"
        ));
    }
    if pixels == 0 {
        return Err("image dimensions must be nonzero".to_owned());
    }
    Ok(pixels)
}

/// Validate length and finite values at the boundary to full-precision processing.
///
/// # Errors
///
/// Rejects shape, sample count, and nonfinite values.
fn validate_samples(width: usize, height: usize, samples: &[f32]) -> Result<(), String> {
    let count = checked_pixel_count(width, height)?
        .checked_mul(3)
        .ok_or_else(|| "RGB sample count overflowed".to_owned())?;
    if samples.len() != count {
        return Err("sample count does not match image dimensions".to_owned());
    }
    if samples.iter().any(|sample| !sample.is_finite()) {
        return Err("image contains nonfinite samples".to_owned());
    }
    Ok(())
}

/// Convert native samples directly to working RGB without an eight-bit intermediate.
///
/// # Errors
///
/// Returns unsupported color or invalid working-frame errors.
fn raster_to_rgb(
    image: DynamicImage,
    format: Option<ImageFormat>,
    icc_profile: Option<Vec<u8>>,
) -> Result<RgbImage, String> {
    let width = usize::try_from(image.width()).map_err(|error| error.to_string())?;
    let height = usize::try_from(image.height()).map_err(|error| error.to_string())?;
    let sample_encoding = match image.color() {
        ColorType::L8 | ColorType::La8 | ColorType::Rgb8 | ColorType::Rgba8 => SampleEncoding::U8,
        ColorType::L16 | ColorType::La16 | ColorType::Rgb16 | ColorType::Rgba16 => {
            SampleEncoding::U16
        }
        ColorType::Rgb32F | ColorType::Rgba32F => SampleEncoding::Float,
        _ => return Err("unsupported native sample precision".to_owned()),
    };
    let mut frame = RgbImage::new(width, height, image.into_rgb32f().into_raw())?;
    // Float TIFF workflows can encode 0–255 values instead of normalized 0–1.
    // Limit the compatibility conversion to that range; retain arbitrary HDR.
    if format == Some(ImageFormat::Tiff) && sample_encoding == SampleEncoding::Float {
        normalize_float_codes(&mut frame.samples);
    }
    frame.sample_encoding = sample_encoding;
    frame.icc_profile = icc_profile;
    Ok(frame)
}

/// Interpret a nonnegative float raster in the normalized or 8-bit code range.
fn normalize_float_codes(samples: &mut [f32]) {
    let (min, max) = samples
        .iter()
        .copied()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), sample| {
            (low.min(sample), high.max(sample))
        });
    if min >= 0.0 && max > 1.0 && max <= 255.0 {
        for sample in samples {
            *sample /= 255.0;
        }
    }
}
