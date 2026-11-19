//! Owned camera RAW decode and complete development on a bounded local pool.

use std::fs::File;
use std::io::Read;
use std::num::NonZeroUsize;
use std::panic;
use std::path::Path;
use std::sync::{Arc, LazyLock};
use std::thread;

use image::{DynamicImage, ImageBuffer, metadata::Orientation};
use rawler::decoders::{Decoder, FormatHint, RawDecodeParams, WellKnownIFD};
use rawler::formats::bmff::Bmff;
use rawler::imgop::convert_from_f32_scaled_u16;
use rawler::imgop::develop::{Intermediate, RawDevelop};
use rawler::pixarray::RgbF32;
use rawler::rawsource::RawSource;
use rawler::tags::TiffCommonTag;
use rayon::prelude::{ParallelIterator, ParallelSliceMut};

use super::{RgbImage, checked_pixel_count, raster_to_rgb};

#[cfg(test)]
mod tests;

/// Maximum owned encoded RAW bytes, separate from raster allocation limits.
const MAX_RAW_BYTES: u64 = 256 * 1024 * 1024;

/// Limit worker stacks and decoder scratch on high-core hosts.
const MAX_RAW_WORKERS: usize = 15;

/// Channel count per temporary code buffer, limited to 128 KiB of RGB16 codes.
const RAW_PACKING_CHUNK: usize = 64 * 1024;

/// Avoid scheduling work for a developed raster with fewer than sixteen full chunks.
const RAW_PARALLEL_MIN_SAMPLES: usize = 16 * RAW_PACKING_CHUNK;

/// One process-wide pool bounds nested decoder and developer parallelism.
static RAW_POOL: LazyLock<Result<rayon::ThreadPool, String>> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(raw_worker_count(thread::available_parallelism().ok()))
        .thread_name(|index| format!("pancake-raw-{index}"))
        .build()
        .map_err(|error| error.to_string())
});

/// Keep a CPU available to the caller and fall back to one worker without a host budget.
fn raw_worker_count(available: Option<NonZeroUsize>) -> usize {
    available.map_or(1, |threads| {
        threads.get().saturating_sub(1).clamp(1, MAX_RAW_WORKERS)
    })
}

/// Validate the encoded source size and execute full camera-RAW development.
///
/// # Errors
///
/// Returns file-read, size-limit, worker-pool, or camera decode errors.
pub(super) fn decode(file: File, path: &Path) -> Result<RgbImage, String> {
    let encoded_size = file.metadata().map_err(|error| error.to_string())?.len();
    if encoded_size > MAX_RAW_BYTES {
        return Err("compressed RAW source exceeds the byte limit".to_owned());
    }
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
        pool.install(|| develop(&source, Some(pool)))
    }))
    .map_err(|_| "camera RAW decoder failed on malformed input".to_owned())?
}

/// Validate encoded sensor dimensions, decode the mosaic, and develop oriented RGB.
///
/// # Errors
///
/// Returns unsupported camera, sensor metadata, or development errors.
fn develop(source: &RawSource, pool: Option<&rayon::ThreadPool>) -> Result<RgbImage, String> {
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
    let intermediate = RawDevelop::default()
        .develop_intermediate(&raw)
        .map_err(|error| error.to_string())?;
    let orientation = u8::try_from(raw.orientation.to_u16()).map_err(|error| error.to_string())?;
    developed_rgb(intermediate, Orientation::from_exif(orientation), pool)
}

/// Convert developed camera samples into the oriented working raster.
///
/// # Errors
///
/// Returns a missing raster or invalid working-frame error.
fn developed_rgb(
    intermediate: Intermediate,
    orientation: Option<Orientation>,
    pool: Option<&rayon::ThreadPool>,
) -> Result<RgbImage, String> {
    let intermediate = match (intermediate, orientation) {
        (Intermediate::ThreeColor(pixels), None | Some(Orientation::NoTransforms)) => {
            return normal_developed_rgb(pixels, pool);
        }
        (intermediate, _) => intermediate,
    };
    let mut developed = developed_raster(intermediate)
        .ok_or_else(|| "camera RAW did not produce a developed raster".to_owned())?;
    if let Some(orientation) = orientation {
        developed.apply_orientation(orientation);
    }
    raster_to_rgb(developed, None, None)
}

/// Reuse untransformed RGB storage while retaining the pinned RGB16 working values.
///
/// # Errors
///
/// Returns a sample count or dimensions inconsistent with the developed raster.
fn normal_developed_rgb(
    pixels: RgbF32,
    pool: Option<&rayon::ThreadPool>,
) -> Result<RgbImage, String> {
    let dimensions = pixels.dim();
    let mut samples = pixels.into_inner().into_flattened();
    for_each_packing_chunk(&mut samples, pool, pack_raw_rgb16_chunk)?;
    RgbImage::from_normalized_u16(dimensions.w, dimensions.h, samples, None)
}

/// Process disjoint slices in place, parallelizing only inside the installed RAW pool.
///
/// # Errors
///
/// Returns a packing error from a chunk before publishing a working raster.
fn for_each_packing_chunk(
    samples: &mut [f32],
    pool: Option<&rayon::ThreadPool>,
    process: impl Fn(&mut [f32]) -> Result<(), String> + Send + Sync,
) -> Result<(), String> {
    if samples.len() >= RAW_PARALLEL_MIN_SAMPLES
        && pool.is_some_and(|pool| {
            pool.current_thread_index().is_some() && pool.current_num_threads() > 1
        })
    {
        samples
            .par_chunks_mut(RAW_PACKING_CHUNK)
            .try_for_each(process)
    } else {
        samples.chunks_mut(RAW_PACKING_CHUNK).try_for_each(process)
    }
}

/// Preserve the pinned Rawler quantization in one bounded working slice.
///
/// # Errors
///
/// Returns a mismatched code count from the quantizer.
fn pack_raw_rgb16_chunk(chunk: &mut [f32]) -> Result<(), String> {
    let codes = convert_from_f32_scaled_u16(chunk, 0, u16::MAX);
    normalize_raw_codes(chunk, &codes)
}

/// Normalize each developed code in place only if it covers the entire slice.
///
/// # Errors
///
/// Rejects a code count that would leave gaps or truncate developed RGB.
fn normalize_raw_codes(chunk: &mut [f32], codes: &[u16]) -> Result<(), String> {
    if chunk.len() != codes.len() {
        return Err("RAW quantizer code count does not match the developed chunk".to_owned());
    }
    let white = f32::from(u16::MAX);
    for (sample, &code) in chunk.iter_mut().zip(codes) {
        *sample = f32::from(code) / white;
    }
    Ok(())
}

/// Quantize developed RGB in its existing contiguous channel order.
fn developed_raster(intermediate: Intermediate) -> Option<DynamicImage> {
    let Intermediate::ThreeColor(pixels) = intermediate else {
        return intermediate.to_dynamic_image();
    };
    let dimensions = pixels.dim();
    let width = u32::try_from(dimensions.w).ok()?;
    let height = u32::try_from(dimensions.h).ok()?;
    let codes = convert_from_f32_scaled_u16(pixels.pixels().as_flattened(), 0, u16::MAX);
    ImageBuffer::from_raw(width, height, codes).map(DynamicImage::ImageRgb16)
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
