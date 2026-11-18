//! Full-precision encoding followed by same-directory atomic publication.

use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufWriter, ErrorKind, Write};
use std::path::{Path, PathBuf};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::openexr::OpenExrEncoder;
use image::{ExtendedColorType, ImageEncoder};
use same_file::Handle;
use tempfile::NamedTempFile;
use tiff::encoder::{TiffEncoder, TiffValue, colortype};
use tiff::tags::Tag;

use super::RgbImage;
use crate::config::BitDepth;

/// Format selected by the destination suffix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    /// Lossless integer RGB PNG.
    Png,
    /// Lossy eight-bit RGB JPEG.
    Jpeg,
    /// Lossless integer RGB TIFF.
    Tiff,
    /// Lossless floating-point RGB `OpenEXR`.
    Exr,
}

impl ExportFormat {
    /// Resolve the format before creating temporary output.
    ///
    /// # Errors
    ///
    /// Returns an error if the suffix is absent or unsupported.
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let extension = path.extension().and_then(OsStr::to_str).unwrap_or_default();
        match extension.to_ascii_lowercase().as_str() {
            "png" => Ok(Self::Png),
            "jpg" | "jpeg" => Ok(Self::Jpeg),
            "tif" | "tiff" => Ok(Self::Tiff),
            "exr" => Ok(Self::Exr),
            _ => Err(format!("unsupported export format: {}", path.display())),
        }
    }

    /// Write the complete image to an already open temporary stream.
    ///
    /// # Errors
    /// Returns an encoder or write failure for the selected format.
    fn encode(
        self,
        file: &mut BufWriter<&mut File>,
        frame: &RgbImage,
        width: u32,
        height: u32,
        depth: BitDepth,
        jpeg_quality: u8,
    ) -> Result<(), String> {
        match self {
            Self::Png => write_png(file, frame, width, height, depth),
            Self::Jpeg => write_jpeg(file, frame, width, height, jpeg_quality),
            Self::Tiff => match depth {
                BitDepth::Eight => {
                    write_tiff::<colortype::RGB8>(file, frame, width, height, quantize8)
                }
                BitDepth::Sixteen => {
                    write_tiff::<colortype::RGB16>(file, frame, width, height, quantize16)
                }
            },
            Self::Exr => OpenExrEncoder::new(file)
                .write_image(
                    bytemuck::cast_slice(frame.samples()),
                    width,
                    height,
                    ExtendedColorType::Rgb32F,
                )
                .map_err(encoding_error),
        }
    }
}

/// A complete encoded output which can be committed or discarded.
#[derive(Debug)]
pub struct PreparedExport {
    /// Staging file in the destination directory. Dropping it rolls back preparation.
    file: NamedTempFile,
    /// Absolute publication path with its parent already resolved.
    destination: PathBuf,
    /// Source aliases to recheck at commit.
    sources: Vec<PathBuf>,
    /// Format-specific reduction in requested precision.
    warning: Option<&'static str>,
}

impl PreparedExport {
    /// Report a reduction in requested precision independent of publication.
    #[must_use]
    pub const fn warning(&self) -> Option<&'static str> {
        self.warning
    }

    /// Atomically install the complete output. Failed installation preserves
    /// existing data and discards staging.
    ///
    /// # Errors
    ///
    /// Returns an error if the destination aliases a source or cannot be replaced.
    pub fn publish(self) -> Result<(), String> {
        check_source_alias(&self.destination, &self.sources)?;
        let _published = self
            .file
            .persist(&self.destination)
            .map_err(|error| destination_error(&self.destination, &error.error))?;
        Ok(())
    }
}

/// Encode and flush an export alongside a destination without changing it.
/// Dropping the returned handle discards the encoded output.
///
/// # Errors
///
/// Returns an error for unsupported formats, invalid dimensions or quality,
/// a destination that aliases a source, or a staging/encoding failure.
pub fn prepare_export(
    path: &Path,
    frame: &RgbImage,
    depth: BitDepth,
    jpeg_quality: u8,
    sources: &[PathBuf],
) -> Result<PreparedExport, String> {
    let format = ExportFormat::from_path(path)?;
    if format == ExportFormat::Jpeg && !(1..=100).contains(&jpeg_quality) {
        return Err(format!(
            "JPEG quality must be between 1 and 100; got {jpeg_quality}"
        ));
    }
    let destination = absolute_destination(path)?;
    check_source_alias(&destination, sources)?;
    let parent = destination
        .parent()
        .ok_or_else(|| format!("missing export directory: {}", destination.display()))?;
    let mut file =
        NamedTempFile::new_in(parent).map_err(|error| destination_error(path, &error))?;
    let width = u32::try_from(frame.width()).map_err(encoding_error)?;
    let height = u32::try_from(frame.height()).map_err(encoding_error)?;
    {
        let mut writer = BufWriter::new(file.as_file_mut());
        format
            .encode(&mut writer, frame, width, height, depth, jpeg_quality)
            .map_err(|error| encoding_error_at(path, &error))?;
        writer
            .flush()
            .map_err(|error| destination_error(path, &error))?;
    }
    file.as_file()
        .sync_all()
        .map_err(|error| destination_error(path, &error))?;
    let warning = (format == ExportFormat::Jpeg && depth == BitDepth::Sixteen).then_some(
        "JPEG supports 8-bit samples; the 16-bit request will be exported as 8-bit JPEG.",
    );
    Ok(PreparedExport {
        file,
        destination,
        sources: sources.to_vec(),
        warning,
    })
}

/// Export and commit a complete image in one operation.
///
/// # Errors
///
/// Returns an error if preparation or publication fails, preserving sources
/// and any old destination.
pub fn export(
    path: &Path,
    frame: &RgbImage,
    depth: BitDepth,
    jpeg_quality: u8,
    sources: &[PathBuf],
) -> Result<(), String> {
    prepare_export(path, frame, depth, jpeg_quality, sources)?.publish()
}

/// Resolve only the parent directory so a destination symlink remains detectable.
///
/// # Errors
/// Returns an error when the file name is missing or the directory is inaccessible.
fn absolute_destination(path: &Path) -> Result<PathBuf, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| format!("missing export file name: {}", path.display()))?;
    let directory = path
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(directory).map_err(|error| destination_error(path, &error))?;
    Ok(parent.join(file_name))
}

/// Detect aliases by file identity, including hardlinks and followed symlinks.
///
/// # Errors
/// Returns an error when an input matches the destination or identity inspection fails.
fn check_source_alias(destination: &Path, sources: &[PathBuf]) -> Result<(), String> {
    match fs::metadata(destination) {
        Ok(info) if info.is_file() => {}
        Ok(_) => {
            return Err(format!(
                "export destination {} is not a regular file",
                destination.display()
            ));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(destination_error(destination, &error)),
    }
    if sources.is_empty() {
        return Ok(());
    }
    let target =
        Handle::from_path(destination).map_err(|error| destination_error(destination, &error))?;
    for source in sources {
        match Handle::from_path(source) {
            Ok(handle) if handle == target => {
                return Err(format!(
                    "export destination {} aliases input {}",
                    destination.display(),
                    source.display()
                ));
            }
            Ok(_) => {}
            // Retained GUI images remain usable after a source is removed.
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "cannot inspect input {}: {error}",
                    source.display()
                ));
            }
        }
    }
    Ok(())
}

/// Stream rows of big-endian PNG samples with at most one row of staging bytes.
///
/// # Errors
/// Returns an allocation, encoding, or write failure.
fn write_png(
    file: &mut BufWriter<&mut File>,
    frame: &RgbImage,
    width: u32,
    height: u32,
    depth: BitDepth,
) -> Result<(), String> {
    let mut info = png::Info::with_size(width, height);
    info.color_type = png::ColorType::Rgb;
    info.bit_depth = match depth {
        BitDepth::Eight => png::BitDepth::Eight,
        BitDepth::Sixteen => png::BitDepth::Sixteen,
    };
    info.icc_profile = frame.icc_profile().map(Cow::Borrowed);
    let encoder = png::Encoder::with_info(file, info).map_err(encoding_error)?;
    let mut writer = encoder.write_header().map_err(encoding_error)?;
    let mut stream = writer.stream_writer().map_err(encoding_error)?;
    let row_sample_count = frame.width().checked_mul(3).ok_or("PNG row is too wide")?;
    let row_byte_count = row_sample_count
        .checked_mul(if depth == BitDepth::Eight { 1 } else { 2 })
        .ok_or("PNG row byte count overflow")?;
    let mut row = Vec::new();
    row.try_reserve_exact(row_byte_count)
        .map_err(encoding_error)?;
    for channels in frame.samples().chunks_exact(row_sample_count) {
        row.clear();
        match depth {
            BitDepth::Eight => row.extend(channels.iter().copied().map(quantize8)),
            BitDepth::Sixteen => {
                row.extend(
                    channels
                        .iter()
                        .copied()
                        .flat_map(|sample| quantize16(sample).to_be_bytes()),
                );
            }
        }
        stream.write_all(&row).map_err(encoding_error)?;
    }
    stream.finish().map_err(encoding_error)?;
    writer.finish().map_err(encoding_error)
}

/// Encode JPEG from eight-bit RGB values with applied quality and ICC metadata.
///
/// # Errors
/// Returns an allocation, encoding, or write failure.
fn write_jpeg(
    file: &mut BufWriter<&mut File>,
    frame: &RgbImage,
    width: u32,
    height: u32,
    quality: u8,
) -> Result<(), String> {
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(frame.samples().len())
        .map_err(encoding_error)?;
    samples.extend(frame.samples().iter().copied().map(quantize8));
    let mut encoder = JpegEncoder::new_with_quality(file, quality);
    if let Some(profile) = frame.icc_profile() {
        encoder
            .set_icc_profile(profile.to_vec())
            .map_err(encoding_error)?;
    }
    encoder
        .write_image(&samples, width, height, ExtendedColorType::Rgb8)
        .map_err(encoding_error)
}

/// Write direct typed TIFF strips into a spool using one reusable conversion buffer.
///
/// # Errors
/// Returns an allocation, encoding, or write failure.
fn write_tiff<C: colortype::ColorType>(
    file: &mut BufWriter<&mut File>,
    frame: &RgbImage,
    width: u32,
    height: u32,
    quantize: fn(f32) -> C::Inner,
) -> Result<(), String>
where
    [C::Inner]: TiffValue,
{
    let mut encoder = TiffEncoder::new(file).map_err(encoding_error)?;
    let mut image = encoder
        .new_image::<C>(width, height)
        .map_err(encoding_error)?;
    if let Some(profile) = frame.icc_profile() {
        image
            .encoder()
            .write_tag(Tag::IccProfile, profile)
            .map_err(encoding_error)?;
    }
    let mut samples = frame.samples().iter().copied();
    let mut strip = Vec::new();
    while image.next_strip_sample_count() != 0 {
        let count = usize::try_from(image.next_strip_sample_count()).map_err(encoding_error)?;
        strip.clear();
        strip.try_reserve_exact(count).map_err(encoding_error)?;
        strip.extend(samples.by_ref().take(count).map(quantize));
        image.write_strip(&strip).map_err(encoding_error)?;
    }
    image.finish().map_err(encoding_error)
}

/// Round clamped normalized samples to eight-bit codes.
#[expect(
    clippy::as_conversions,
    reason = "Clamped rounded float converted to its integer code"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Rounded value is in the full u8 range"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "Clamping guarantees a nonnegative sample"
)]
fn quantize8(sample: f32) -> u8 {
    (sample.clamp(0.0, 1.0) * f32::from(u8::MAX)).round() as u8
}

/// Round clamped normalized samples to full sixteen-bit codes.
#[expect(
    clippy::as_conversions,
    reason = "Clamped rounded float converted to its integer code"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "Rounded value is in the full u16 range"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "Clamping guarantees a nonnegative sample"
)]
fn quantize16(sample: f32) -> u16 {
    (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16
}

/// Convert write and allocation errors to the shared image boundary type.
fn encoding_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Add the requested path to the encoder's failure.
fn encoding_error_at(path: &Path, error: &str) -> String {
    format!("cannot encode {}: {error}", path.display())
}

/// Identify the destination of a failed write, flush, or rename.
fn destination_error(path: &Path, error: &std::io::Error) -> String {
    format!("cannot publish export {}: {error}", path.display())
}
