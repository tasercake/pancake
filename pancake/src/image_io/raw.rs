//! Owned camera RAW decode and complete development on a bounded local pool.

use std::fs::File;
use std::io::Read;
use std::panic;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use image::metadata::Orientation;
use rawler::decoders::{Decoder, FormatHint, RawDecodeParams, WellKnownIFD};
use rawler::formats::bmff::Bmff;
use rawler::imgop::develop::RawDevelop;
use rawler::rawsource::RawSource;
use rawler::tags::TiffCommonTag;

use super::{RgbImage, checked_pixel_count, raster_to_rgb};

/// Maximum owned encoded RAW bytes, separate from raster allocation limits.
const MAX_RAW_BYTES: u64 = 256 * 1024 * 1024;

/// One process-wide pool bounds nested decoder and developer parallelism.
static RAW_POOL: LazyLock<Result<rayon::ThreadPool, String>> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|index| format!("pancake-raw-{index}"))
        .build()
        .map_err(|error| error.to_string())
});

/// Validate the encoded source size and execute full camera-RAW development.
///
/// # Errors
///
/// Returns file-read, size-limit, worker-pool, or camera decode errors.
pub(super) fn decode(file: File, path: &Path) -> Result<RgbImage, String> {
    let mut bytes = Vec::new();
    let count = file
        .take(MAX_RAW_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    let count = u64::try_from(count).map_err(|error| error.to_string())?;
    if count > MAX_RAW_BYTES {
        return Err("compressed RAW source exceeds the byte limit".to_owned());
    }
    let source = RawSource::new_from_shared_vec(Arc::new(bytes)).with_path(path);
    let pool = RAW_POOL.as_ref().map_err(Clone::clone)?;
    // Protect the application boundary from unwinding third-party decoders.
    panic::catch_unwind(panic::AssertUnwindSafe(|| {
        pool.install(|| develop(&source))
    }))
    .map_err(|_| "camera RAW decoder failed on malformed input".to_owned())?
}

/// Validate encoded sensor dimensions, decode the mosaic, and develop oriented RGB.
///
/// # Errors
///
/// Returns unsupported camera, sensor metadata, or development errors.
fn develop(source: &RawSource) -> Result<RgbImage, String> {
    let decoder = rawler::get_decoder(source).map_err(|error| error.to_string())?;
    match decoder.format_hint() {
        FormatHint::DNG => preflight_dng(&*decoder)?,
        FormatHint::CR3 => preflight_cr3(source)?,
        format => return Err(format!("unsupported camera RAW format {format:?}")),
    }
    let raw = decoder
        .raw_image(source, &RawDecodeParams::default(), false)
        .map_err(|error| error.to_string())?;
    let _count = checked_pixel_count(raw.width, raw.height)?;
    let mut developed = RawDevelop::default()
        .develop_intermediate(&raw)
        .map_err(|error| error.to_string())?
        .to_dynamic_image()
        .ok_or_else(|| "camera RAW did not produce a developed raster".to_owned())?;
    let orientation = u8::try_from(raw.orientation.to_u16()).map_err(|error| error.to_string())?;
    if let Some(orientation) = Orientation::from_exif(orientation) {
        developed.apply_orientation(orientation);
    }
    raster_to_rgb(developed, None, None)
}

/// Inspect the Rawler-selected DNG image IFD before unpacking compressed samples.
///
/// # Errors
///
/// Returns missing-raster, invalid dimensions, or channel-count errors.
fn preflight_dng(decoder: &dyn Decoder) -> Result<(), String> {
    let raw_ifd = decoder
        .ifd(WellKnownIFD::Raw)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "DNG has no sensor raster".to_owned())?;
    let value = |tag| {
        raw_ifd
            .get_entry(tag)
            .and_then(|entry| entry.value.get_usize(0).ok().flatten())
            .ok_or_else(|| format!("DNG has no valid {tag:?}"))
    };
    let width = value(TiffCommonTag::ImageWidth)?;
    let height = value(TiffCommonTag::ImageLength)?;
    let channels = value(TiffCommonTag::SamplesPerPixel)?;
    if !matches!(channels, 1 | 3) {
        return Err("DNG sensor must contain one or three channels".to_owned());
    }
    let _count = checked_pixel_count(width, height)?;
    Ok(())
}

/// Inspect each CR3 sensor track's dimensions before Bayer decompression.
///
/// # Errors
///
/// Returns malformed container, missing sensor, or dimension-limit errors.
fn preflight_cr3(source: &RawSource) -> Result<(), String> {
    let container = Bmff::new_buf(source.buf()).map_err(|error| error.to_string())?;
    let mut tracks = container
        .filebox
        .moov
        .traks
        .iter()
        .filter_map(|track| track.mdia.minf.stbl.stsd.craw.as_ref()?.cmp1.as_ref());
    let first = tracks
        .next()
        .ok_or_else(|| "CR3 has no sensor raster".to_owned())?;
    for raster in std::iter::once(first).chain(tracks) {
        let width = usize::try_from(raster.f_width).map_err(|error| error.to_string())?;
        let height = usize::try_from(raster.f_height).map_err(|error| error.to_string())?;
        let _count = checked_pixel_count(width, height)?;
    }
    Ok(())
}
