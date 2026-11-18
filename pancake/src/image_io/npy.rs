//! Bounded row-major `NumPy` array reader for unsigned and float image samples.

use std::io::{self, Read};

use super::{RgbImage, SampleEncoding, checked_pixel_count, normalize_float_codes};

/// Limit the dictionary scanned before converting any raster samples.
const MAX_HEADER_BYTES: usize = 65_536;

/// Encoded scalar types recognized in `NumPy` image files.
#[derive(Clone, Copy, Debug)]
enum SampleType {
    /// An endian-independent unsigned byte.
    U8,
    /// A little- or big-endian unsigned sixteen-bit value.
    U16(bool),
    /// A little- or big-endian IEEE-754 single-precision float.
    F32(bool),
}

impl SampleType {
    /// Identify dtype by the complete recognized scalar descriptor.
    ///
    /// # Errors
    ///
    /// Returns malformed or unsupported scalar dtype errors.
    fn from_header(header: &str) -> Result<Self, String> {
        let value = scalar_field(header, "descr")?.trim_matches(['\'', '"']);
        match value {
            "|u1" => Ok(Self::U8),
            "<u2" => Ok(Self::U16(true)),
            ">u2" => Ok(Self::U16(false)),
            "<f4" => Ok(Self::F32(true)),
            ">f4" => Ok(Self::F32(false)),
            _ => Err(format!("unsupported NumPy dtype {value:?}")),
        }
    }

    /// Read one complete sample and convert integer code values without quantization.
    ///
    /// # Errors
    ///
    /// Returns a read error if the complete sample is unavailable.
    fn read(self, reader: &mut impl Read) -> io::Result<f32> {
        match self {
            Self::U8 => {
                let mut bytes = [0_u8];
                reader.read_exact(&mut bytes)?;
                let [sample] = bytes;
                Ok(f32::from(sample) / 255.0)
            }
            Self::U16(little) => {
                let mut bytes = [0_u8; 2];
                reader.read_exact(&mut bytes)?;
                let value = if little {
                    u16::from_le_bytes(bytes)
                } else {
                    u16::from_be_bytes(bytes)
                };
                Ok(f32::from(value) / 65_535.0)
            }
            Self::F32(little) => {
                let mut bytes = [0_u8; 4];
                reader.read_exact(&mut bytes)?;
                Ok(if little {
                    f32::from_le_bytes(bytes)
                } else {
                    f32::from_be_bytes(bytes)
                })
            }
        }
    }

    /// Report the native source sample type independent of endian order.
    const fn encoding(self) -> SampleEncoding {
        match self {
            Self::U8 => SampleEncoding::U8,
            Self::U16(_) => SampleEncoding::U16,
            Self::F32(_) => SampleEncoding::Float,
        }
    }
}

/// Source layout before converting array samples to interleaved RGB.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrayLayout {
    /// A two-dimensional grayscale raster.
    GrayscaleArray,
    /// A three-dimensional array with a singleton channel axis.
    SingleChannel,
    /// A three-dimensional array with three RGB channels.
    Rgb,
}

/// Read and validate array metadata before reading row-major grayscale or RGB.
///
/// # Errors
///
/// Returns metadata, shape, raster read, and sample validity errors.
pub(super) fn decode(mut input: impl Read) -> Result<RgbImage, String> {
    let header = read_header(&mut input)?;
    let (width, height, layout) = parse_shape(&header)?;
    let pixel_count = checked_pixel_count(width, height)?;
    let mut samples = Vec::with_capacity(pixel_count.saturating_mul(3));
    let dtype = SampleType::from_header(&header)?;
    for _ in 0..pixel_count {
        let first = dtype.read(&mut input).map_err(|error| error.to_string())?;
        if layout == ArrayLayout::Rgb {
            let second = dtype.read(&mut input).map_err(|error| error.to_string())?;
            let third = dtype.read(&mut input).map_err(|error| error.to_string())?;
            samples.extend([first, second, third]);
        } else {
            samples.extend([first; 3]);
        }
    }
    if input.read(&mut [0_u8]).map_err(|error| error.to_string())? != 0 {
        return Err("NumPy raster contains trailing bytes".to_owned());
    }
    let mut image = RgbImage::new(width, height, samples)?;
    image.sample_encoding = dtype.encoding();
    if layout == ArrayLayout::GrayscaleArray {
        image.mark_grayscale_array();
    }
    if image.sample_encoding == SampleEncoding::Float {
        normalize_float_codes(&mut image.samples);
    }
    Ok(image)
}

/// Read the fixed prefix, checked header length and UTF-8-compatible metadata.
///
/// # Errors
///
/// Returns unsupported version, length, read, or dictionary errors.
fn read_header(input: &mut impl Read) -> Result<String, String> {
    let mut signature = [0_u8; 8];
    input
        .read_exact(&mut signature)
        .map_err(|error| error.to_string())?;
    let length = match signature {
        [0x93, b'N', b'U', b'M', b'P', b'Y', 1, 0] => {
            let mut bytes = [0_u8; 2];
            input
                .read_exact(&mut bytes)
                .map_err(|error| error.to_string())?;
            usize::from(u16::from_le_bytes(bytes))
        }
        [0x93, b'N', b'U', b'M', b'P', b'Y', 2 | 3, 0] => {
            let mut bytes = [0_u8; 4];
            input
                .read_exact(&mut bytes)
                .map_err(|error| error.to_string())?;
            usize::try_from(u32::from_le_bytes(bytes)).map_err(|error| error.to_string())?
        }
        _ => return Err("invalid NumPy format signature or version".to_owned()),
    };
    if length > MAX_HEADER_BYTES {
        return Err("NumPy header exceeds the metadata limit".to_owned());
    }
    let mut header = vec![0_u8; length];
    input
        .read_exact(&mut header)
        .map_err(|error| error.to_string())?;
    let header = String::from_utf8(header).map_err(|error| error.to_string())?;
    if !header.starts_with('{') || !header.ends_with('\n') || !header.trim_end().ends_with('}') {
        return Err("malformed NumPy header dictionary".to_owned());
    }
    Ok(header)
}

/// Locate a simple `NumPy` header key without evaluating Python text.
///
/// # Errors
///
/// Returns an error for a missing or malformed key/value separator.
fn field<'a>(header: &'a str, key: &str) -> Result<&'a str, String> {
    header
        .split_once(&format!("'{key}'"))
        .or_else(|| header.split_once(&format!("\"{key}\"")))
        .and_then(|(_, value)| value.trim_start().strip_prefix(':'))
        .map(str::trim_start)
        .ok_or_else(|| format!("missing NumPy header field {key:?}"))
}

/// Read a scalar value whether or not another entry or trailing comma follows.
/// # Errors
/// Returns a missing or malformed header field error.
fn scalar_field<'a>(header: &'a str, key: &str) -> Result<&'a str, String> {
    let value = field(header, key)?.trim_end().trim_end_matches('}');
    Ok(value
        .split_once(',')
        .map_or(value, |(scalar, _)| scalar)
        .trim())
}

/// Identify a C-order, two- or three-dimensional grayscale/RGB image shape.
///
/// # Errors
///
/// Returns malformed, unsupported order, or unsupported channel errors.
fn parse_shape(header: &str) -> Result<(usize, usize, ArrayLayout), String> {
    if scalar_field(header, "fortran_order")? != "False" {
        return Err("NumPy image must use contiguous row-major order".to_owned());
    }
    let shape = field(header, "shape")?
        .strip_prefix('(')
        .and_then(|suffix| suffix.split_once(')'))
        .ok_or_else(|| "malformed NumPy shape tuple".to_owned())?
        .0;
    let dimensions = shape
        .trim_end_matches(',')
        .split(',')
        .map(|value| value.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    match dimensions.as_slice() {
        [height, width] => Ok((*width, *height, ArrayLayout::GrayscaleArray)),
        [height, width, 1] => Ok((*width, *height, ArrayLayout::SingleChannel)),
        [height, width, 3] => Ok((*width, *height, ArrayLayout::Rgb)),
        _ => Err("NumPy image must be H×W, H×W×1, or H×W×3".to_owned()),
    }
}
