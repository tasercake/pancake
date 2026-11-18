//! Application image input contracts with real temporary source files.

use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use image::codecs::png::PngEncoder;
use image::{DynamicImage, ExtendedColorType, GrayImage, ImageBuffer, ImageEncoder, ImageFormat};

use pancake::image_io::{RgbImage, SampleEncoding, decode};

/// Test results include recoverable fixture construction and application errors.
type TestResult = Result<(), Box<dyn Error>>;

/// An oversized IHDR with a correct CRC and no compressed data.
const OVERSIZED_PNG: [u8; 33] = [
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 1, 0, 0, 0, 1, 0, 0, 8, 2, 0,
    0, 0, 0xe3, 0xe6, 0xa7, 0xb4,
];

/// Check actual test behavior without panicking in a fallible test.
///
/// # Errors
///
/// Returns the specific contract failure when the condition is false.
fn require(condition: bool, diagnostic: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(diagnostic.to_owned()).into())
    }
}

/// Corrupt paths are individually diagnosed without returning a decoded frame.
///
/// # Errors
///
/// Returns construction, unexpected decode success, or diagnostic/immutability errors.
#[test]
fn malformed_file_diagnostics_identify_each_input() -> TestResult {
    let directory = tempfile::tempdir()?;
    for extension in ["png", "dng", "npy", "unknown"] {
        let path = directory.path().join(format!("invalid‐frame.{extension}"));
        let content = b"not a valid encoded image\x00\xff";
        fs::write(&path, content)?;
        let error = decode(&path).err().ok_or_else(|| {
            io::Error::other(format!(
                "malformed {extension} was accepted as a valid frame"
            ))
        })?;
        require(
            error.contains(&*path.to_string_lossy()),
            &format!("missing source path: {error}"),
        )?;
        require(
            fs::read(path)? == content,
            "decoding must preserve the input",
        )?;
    }
    Ok(())
}

/// The pixel limit is applied to the dimensions before requiring a raster.
///
/// # Errors
///
/// Returns construction, unexpected decode success, or dimension diagnostic errors.
#[test]
fn oversized_png_is_rejected_before_pixels_are_read() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("oversized.png");
    fs::write(&path, OVERSIZED_PNG)?;
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("oversized frame was decoded"))?;
    require(
        error.contains("limit") && error.contains("oversized.png"),
        &format!("must reject oversized dimensions before decoding compressed data: {error}"),
    )
}

/// Shared frames validate RGB count, positive shape and finite samples.
///
/// # Errors
///
/// Returns an error if any boundary is incorrectly validated or alters sample bits.
#[test]
fn raster_constructor_validates_shape_and_samples() -> TestResult {
    let image = RgbImage::new(2, 1, vec![0.0_f32, 0.25, 0.5, 0.75, 1.0, 1.25])?;
    require((image.width(), image.height()) == (2, 1), "shape survives")?;
    require(
        image
            .samples()
            .iter()
            .map(|value| value.to_bits())
            .eq([0.0_f32, 0.25, 0.5, 0.75, 1.0, 1.25].map(f32::to_bits)),
        "sample bits survive",
    )?;
    require(
        RgbImage::new(0, 1, Vec::new()).is_err(),
        "zero width must fail",
    )?;
    require(
        RgbImage::new(1, 0, Vec::new()).is_err(),
        "zero height must fail",
    )?;
    require(
        RgbImage::new(usize::MAX, 2, Vec::new()).is_err(),
        "overflowing area must fail",
    )?;
    require(
        RgbImage::new(1, 1, vec![0.0_f32]).is_err(),
        "sample undercount must fail",
    )?;
    require(
        RgbImage::new(1, 1, vec![0.0_f32, 1.0, f32::NAN]).is_err(),
        "nonfinite samples must fail",
    )
}

/// Check normalized integer codes in RGB order without lossy reconversion.
///
/// # Errors
///
/// Returns an error for an altered sample count or normalized sample value.
fn require_codes(
    frame: &RgbImage,
    expected: impl IntoIterator<Item = u16>,
    maximum_code: f32,
) -> TestResult {
    let mut actual = frame.samples().iter().copied();
    for code in expected {
        let sample = actual
            .next()
            .ok_or_else(|| io::Error::other("decoded sample count is too short"))?;
        require(
            sample.mul_add(maximum_code, -f32::from(code)).abs() <= 0.01,
            &format!("working sample {sample} changed encoded value {code}"),
        )?;
    }
    require(actual.next().is_none(), "decoded sample count is too long")
}

/// Keep every RGB16 code and treat alpha as an unassociated, discarded channel.
///
/// # Errors
///
/// Returns an encoding, decoding or precision-preservation error.
#[test]
fn png_codec_keeps_low_bits_and_drops_alpha() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("焦点‐sixteen.png");
    let native = [1_u16, 257, 32769, 43, 65535, 555, 1234, 0];
    let source = ImageBuffer::from_raw(2, 1, native.to_vec())
        .ok_or_else(|| io::Error::other("bad RGBA16 fixture shape"))?;
    DynamicImage::ImageRgba16(source).save_with_format(&path, ImageFormat::Png)?;
    let frame = decode(&path)?;
    require(
        frame.sample_encoding() == SampleEncoding::U16,
        "source precision changed",
    )?;
    require(frame.icc_profile().is_none(), "source has no profile")?;
    require_codes(&frame, [1, 257, 32769, 65535, 555, 1234], 65535.0)
}

/// Apply each native EXIF geometry exactly once on an asymmetric six-pixel frame.
///
/// # Errors
///
/// Returns an encoding, decoding or orientation mismatch error.
#[test]
fn png_orientation_follows_each_exif_value() -> TestResult {
    let directory = tempfile::tempdir()?;
    let rasters: [(u8, usize, [u16; 6]); 8] = [
        (1, 2, [3, 17, 42, 64, 90, 251]),
        (2, 2, [17, 3, 64, 42, 251, 90]),
        (3, 2, [251, 90, 64, 42, 17, 3]),
        (4, 2, [90, 251, 42, 64, 3, 17]),
        (5, 3, [3, 42, 90, 17, 64, 251]),
        (6, 3, [90, 42, 3, 251, 64, 17]),
        (7, 3, [251, 64, 17, 90, 42, 3]),
        (8, 3, [17, 64, 251, 3, 42, 90]),
    ];
    for (orientation, width, raster) in rasters {
        let path = directory
            .path()
            .join(format!("orientation-{orientation}.png"));
        let mut encoder = PngEncoder::new(File::create(&path)?);
        encoder.set_exif_metadata(vec![
            0x49,
            0x49,
            42,
            0,
            8,
            0,
            0,
            0,
            1,
            0,
            0x12,
            0x01,
            3,
            0,
            1,
            0,
            0,
            0,
            orientation,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ])?;
        encoder.write_image(&[3, 17, 42, 64, 90, 251], 2, 3, ExtendedColorType::L8)?;
        let frame = decode(&path)?;
        require(
            (frame.width(), frame.height()) == (width, if width == 2 { 3 } else { 2 }),
            "wrong oriented dimensions",
        )?;
        require_codes(&frame, raster.into_iter().flat_map(|code| [code; 3]), 255.0)?;
    }
    Ok(())
}

/// Decode an ordinary BMP grayscale raster and reject a directory passed as one.
///
/// # Errors
///
/// Returns an encoding, decoding or sample-count error.
#[test]
fn bmp_and_directory_input_paths() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("frame.bmp");
    let native = GrayImage::from_raw(2, 1, vec![7, 237])
        .ok_or_else(|| io::Error::other("bad BMP fixture dimensions"))?;
    native.save_with_format(&path, ImageFormat::Bmp)?;
    let frame = decode(&path)?;
    require(
        frame.sample_encoding() == SampleEncoding::U8,
        "BMP changed precision",
    )?;
    require_codes(&frame, [7, 7, 7, 237, 237, 237], 255.0)?;
    require(
        decode(directory.path()).is_err(),
        "directory was treated as an image",
    )
}

/// Verify TIFF metadata chooses the correct normalized float code scale.
///
/// # Errors
///
/// Returns an encoding, decoding or scaled-sample mismatch error.
#[test]
fn float_tiff_keeps_levels_on_both_input_scales() -> TestResult {
    let directory = tempfile::tempdir()?;
    for (name, samples, expected) in [
        ("normalized", [0.0_f32, 0.25, 1.0], [0.0_f32, 0.25, 1.0]),
        ("code-values", [0.0, 49.0, 255.0], [0.0, 49.0 / 255.0, 1.0]),
        ("hdr", [-0.5, 0.25, 65504.0], [-0.5, 0.25, 65504.0]),
    ] {
        let path = directory.path().join(format!("{name}.tiff"));
        let source = ImageBuffer::from_raw(1, 1, samples.to_vec())
            .ok_or_else(|| io::Error::other("bad float TIFF fixture dimensions"))?;
        DynamicImage::ImageRgb32F(source).save_with_format(&path, ImageFormat::Tiff)?;
        let frame = decode(&path)?;
        require(
            frame.sample_encoding() == SampleEncoding::Float,
            "TIFF lost float precision",
        )?;
        require(
            frame
                .into_samples()
                .into_iter()
                .map(f32::to_bits)
                .eq(expected.map(f32::to_bits)),
            "unexpected TIFF float scale",
        )?;
    }
    Ok(())
}

/// Preserve finite HDR EXR floats and reject a nonfinite raster during decode.
///
/// # Errors
///
/// Returns an encoding, decode, precision or nonfinite acceptance error.
#[test]
fn exr_decode_keeps_float_bits_and_rejects_nonfinite_values() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("exposure.exr");
    let samples = [-0.25_f32, f32::from_bits(0x3f00_0001), 65504.0];
    let source = ImageBuffer::from_raw(1, 1, samples.to_vec())
        .ok_or_else(|| io::Error::other("bad float EXR fixture dimensions"))?;
    DynamicImage::ImageRgb32F(source).save_with_format(&path, ImageFormat::OpenExr)?;
    let frame = decode(&path)?;
    require(
        frame
            .into_samples()
            .into_iter()
            .map(f32::to_bits)
            .eq(samples.map(f32::to_bits)),
        "EXR changed float sample bits",
    )?;
    let source = ImageBuffer::from_raw(1, 1, vec![0.0_f32, f32::NAN, 0.0])
        .ok_or_else(|| io::Error::other("bad malformed EXR fixture dimensions"))?;
    DynamicImage::ImageRgb32F(source).save_with_format(&path, ImageFormat::OpenExr)?;
    require(
        decode(&path).is_err(),
        "nonfinite EXR samples were accepted",
    )
}

/// Write a version 1 `NumPy` dictionary and test-owned binary payload.
///
/// # Errors
///
/// Returns header-size conversion, write or path errors.
fn write_array(path: &Path, dtype: &str, shape: &str, payload: &[u8]) -> TestResult {
    let dictionary = format!("{{'descr': '{dtype}', 'fortran_order': False, 'shape': {shape}, }}");
    let padding = 64_usize.saturating_sub(dictionary.len().saturating_add(11) % 64);
    let dictionary = format!("{dictionary}{}\n", " ".repeat(padding));
    let mut output = File::create(path)?;
    output.write_all(b"\x93NUMPY\x01\x00")?;
    output.write_all(&u16::try_from(dictionary.len())?.to_le_bytes())?;
    output.write_all(dictionary.as_bytes())?;
    output.write_all(payload)?;
    Ok(())
}

/// Decode real grayscale/RGB array files with their actual byte order.
///
/// # Errors
///
/// Returns a test-file write, decode, or sample-value mismatch error.
#[test]
fn npy_array_decodes_endian_codes_and_requires_complete_payload() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("values.npy");
    write_array(&path, ">u2", "(1, 2)", &[0, 1, 1, 2])?;
    let frame = decode(&path)?;
    require_codes(&frame, [1, 1, 1, 258, 258, 258], 65535.0)?;
    write_array(&path, "|u1", "(1, 2, 3)", &[1, 7, 255, 42, 53, 48])?;
    let frame = decode(&path)?;
    require_codes(&frame, [1, 7, 255, 42, 53, 48], 255.0)?;
    for payload in [&[1, 7, 255, 42, 53, 48, 0][..], &[1, 7][..]] {
        write_array(&path, "|u1", "(1, 2, 3)", payload)?;
        require(
            decode(&path).is_err(),
            "invalid array byte count was accepted",
        )?;
    }
    Ok(())
}

/// Exercise complete Bayer development with a generated DNG without a thumbnail.
///
/// # Errors
///
/// Returns setup, source decode, dimensions or real RGB16 precision errors.
#[test]
fn bayer_dng_develops_true_sixteen_bit_rgb() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("bayer.dng");
    // Frozen generator bayer_dng frame_00, source 99051b2e…66817.
    let encoded = include_bytes!("data/bayer-pattern.dng");
    fs::write(&path, encoded)?;
    let frame = decode(&path)?;
    require(
        (frame.width(), frame.height(), frame.sample_encoding()) == (160, 112, SampleEncoding::U16),
        "RAW was not developed at full geometry and deep precision",
    )?;
    let (low, high) = frame
        .samples()
        .iter()
        .copied()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| {
            (low.min(value), high.max(value))
        });
    require(high - low > 0.15, "developed RAW lost tonal range")?;
    let native_low_bits = frame
        .samples()
        .iter()
        .filter(|&&value| {
            let code = (value * 65_535.0_f32).round();
            code % 257.0 > 0.0
        })
        .count();
    require(
        native_low_bits > 10_000,
        "RAW was quantized to eight-bit or lost native sample precision",
    )?;
    require(
        fs::read(&path)? == encoded.as_slice(),
        "source was modified",
    )
}

/// Check original RGB profile bytes without changing sample encoding.
///
/// # Errors
///
/// Returns native PNG setup, decode, profile or sample-preservation errors.
#[test]
fn png_input_retains_embedded_icc_without_altering_rgb() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("profiled.png");
    let icc = include_bytes!("data/srgb.png.icc");
    let mut encoder = PngEncoder::new(File::create(&path)?);
    encoder.set_icc_profile(icc.to_vec())?;
    encoder.write_image(&[11, 49, 137], 1, 1, ExtendedColorType::Rgb8)?;
    let frame = decode(&path)?;
    require(
        frame.icc_profile() == Some(icc.as_slice()),
        "embedded color profile changed",
    )?;
    require_codes(&frame, [11, 49, 137], 255.0)
}

/// Return the first classic little-endian TIFF directory's tag fields.
///
/// # Errors
///
/// Returns malformed or truncated fixture directory errors.
fn tiff_fields(bytes: &mut [u8]) -> Result<&mut [[u8; 12]], Box<dyn Error>> {
    let offset = bytes
        .get(4..8)
        .ok_or_else(|| io::Error::other("no TIFF header"))?;
    let offset = usize::try_from(u32::from_le_bytes(offset.try_into()?))?;
    let directory = bytes
        .get_mut(offset..)
        .ok_or_else(|| io::Error::other("no TIFF directory"))?;
    let ([count_low, count_high], fields) = directory
        .split_first_chunk_mut::<2>()
        .ok_or_else(|| io::Error::other("truncated TIFF directory"))?;
    let count = usize::from(u16::from_le_bytes([*count_low, *count_high]));
    fields
        .as_chunks_mut::<12>()
        .0
        .get_mut(..count)
        .ok_or_else(|| io::Error::other("truncated TIFF directory fields").into())
}

/// Identify a classic TIFF field by its unsigned tag number.
const fn tiff_tag(field: &[u8; 12]) -> u16 {
    let [identifier_low, identifier_high, ..] = *field;
    u16::from_le_bytes([identifier_low, identifier_high])
}

/// Update one classic TIFF scalar tag value, leaving the raster payload untouched.
///
/// # Errors
///
/// Returns invalid fixture header/directory or missing-tag errors.
fn set_dng_scalar(bytes: &mut [u8], tag: u16, value: u32) -> TestResult {
    let field = tiff_fields(bytes)?
        .iter_mut()
        .find(|field| tiff_tag(field) == tag)
        .ok_or_else(|| io::Error::other(format!("TIFF fixture omitted tag {tag}")))?;
    let [.., output_a, output_b, output_c, output_d] = field;
    [*output_a, *output_b, *output_c, *output_d] = value.to_le_bytes();
    Ok(())
}

/// Reject impossible camera sensor dimensions and extra channels before decompression.
///
/// # Errors
///
/// Returns fixture modification, false decode success or missing diagnostic errors.
#[test]
fn malformed_dng_sensor_metadata_fails_preflight() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("oversized-sensor.dng");
    for (tag, value, message) in [
        (256, 65_536, "image limit"),
        (277, 2, "one or three channels"),
    ] {
        let mut bytes = include_bytes!("data/bayer-pattern.dng").to_vec();
        set_dng_scalar(&mut bytes, tag, value)?;
        fs::write(&path, bytes)?;
        let error = decode(&path)
            .err()
            .ok_or_else(|| io::Error::other("unsafe RAW shape accepted"))?;
        require(
            error.contains(message),
            &format!("expected {message}, got {error}"),
        )?;
    }
    Ok(())
}

/// Reject an over-budget encoded RAW file by its actual byte count.
///
/// # Errors
///
/// Returns file setup or an unexpected decode result.
#[test]
fn encoded_raw_size_is_limited() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("encoded-too-large.dng");
    File::create(&path)?.set_len(268_435_457)?;
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("oversized compressed RAW was accepted"))?;
    require(
        error.contains("compressed RAW source exceeds the byte limit"),
        &format!("expected encoded size diagnostic, got {error}"),
    )
}

/// Reject missing sources and PNG files with a valid header but truncated raster.
///
/// # Errors
/// Returns a setup error or an incorrect input diagnosis.
#[test]
fn missing_and_truncated_rasters_report_the_offending_path() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("recoverable.png");
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("a missing raster was decoded"))?;
    require(
        error.contains("recoverable.png"),
        &format!("missing path diagnostic: {error}"),
    )?;
    let source = GrayImage::from_raw(2, 2, vec![7, 19, 47, 193])
        .ok_or_else(|| io::Error::other("bad PNG fixture dimensions"))?;
    source.save_with_format(&path, ImageFormat::Png)?;
    let mut bytes = fs::read(&path)?;
    let data_start = bytes
        .windows(4)
        .position(|marker| marker == b"IDAT")
        .ok_or_else(|| io::Error::other("PNG fixture has no IDAT"))?;
    bytes.truncate(data_start.saturating_add(5));
    fs::write(&path, &bytes)?;
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("a truncated PNG raster was decoded"))?;
    require(
        error.contains("recoverable.png"),
        &format!("corrupt path diagnostic: {error}"),
    )?;
    require(
        fs::read(&path)? == bytes,
        "decoder modified truncated source",
    )
}

/// Treat unknown positive HDR TIFF levels above 255 as working float data.
///
/// # Errors
/// Returns changed precision, an invalid sample, or fixture construction failure.
#[test]
fn positive_hdr_tiff_does_not_use_ordinary_integer_code_scale() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("positive-hdr.tif");
    let original = [0.25_f32, 256.0, 65504.0];
    let source = ImageBuffer::from_raw(1, 1, original.to_vec())
        .ok_or_else(|| io::Error::other("bad positive HDR fixture dimensions"))?;
    DynamicImage::ImageRgb32F(source).save_with_format(&path, ImageFormat::Tiff)?;
    let frame = decode(&path)?;
    require(
        frame
            .into_samples()
            .into_iter()
            .map(f32::to_bits)
            .eq(original.map(f32::to_bits)),
        "positive HDR TIFF was rescaled as an eight-bit raster",
    )
}

/// Huge raster area is rejected even when the two axes are within limits.
///
/// # Errors
/// Returns an invalid constructor result or a misleading geometry diagnostic.
#[test]
fn shape_caps_precede_sample_validation() -> TestResult {
    for (width, height) in [(32769, 2), (2, 32769), (8192, 8192)] {
        let error = RgbImage::new(width, height, Vec::new())
            .err()
            .ok_or_else(|| io::Error::other("oversized working raster shape was accepted"))?;
        require(
            error.contains("image limit"),
            &format!("expected shape cap before sample length checking: {error}"),
        )?;
    }
    Ok(())
}

/// Rotate a developed Bayer DNG according to its own raw metadata.
///
/// # Errors
/// Returns missing fixture metadata or a mismatch in decoded pixels or shape.
#[test]
fn developed_bayer_dng_obeys_camera_orientation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("original.dng");
    let mut bytes = include_bytes!("data/bayer-pattern.dng").to_vec();
    fs::write(&path, &bytes)?;
    let base = decode(&path)?;

    let entries = tiff_fields(&mut bytes)?;
    let resolution = entries
        .iter_mut()
        .find(|entry| tiff_tag(entry) == 282)
        .ok_or_else(|| {
            io::Error::other("DNG fixture has no replaceable optional resolution tag")
        })?;
    // The optional XResolution is replaced by Orientation = 6 (90° clockwise).
    *resolution = [0x12, 0x01, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0];
    entries.sort_unstable_by_key(tiff_tag);
    let rotated_path = directory.path().join("rotated.dng");
    fs::write(&rotated_path, &bytes)?;
    let actual = decode(&rotated_path)?;
    require(
        (actual.width(), actual.height()) == (base.height(), base.width()),
        "camera orientation did not transpose output dimensions",
    )?;
    let rows = base.samples().as_chunks::<3>().0.chunks_exact(base.width());
    let expected = (0..base.width()).flat_map(|x| {
        rows.clone()
            .rev()
            .filter_map(move |row| row.get(x))
            .copied()
            .flatten()
    });
    require(
        actual
            .samples()
            .iter()
            .copied()
            .map(f32::to_bits)
            .eq(expected.map(f32::to_bits)),
        "camera orientation changed pixels or used the wrong rotation",
    )
}

/// Reject an existing TIFF whose orientation tag points outside the file.
///
/// # Errors
/// Returns a fixture or unexpected decode result.
#[test]
fn invalid_tiff_orientation_storage_reports_decode_failure() -> TestResult {
    use tiff::encoder::{TiffEncoder, colortype};
    use tiff::tags::Tag;

    let directory = tempfile::tempdir()?;
    let path = directory.path().join("invalid-orientation.tif");
    let mut encoder = TiffEncoder::new(File::create(&path)?)?;
    let mut image = encoder.new_image::<colortype::RGB8>(1, 1)?;
    image
        .encoder()
        .write_tag(Tag::Orientation, &[1_u16, 1, 1][..])?;
    image.write_data(&[7, 11, 23])?;
    let mut bytes = fs::read(&path)?;
    set_dng_scalar(&mut bytes, 274, u32::MAX)?;
    fs::write(&path, &bytes)?;
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("invalid TIFF orientation storage was accepted"))?;
    require(
        error.contains("invalid-orientation.tif"),
        &format!("missing TIFF path: {error}"),
    )?;
    require(
        fs::read(&path)? == bytes,
        "metadata failure changed the source file",
    )
}

/// Reject Bayer DNG containers when dimensions survive but the strip is incomplete.
///
/// # Errors
/// Returns a fixture error or incorrect source diagnostic.
#[test]
fn corrupt_bayer_payload_is_not_developed() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("incomplete.dng");
    let mut bytes = include_bytes!("data/bayer-pattern.dng").to_vec();
    set_dng_scalar(&mut bytes, 273, u32::MAX)?;
    fs::write(&path, &bytes)?;
    let error = decode(&path)
        .err()
        .ok_or_else(|| io::Error::other("incomplete RAW strip produced a raster"))?;
    require(
        error.contains("incomplete.dng"),
        &format!("missing RAW source path: {error}"),
    )?;
    require(
        fs::read(&path)? == bytes,
        "RAW failure changed the source file",
    )
}

/// An incorrectly typed optional ICC tag must not change a valid TIFF raster.
///
/// # Errors
/// Returns a fixture, decode, or metadata/raster integrity failure.
#[test]
fn malformed_optional_tiff_profile_preserves_valid_samples() -> TestResult {
    use tiff::encoder::{TiffEncoder, colortype};
    use tiff::tags::Tag;

    let directory = tempfile::tempdir()?;
    let path = directory.path().join("wrong-profile-type.tiff");
    let mut encoder = TiffEncoder::new(File::create(&path)?)?;
    let mut encoded = encoder.new_image::<colortype::RGB16>(1, 1)?;
    encoded
        .encoder()
        .write_tag(Tag::IccProfile, &[7_u16, 13, 251][..])?;
    encoded.write_data(&[1, 17, 4097])?;
    let frame = decode(&path)?;
    require(
        frame.icc_profile().is_none() && frame.sample_encoding() == SampleEncoding::U16,
        "incorrectly typed ICC metadata changed source precision or was returned as a byte profile",
    )?;
    require_codes(&frame, [1, 17, 4097], 65_535.0)
}
