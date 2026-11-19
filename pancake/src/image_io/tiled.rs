//! Typed streaming import of tiled DEFLATE RGB16 TIFFs.

use std::cell::Cell;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::thread;

use tiff::decoder::{ChunkType, Decoder, DecodingResult, DecodingSampleType};
use tiff::tags::Tag;

use super::{RgbImage, checked_pixel_count, profile_from_tiff};

#[cfg(test)]
mod tests;

/// Keep a reusable decoded tile to six MiB, with larger tiles using native raster import.
const MAX_TILE_PIXELS: usize = 1_048_576;

/// Avoid thread and codec setup for short decodes.
const PARALLEL_MIN_PIXELS: usize = 1_048_576;

/// At most 24 MiB of tile scratch, with the caller participating as one lane.
const MAX_TIFF_WORKERS: usize = 4;

/// Only one import may start extra tile threads across the process.
static PARALLEL_TIFF: Mutex<()> = Mutex::new(());

/// Start and exclusive end of one validated compressed tile.
type TileSpan = (u64, u64);

/// One pinned TIFF decoder with its own logical, bounded and buffered file cursor.
type TileDecoder<'source, 'file> = Decoder<BufReader<TileInput<'source, &'file File>>>;

/// The original opened file shared by every production tile decoder.
type FileSource<'file> = PositionalInput<&'file File>;

/// A seekable input shared across independently buffered decode lanes.
#[derive(Debug)]
struct PositionalInput<R> {
    /// Serialize only the seek and compressed-byte read on the shared input.
    input: Mutex<R>,
    /// Initial source size used to validate offsets and resolve logical end seeks.
    length: u64,
}

impl<'file> PositionalInput<&'file File> {
    /// Record the size of the existing input handle.
    /// # Errors
    /// Fails if the original handle cannot report file metadata.
    fn new(input: &'file File) -> io::Result<Self> {
        Ok(Self::with_length(input, input.metadata()?.len()))
    }

    /// Parse a TIFF directory using an independent buffer over the opened input.
    /// # Errors
    /// Returns directory parse and source I/O failures.
    fn decoder<'source>(
        &'source self,
        span: &'source Cell<Option<TileSpan>>,
    ) -> tiff::TiffResult<TileDecoder<'source, 'file>> {
        Decoder::new(BufReader::with_capacity(65_536, self.cursor(span)))
    }
}

impl<R: Read + Seek> PositionalInput<R> {
    /// Pair the source with its validated byte length.
    const fn with_length(input: R, length: u64) -> Self {
        Self {
            input: Mutex::new(input),
            length,
        }
    }

    /// Create an independent logical cursor with an initially caller-selected span.
    const fn cursor<'source>(
        &'source self,
        span: &'source Cell<Option<TileSpan>>,
    ) -> TileInput<'source, R> {
        TileInput {
            source: self,
            position: 0,
            span,
        }
    }

    /// Seek and read together, leaving other cursors' logical positions unaffected.
    /// # Errors
    /// Returns a lock or file I/O failure.
    fn read_at(&self, position: u64, buffer: &mut [u8]) -> io::Result<usize> {
        let mut input = self
            .input
            .lock()
            .map_err(|_| io::Error::other("TIFF source lock failed"))?;
        let _ = input.seek(SeekFrom::Start(position))?;
        input.read(buffer)
    }
}

/// Bounds TIFF decompression reads to the current declared compressed tile.
#[derive(Debug)]
struct TileInput<'source, R> {
    /// Original source handle, reused for metadata, tiles and ICC.
    source: &'source PositionalInput<R>,
    /// Logical position for this lane's compressed input buffer.
    position: u64,
    /// Apply the current cap before reading any bytes into the lane buffer.
    span: &'source Cell<Option<TileSpan>>,
}

impl<R: Read + Seek> Read for TileInput<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let bound = if let Some((start, end)) = self.span.get() {
            if self.position < start || self.position > end {
                return Err(io::Error::other("read outside the compressed TIFF tile"));
            }
            let remaining = end.saturating_sub(self.position);
            usize::try_from(remaining)
                .unwrap_or(usize::MAX)
                .min(buffer.len())
        } else {
            buffer.len()
        };
        if bound == 0 {
            return Ok(0);
        }
        let count = self.source.read_at(
            self.position,
            buffer
                .get_mut(..bound)
                .ok_or_else(|| io::Error::other("invalid TIFF read size"))?,
        )?;
        self.position = self
            .position
            .checked_add(u64::try_from(count).map_err(io::Error::other)?)
            .ok_or_else(|| io::Error::other("TIFF stream position overflowed"))?;
        Ok(count)
    }
}

impl<R> Seek for TileInput<'_, R> {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let position = match position {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::Current(delta) => self.position.checked_add_signed(delta),
            SeekFrom::End(delta) => self.source.length.checked_add_signed(delta),
        }
        .ok_or_else(|| io::Error::other("TIFF logical position overflowed"))?;
        if self.span.get().is_some_and(|(start, _)| start != position) {
            return Err(io::Error::other(
                "TIFF tile start does not match its validated span",
            ));
        }
        self.position = position;
        Ok(self.position)
    }
}

/// Unoriented unsigned contiguous sample layout eligible for bounded typed decoding.
fn supports_streaming(decoder: &mut TileDecoder<'_, '_>) -> bool {
    decoder.get_chunk_type() == ChunkType::Tile
        && matches!(decoder.colortype(), Ok(tiff::ColorType::RGB(16)))
        && matches!(decoder.find_tag_unsigned::<u16>(Tag::SamplesPerPixel), Ok(Some(3)))
        // TIFF tag values for Deflate and OldDeflate as recognized by the native decoder.
        && matches!(decoder.find_tag_unsigned::<u16>(Tag::Compression), Ok(Some(8 | 32_946)))
        && matches!(decoder.find_tag_unsigned::<u16>(Tag::Orientation), Ok(None | Some(1)))
        && decoder.image_buffer_layout().is_ok_and(|layout| {
            layout.planes == 1 && layout.sample_type == Some(DecodingSampleType::U16)
        })
}

/// Tile and output geometry kept in elements unless passed to the TIFF stride API.
#[derive(Debug, Eq, PartialEq)]
struct Layout {
    /// Pixels across the full raster.
    width: usize,
    /// Pixels down the full raster.
    height: usize,
    /// Pixels across a full encoded tile before edge padding is removed.
    tile_width: usize,
    /// Rows in a full tile before edge padding is removed.
    tile_height: usize,
    /// Number of tile columns, including the right edge.
    columns: usize,
}

impl Layout {
    /// Read exact tile dimensions and verify the optimized scratch limit.
    ///
    /// # Errors
    ///
    /// Rejects invalid dimensions or an unaddressable raster.
    fn read(decoder: &mut TileDecoder<'_, '_>) -> Result<Option<Self>, String> {
        let (width, height) = decoder.dimensions().map_err(|error| error.to_string())?;
        let width = usize::try_from(width).map_err(|error| error.to_string())?;
        let height = usize::try_from(height).map_err(|error| error.to_string())?;
        let _pixels = checked_pixel_count(width, height)?;
        let tile_width = decoder
            .get_tag_unsigned::<usize>(Tag::TileWidth)
            .map_err(|error| error.to_string())?;
        let tile_height = decoder
            .get_tag_unsigned::<usize>(Tag::TileLength)
            .map_err(|error| error.to_string())?;
        let tile_pixels = tile_width
            .checked_mul(tile_height)
            .ok_or_else(|| "TIFF tile area overflowed".to_owned())?;
        if tile_pixels == 0 {
            return Err("TIFF tile dimensions must be nonzero".to_owned());
        }
        Ok((tile_pixels <= MAX_TILE_PIXELS).then(|| Self {
            width,
            height,
            tile_width,
            tile_height,
            columns: width.div_ceil(tile_width),
        }))
    }

    /// Count complete and partial tile rows in the raster.
    const fn tile_rows(&self) -> usize {
        self.height.div_ceil(self.tile_height)
    }

    /// Workspace size of a full row of tiles, in RGB elements.
    /// # Errors
    /// Rejects a band that does not fit in an addressable slice.
    fn rgb_band_size(&self) -> Result<usize, String> {
        rgb_samples(self.width)?
            .checked_mul(self.tile_height)
            .ok_or_else(|| "TIFF working band size overflowed".to_owned())
    }

    /// Copy the available columns of a tile into the interleaved float output.
    ///
    /// # Errors
    ///
    /// Reports invalid tile dimensions, strides or decoded sample type and extent.
    fn read_tile(
        &self,
        decoder: &mut TileDecoder<'_, '_>,
        scratch: &mut DecodingResult,
        samples: &mut [f32],
        index: usize,
    ) -> Result<(), String> {
        let rgb_stride = rgb_samples(self.width)?;
        let column = index
            .checked_rem(self.columns)
            .and_then(|column| column.checked_mul(self.tile_width))
            .ok_or_else(|| "TIFF tile column overflowed".to_owned())?;
        let width = self.width.saturating_sub(column).min(self.tile_width);
        let height = samples
            .len()
            .checked_div(rgb_stride)
            .ok_or_else(|| "TIFF working row has an invalid stride".to_owned())?;
        let start = rgb_samples(column)?;
        let tile_stride = rgb_samples(width)?;
        let tile_bytes = tile_stride
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| "TIFF tile stride overflowed".to_owned())?;
        let chunk = u32::try_from(index).map_err(|error| error.to_string())?;
        decoder
            .read_chunk_to_buffer(scratch, chunk, tile_bytes)
            .map_err(|error| format!("TIFF tile {index}: {error}"))?;
        let DecodingResult::U16(codes) = scratch else {
            return Err("TIFF tile sample type changed".to_owned());
        };
        if codes.len() != height.saturating_mul(tile_stride) {
            return Err("TIFF tile decoded sample count changed".to_owned());
        }
        for (output, native) in samples
            .chunks_exact_mut(rgb_stride)
            .zip(codes.chunks_exact(tile_stride))
        {
            let target = output
                .get_mut(start..start.saturating_add(tile_stride))
                .ok_or_else(|| "TIFF tile output lies outside its raster".to_owned())?;
            for (sample, &code) in target.iter_mut().zip(native) {
                *sample = f32::from(code) / f32::from(u16::MAX);
            }
        }
        Ok(())
    }
}

/// Convert a pixel count or row width to interleaved RGB elements.
///
/// # Errors
///
/// Rejects a count that cannot hold three samples per pixel.
fn rgb_samples(pixels: usize) -> Result<usize, String> {
    pixels
        .checked_mul(3)
        .ok_or_else(|| "TIFF RGB sample count overflowed".to_owned())
}

/// Validate the entire tile directory before decoding any compressed payload.
///
/// # Errors
///
/// Rejects missing tile records and empty, overflowing or out-of-file ranges.
fn tile_spans(
    decoder: &mut TileDecoder<'_, '_>,
    layout: &Layout,
    source_len: u64,
) -> Result<Vec<TileSpan>, String> {
    let offsets = decoder
        .get_tag_u64_vec(Tag::TileOffsets)
        .map_err(|error| error.to_string())?;
    let counts = decoder
        .get_tag_u64_vec(Tag::TileByteCounts)
        .map_err(|error| error.to_string())?;
    let tiles = layout.columns.saturating_mul(layout.tile_rows());
    if offsets.len() != tiles || counts.len() != tiles {
        return Err("TIFF tile directory size does not match the image".to_owned());
    }
    offsets
        .into_iter()
        .zip(counts)
        .map(|(offset, count)| {
            offset
                .checked_add(count)
                .filter(|&end| count != 0 && end <= source_len)
                .map(|end| (offset, end))
                .ok_or_else(|| "compressed TIFF tile lies outside the source file".to_owned())
        })
        .collect()
}

/// Decode adjacent complete tile rows into their exclusive working sample range.
/// # Errors
/// Returns a malformed tile, I/O error or inconsistent working range.
fn read_rows(
    layout: &Layout,
    decoder: &mut TileDecoder<'_, '_>,
    span: &Cell<Option<TileSpan>>,
    spans: &[TileSpan],
    samples: &mut [f32],
    first_tile: usize,
) -> Result<(), String> {
    let mut scratch = DecodingResult::U16(Vec::new());
    let rows = spans.chunks(layout.columns);
    let bands = samples.chunks_mut(layout.rgb_band_size()?);
    if rows.len() != bands.len() {
        return Err("TIFF tile rows do not match their working bands".to_owned());
    }
    let mut index = first_tile;
    for (row, band) in rows.zip(bands) {
        for &extent in row {
            span.set(Some(extent));
            layout.read_tile(decoder, &mut scratch, band, index)?;
            index = index.saturating_add(1);
        }
    }
    span.set(None);
    Ok(())
}

/// Limit lanes by useful row count, host affinity and aggregate scratch budget.
fn lane_count(pixels: usize, tile_rows: usize, host_cpus: usize) -> usize {
    if pixels < PARALLEL_MIN_PIXELS {
        1
    } else {
        tile_rows.min(host_cpus).clamp(1, MAX_TIFF_WORKERS)
    }
}

/// Read and validate a worker's view of the source before filling its band.
/// # Errors
/// Returns directory changes, malformed compressed data and source I/O failures.
fn read_worker_band(
    source: &FileSource<'_>,
    layout: &Layout,
    first_tile: usize,
    spans: &[TileSpan],
    samples: &mut [f32],
) -> Result<(), String> {
    let span = Cell::new(None);
    let mut decoder = source.decoder(&span).map_err(|error| error.to_string())?;
    if !supports_streaming(&mut decoder) || Layout::read(&mut decoder)?.as_ref() != Some(layout) {
        return Err("TIFF layout changed during tile decode".to_owned());
    }
    let parsed = tile_spans(&mut decoder, layout, source.length)?;
    if parsed.get(first_tile..first_tile.saturating_add(spans.len())) != Some(spans) {
        return Err("TIFF compressed spans changed during tile decode".to_owned());
    }
    read_rows(layout, &mut decoder, &span, spans, samples, first_tile)
}

/// Use disjoint working slices and a process-wide lease to decode adjacent bands.
///
/// A busy lease or unavailable thread falls back to serial after all spawned work joins.
///
/// # Errors
/// Returns actual decode and worker failures in tile order.
fn read_parallel(
    source: &FileSource<'_>,
    layout: &Layout,
    spans: &[TileSpan],
    samples: &mut [f32],
    lanes: usize,
) -> Result<bool, String> {
    let Ok(_lease) = PARALLEL_TIFF.try_lock() else {
        return Ok(false);
    };
    let rows_per_lane = layout.tile_rows().div_ceil(lanes);
    let groups = spans.chunks(layout.columns.saturating_mul(rows_per_lane));
    let bands = samples.chunks_mut(layout.rgb_band_size()?.saturating_mul(rows_per_lane));
    if groups.len() != bands.len() {
        return Err("TIFF worker groups do not match their working bands".to_owned());
    }
    let mut work = groups.zip(bands);
    let Some((first_spans, first_samples)) = work.next() else {
        return Err("TIFF has no worker bands".to_owned());
    };
    thread::scope(|scope| {
        let mut workers = Vec::with_capacity(lanes.saturating_sub(1));
        let mut start = first_spans.len();
        for (spans, samples) in work {
            let first_tile = start;
            start = start.saturating_add(spans.len());
            let worker = thread::Builder::new()
                .name("pancake-tiff".to_owned())
                .spawn_scoped(scope, move || {
                    read_worker_band(source, layout, first_tile, spans, samples)
                });
            workers.push(worker);
        }
        let mut result = read_worker_band(source, layout, 0, first_spans, first_samples);
        let mut complete = true;
        for worker in workers {
            if let Ok(worker) = worker {
                let joined = worker
                    .join()
                    .unwrap_or_else(|_| Err("TIFF worker terminated unexpectedly".to_owned()));
                result = result.and(joined);
            } else {
                complete = false;
            }
        }
        result.map(|()| complete)
    })
}

/// Stream the eligible tiled TIFF directly to normalized working samples.
///
/// # Errors
///
/// Rejects invalid dimensions, truncated or invalid compressed ranges and decode errors.
pub(super) fn decode(input: &File) -> Result<Option<RgbImage>, String> {
    // The caller's original buffer remains quiescent and the fallback rewinds it explicitly.
    let source = FileSource::new(input).map_err(|error| error.to_string())?;
    let span = Cell::new(None);
    let Ok(mut decoder) = source.decoder(&span) else {
        return Ok(None);
    };
    if !supports_streaming(&mut decoder) {
        return Ok(None);
    }
    let Some(layout) = Layout::read(&mut decoder)? else {
        return Ok(None);
    };
    let spans = tile_spans(&mut decoder, &layout, source.length)?;
    let pixels = checked_pixel_count(layout.width, layout.height)?;
    let mut samples = vec![0.0_f32; rgb_samples(pixels)?];
    let host_cpus = thread::available_parallelism().map_or(1, NonZeroUsize::get);
    let lanes = lane_count(pixels, layout.tile_rows(), host_cpus);
    if lanes == 1 || !read_parallel(&source, &layout, &spans, &mut samples, lanes)? {
        read_rows(&layout, &mut decoder, &span, &spans, &mut samples, 0)?;
    }
    // Shape comes from the checked allocation, and zero and normalized u16 samples are finite.
    RgbImage::from_normalized_u16(
        layout.width,
        layout.height,
        samples,
        profile_from_tiff(decoder),
    )
    .map(Some)
}
