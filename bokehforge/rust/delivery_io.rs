use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use numpy::{
    Element, PyArray1, PyArray3, PyArrayMethods, PyReadonlyArray1, PyReadonlyArrayDyn,
    PyUntypedArrayMethods,
};
use pyo3::exceptions::{PyFileNotFoundError, PyOSError, PyValueError};
use pyo3::prelude::*;
use rayon::prelude::*;
use tiff::decoder::{ChunkType, Decoder, DecodingResult, Limits};
use tiff::tags::{ByteOrder, CompressionMethod, Tag};
use tiff::{ColorType, TiffError, TiffFormatError};

const IMAGE_DEPTH: Tag = Tag::Unknown(32997);
const PARALLEL_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

// Each strip decoder seeks independently within the same open file.
struct PositionalFile<'a> {
    file: &'a File,
    position: u64,
    length: u64,
}

impl Read for PositionalFile<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        #[cfg(unix)]
        let count = std::os::unix::fs::FileExt::read_at(self.file, buffer, self.position)?;
        #[cfg(windows)]
        let count = std::os::windows::fs::FileExt::seek_read(self.file, buffer, self.position)?;
        self.position += count as u64;
        Ok(count)
    }
}

impl Seek for PositionalFile<'_> {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(offset) => Some(offset),
            SeekFrom::End(offset) => self.length.checked_add_signed(offset),
            SeekFrom::Current(offset) => self.position.checked_add_signed(offset),
        };
        self.position = position.ok_or(io::ErrorKind::InvalidInput)?;
        Ok(self.position)
    }
}

enum ReadError {
    Open(io::Error),
    Decode(TiffError),
    Delivery(String),
}

impl From<TiffError> for ReadError {
    fn from(error: TiffError) -> Self {
        Self::Decode(error)
    }
}

impl ReadError {
    fn into_pyerr(self, path: PathBuf) -> PyErr {
        let path = std::path::absolute(&path).unwrap_or(path);
        match self {
            Self::Open(error) => {
                let args = (
                    error.raw_os_error(),
                    error.to_string(),
                    path.into_os_string(),
                );
                if error.kind() == io::ErrorKind::NotFound {
                    PyFileNotFoundError::new_err(args)
                } else {
                    PyOSError::new_err(args)
                }
            }
            Self::Decode(TiffError::FormatError(TiffFormatError::ImageFileDirectoryNotFound)) => {
                PyValueError::new_err(format!(
                    "The delivery has 0 TIFF pages. The benchmark requires one page. ({})",
                    path.display()
                ))
            }
            Self::Decode(error) => PyValueError::new_err(format!(
                "Unable to decode TIFF {} ({error})",
                path.display()
            )),
            Self::Delivery(message) => {
                PyValueError::new_err(format!("{message} ({})", path.display()))
            }
        }
    }
}

fn page_count(decoder: &mut Decoder<impl Read + Seek>) -> Result<usize, TiffError> {
    let mut next = decoder.ifd_pointer();
    let mut seen = HashSet::new();
    while let Some(pointer) = next {
        if !seen.insert(pointer.0) {
            return Err(TiffFormatError::CycleInOffsets.into());
        }
        next = decoder.read_directory(pointer)?.next();
    }
    Ok(seen.len())
}

fn photometric_name(photometric: u16) -> String {
    match photometric {
        0 => "MINISWHITE",
        1 => "MINISBLACK",
        2 => "RGB",
        3 => "PALETTE",
        4 => "MASK",
        5 => "SEPARATED",
        6 => "YCBCR",
        8 => "CIELAB",
        9 => "ICCLAB",
        10 => "ITULAB",
        value => return value.to_string(),
    }
    .to_owned()
}

fn read_parallel_strips(
    file: &File,
    length: u64,
    strips: usize,
    strip_bytes: usize,
    pixels: usize,
    limits: Limits,
) -> Result<DecodingResult, TiffError> {
    let mut result = DecodingResult::F32(vec![0.0; pixels * 3]);
    let mut result_buffer = result.as_buffer(0);
    let output = result_buffer.as_bytes_mut();
    crate::parallel(|| {
        let group_strips = strips.div_ceil(rayon::current_num_threads());
        output
            .par_chunks_mut(group_strips * strip_bytes)
            .enumerate()
            .try_for_each(|(group, bytes)| {
                let reader = PositionalFile {
                    file,
                    position: 0,
                    length,
                };
                let mut decoder = Decoder::new(BufReader::new(reader))?.with_limits(limits.clone());
                for (index, strip) in bytes.chunks_mut(strip_bytes).enumerate() {
                    decoder.read_chunk_bytes((group * group_strips + index) as u32, strip)?;
                }
                Ok::<(), TiffError>(())
            })
    })?;
    Ok(result)
}

fn read_contiguous_strip(
    file: &File,
    position: u64,
    data_bytes: usize,
) -> Result<DecodingResult, TiffError> {
    const CHUNK: usize = 8 * 1024 * 1024;
    let length = file.metadata()?.len();
    if position > length || data_bytes as u64 > length - position {
        return Err(TiffError::IoError(io::ErrorKind::UnexpectedEof.into()));
    }
    let mut result = DecodingResult::F32(vec![0.0; data_bytes / 4]);
    let mut buffer = result.as_buffer(0);
    crate::parallel(|| {
        buffer
            .as_bytes_mut()
            .par_chunks_mut(CHUNK)
            .enumerate()
            .try_for_each(|(index, chunk)| {
                let position = position + (index * CHUNK) as u64;
                PositionalFile {
                    file,
                    position,
                    length,
                }
                .read_exact(chunk)
            })
    })?;
    Ok(result)
}

fn decode_delivery(path: &Path) -> Result<Option<(DecodingResult, [usize; 3])>, ReadError> {
    let file = File::open(path).map_err(ReadError::Open)?;
    let mut limits = Limits::default();
    // An uncompressed delivery can store the whole image in one strip.
    limits.intermediate_buffer_size = limits.decoding_buffer_size;
    let mut decoder = Decoder::new(BufReader::new(&file))?.with_limits(limits.clone());
    if decoder.more_images() {
        let pages = page_count(&mut decoder)?;
        return Err(ReadError::Delivery(format!(
            "The delivery has {pages} TIFF pages. The benchmark requires one page."
        )));
    }

    let photometric = decoder.get_tag_unsigned::<u16>(Tag::PhotometricInterpretation)?;
    let samples = decoder
        .find_tag_unsigned::<u16>(Tag::SamplesPerPixel)?
        .unwrap_or(1);
    let planar = decoder
        .find_tag_unsigned::<u16>(Tag::PlanarConfiguration)?
        .unwrap_or(1);
    let depth = decoder.find_tag_unsigned::<u32>(IMAGE_DEPTH)?.unwrap_or(1);
    if photometric != 2 || samples != 3 || planar != 1 || depth != 1 {
        let axes = match (photometric == 2 || samples > 1, planar == 1, depth == 1) {
            (false, _, true) => "YX",
            (false, _, false) => "ZYX",
            (true, true, true) => "YXS",
            (true, true, false) => "ZYXS",
            (true, false, true) => "SYX",
            (true, false, false) => "SZYX",
        };
        return Err(ReadError::Delivery(format!(
            "The benchmark requires an interleaved TIFF with three RGB channels per pixel. The TIFF uses {} photometric interpretation, {samples} samples per pixel, and {axes} axis order.",
            photometric_name(photometric)
        )));
    }
    let orientation = decoder
        .find_tag_unsigned::<u16>(Tag::Orientation)?
        .unwrap_or(1);
    if orientation != 1 {
        return Err(ReadError::Delivery(format!(
            "The TIFF has orientation {orientation}. The benchmark requires orientation 1 (top-left)."
        )));
    }

    let compression = CompressionMethod::from_u16_exhaustive(
        decoder
            .find_tag_unsigned::<u16>(Tag::Compression)?
            .unwrap_or(1),
    );
    let bits = decoder
        .find_tag_unsigned_vec::<u16>(Tag::BitsPerSample)?
        .unwrap_or_default();
    let fill_order = decoder
        .find_tag_unsigned::<u16>(Tag::FillOrder)?
        .unwrap_or(1);
    // Leave uncommon representations and external codecs to the established
    // decoder. In particular, JPEG/WebP sample values can vary by decoder.
    if !matches!(
        compression,
        CompressionMethod::None
            | CompressionMethod::Deflate
            | CompressionMethod::OldDeflate
            | CompressionMethod::LZW
            | CompressionMethod::PackBits
            | CompressionMethod::ZSTD
    ) || bits.is_empty()
        || bits.iter().any(|bit| !matches!(bit, 8 | 16 | 32 | 64))
        || fill_order != 1
    {
        return Ok(None);
    }

    let strips = decoder.get_chunk_type() == ChunkType::Strip;
    let (offset_tag, counts_tag) = if strips {
        (Tag::StripOffsets, Tag::StripByteCounts)
    } else {
        (Tag::TileOffsets, Tag::TileByteCounts)
    };
    let offsets = decoder
        .find_tag_unsigned_vec::<u64>(offset_tag)?
        .unwrap_or_default();
    let counts = decoder
        .find_tag_unsigned_vec::<u64>(counts_tag)?
        .unwrap_or_default();
    if offsets.contains(&0) || counts.contains(&0) {
        // Sparse TIFF chunks have an implicit fill value in tifffile.
        return Ok(None);
    }

    let (width, height) = decoder.dimensions()?;
    let shape = [height as usize, width as usize, 3];
    let format = decoder
        .find_tag_unsigned_vec::<u16>(Tag::SampleFormat)?
        .unwrap_or_default();
    if strips
        && decoder.colortype()? == ColorType::RGB(32)
        && !format.is_empty()
        && format.iter().all(|&item| item == 3)
    {
        let data_bytes = (u64::from(width) * u64::from(height))
            .checked_mul(12)
            .ok_or(TiffError::IntSizeError)?;
        if data_bytes > limits.decoding_buffer_size as u64 {
            return Ok(None);
        }
        let predictor = decoder
            .find_tag_unsigned::<u16>(Tag::Predictor)?
            .unwrap_or(1);
        if data_bytes >= PARALLEL_IMAGE_BYTES
            && compression == CompressionMethod::None
            && predictor == 1
            && decoder.byte_order() == ByteOrder::native()
            && counts == [data_bytes]
            && offsets.len() == 1
        {
            let samples = read_contiguous_strip(&file, offsets[0], data_bytes as usize)?;
            return Ok(Some((samples, shape)));
        }
        if compression != CompressionMethod::None
            && data_bytes >= PARALLEL_IMAGE_BYTES
            && offsets.len() >= 8
        {
            let strip_rows = decoder.chunk_dimensions().1.min(height);
            let strip_bytes = width as usize * strip_rows as usize * 12;
            let length = file.metadata().map_err(TiffError::from)?.len();
            let samples = read_parallel_strips(
                &file,
                length,
                offsets.len(),
                strip_bytes,
                data_bytes as usize / 12,
                limits,
            )?;
            return Ok(Some((samples, shape)));
        }
    }
    Ok(Some((decoder.read_image()?, shape)))
}

fn into_array<'py, T: Element>(
    py: Python<'py>,
    samples: Vec<T>,
    shape: [usize; 3],
) -> PyResult<Bound<'py, PyAny>> {
    Ok(PyArray1::from_vec(py, samples).reshape(shape)?.into_any())
}

#[pyfunction]
pub(crate) fn read_delivery_tiff(
    py: Python<'_>,
    path: PathBuf,
) -> PyResult<Option<Bound<'_, PyAny>>> {
    let result = py.detach(|| decode_delivery(&path));
    let (data, shape) = match result {
        Ok(Some(decoded)) => decoded,
        Ok(None)
        | Err(ReadError::Decode(
            TiffError::UnsupportedError(_)
            | TiffError::LimitsExceeded
            | TiffError::FormatError(TiffFormatError::UnknownPredictor(_)),
        )) => return Ok(None),
        Err(error) => return Err(error.into_pyerr(path)),
    };
    match data {
        DecodingResult::U8(samples) => into_array(py, samples, shape),
        DecodingResult::U16(samples) => into_array(py, samples, shape),
        DecodingResult::U32(samples) => into_array(py, samples, shape),
        DecodingResult::U64(samples) => into_array(py, samples, shape),
        DecodingResult::I8(samples) => into_array(py, samples, shape),
        DecodingResult::I16(samples) => into_array(py, samples, shape),
        DecodingResult::I32(samples) => into_array(py, samples, shape),
        DecodingResult::I64(samples) => into_array(py, samples, shape),
        DecodingResult::F16(samples) => into_array(py, samples, shape),
        DecodingResult::F32(samples) => into_array(py, samples, shape),
        DecodingResult::F64(samples) => into_array(py, samples, shape),
    }
    .map(Some)
}

fn expand_samples(source: &[u8], output: &mut [f32], channels: usize, lookup: &[f32; 256]) {
    match channels {
        1 | 2 => {
            for (input, pixel) in source
                .chunks_exact(channels)
                .zip(output.as_chunks_mut::<3>().0)
            {
                pixel.fill(lookup[usize::from(input[0])]);
            }
        }
        3 => {
            for (&sample, value) in source.iter().zip(output) {
                *value = lookup[usize::from(sample)];
            }
        }
        4 => {
            for (input, pixel) in source
                .as_chunks::<4>()
                .0
                .iter()
                .zip(output.as_chunks_mut::<3>().0)
            {
                pixel[0] = lookup[usize::from(input[0])];
                pixel[1] = lookup[usize::from(input[1])];
                pixel[2] = lookup[usize::from(input[2])];
            }
        }
        _ => unreachable!("validated PNG layout"),
    }
}

#[pyfunction]
fn expand_png_rgb8<'py>(
    py: Python<'py>,
    pixels: PyReadonlyArrayDyn<'py, u8>,
    lookup: PyReadonlyArray1<'py, f32>,
) -> PyResult<Bound<'py, PyArray3<f32>>> {
    let (height, width, channels) = match *pixels.shape() {
        [height, width] => (height, width, 1),
        [height, width, channels @ 2..=4] => (height, width, channels),
        _ => {
            return Err(PyValueError::new_err(
                "Expected a grayscale or RGB byte image",
            ));
        }
    };
    let source = crate::c_slice(&pixels)?;
    let lookup: &[f32; 256] = crate::c_slice(&lookup)?
        .try_into()
        .map_err(|_| PyValueError::new_err("Expected 256 transfer values"))?;
    let rgb_len = (source.len() / channels)
        .checked_mul(3)
        .ok_or_else(|| PyValueError::new_err("PNG dimensions are too large"))?;
    let rgb = py.detach(|| {
        const CHUNK_PIXELS: usize = 32 * 1024;
        let mut output = vec![0.0; rgb_len];
        if rgb_len >= 3 * CHUNK_PIXELS {
            crate::parallel(|| {
                source
                    .par_chunks(channels * CHUNK_PIXELS)
                    .zip(output.par_chunks_mut(3 * CHUNK_PIXELS))
                    .for_each(|(source, output)| expand_samples(source, output, channels, lookup));
            });
        } else {
            expand_samples(source, &mut output, channels, lookup);
        }
        output
    });
    PyArray1::from_vec(py, rgb).reshape([height, width, 3])
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(read_delivery_tiff, module)?)?;
    module.add_function(wrap_pyfunction!(expand_png_rgb8, module)?)
}
