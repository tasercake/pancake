//! Direct dependency capability tests, separate from the application graph.

use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use std::sync::LazyLock;

use image::{ColorType, ImageBuffer, ImageFormat, Rgb};
use rawler::formats::tiff::reader::TiffReader;
use tiff::decoder::{Decoder, DecodingResult};
use tiff::encoder::{Compression, DeflateLevel, TiffEncoder, colortype};

/// Probe result with recoverable dependency and I/O errors.
type ProbeResult = Result<(), Box<dyn Error>>;

/// One process-wide bounded pool owns every RAW decode/development operation.
static RAW_POOL: LazyLock<Result<rayon::ThreadPool, String>> = LazyLock::new(|| {
    rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|index| format!("pancake-raw-{index}"))
        .build()
        .map_err(|error| error.to_string())
});

/// Obtain the shared RAW pool without building another or mutating Rayon globals.
fn raw_pool() -> Result<&'static rayon::ThreadPool, io::Error> {
    RAW_POOL
        .as_ref()
        .map_err(|message| io::Error::other(message.clone()))
}

/// RGB16 values include low bits that an eight-bit intermediate cannot preserve.
const RGB16: [u16; 12] = [
    0, 1, 255, 256, 257, 258, 1023, 4095, 4096, 16383, 32769, 65535,
];

/// Float values include negatives and highlights above display white.
const FLOAT: [f32; 12] = [
    -0.25,
    0.0,
    0.125,
    1.0,
    2.5,
    65504.0,
    f32::from_bits(0x3f00_0001),
    -1.0,
    1.25,
    32.0,
    1.5,
    0.25,
];

/// Decode a complete real CR3 sensor through Rawler's safe public API.
fn decode_sensor(path: &Path, output: &Path) -> ProbeResult {
    let bytes = fs::read(path)?;
    let source = rawler::rawsource::RawSource::new_from_shared_vec(std::sync::Arc::new(bytes));
    let raw = raw_pool()?
        .install(|| rawler::decode(&source, &rawler::decoders::RawDecodeParams::default()))?;
    let rawler::RawImageData::Integer(samples) = &raw.data else {
        return Err(io::Error::other("expected integer sensor data").into());
    };
    let expected = raw
        .width
        .checked_mul(raw.height)
        .ok_or_else(|| io::Error::other("sensor dimensions overflow"))?;
    if raw.cpp != 1 || samples.len() != expected || raw.bps != 16 {
        return Err(io::Error::other(format!("sensor shape or native precision changed: width={}, height={}, cpp={}, samples={}, bits={}", raw.width, raw.height, raw.cpp, samples.len(), raw.bps)).into());
    }
    if !samples.iter().any(|sample| sample & 255 != 0) {
        return Err(io::Error::other("sensor low bits were discarded").into());
    }
    let bytes: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    fs::write(output.join("rawler-sensor.u16"), bytes)?;
    let mut metadata = File::create(output.join("rawler-sensor.txt"))?;
    writeln!(
        metadata,
        "camera={:?} {:?}\nwidth={}\nheight={}\nbits={}\nactive={:?}\ncrop={:?}\norientation={:?}",
        raw.make,
        raw.model,
        raw.width,
        raw.height,
        raw.bps,
        raw.active_area,
        raw.crop_area,
        raw.orientation
    )?;
    let container = rawler::formats::bmff::Bmff::new_buf(source.buf())?;
    if let Some(description) = &container.filebox.moov.cr3desc {
        writeln!(
            metadata,
            "SensorInfo={:?}",
            description
                .cmt3
                .tiff
                .root_ifd()
                .get_entry(rawler::decoders::cr3::Cr3MakernoteTag::SensorInfo)
        )?;
    }
    for track in &container.filebox.moov.traks {
        if let Some(craw) = &track.mdia.minf.stbl.stsd.craw {
            writeln!(metadata, "CRAW CMP1={:?}\nCDI1={:?}", craw.cmp1, craw.cdi1)?;
        }
    }
    Ok(())
}

/// Preserve all 16-bit RGB samples through PNG and ZIP-compressed TIFF.
fn rgb16_roundtrips(output: &Path) -> ProbeResult {
    let png_path = output.join("rgb16.png");
    let image: ImageBuffer<Rgb<u16>, Vec<u16>> = ImageBuffer::from_raw(2, 2, RGB16.to_vec())
        .ok_or_else(|| io::Error::other("invalid RGB fixture dimensions"))?;
    image.save_with_format(&png_path, ImageFormat::Png)?;
    let png_image = image::open(png_path)?;
    if png_image.color() != ColorType::Rgb16
        || png_image.into_rgb16().as_raw().as_slice() != RGB16.as_slice()
    {
        return Err(io::Error::other("PNG16 round trip changed samples").into());
    }
    let tiff_path = output.join("rgb16.tiff");
    let mut encoder = TiffEncoder::new(File::create(&tiff_path)?)?
        .with_compression(Compression::Deflate(DeflateLevel::default()));
    encoder.write_image::<colortype::RGB16>(2, 2, &RGB16)?;
    let mut decoder = Decoder::new(File::open(tiff_path)?)?;
    if !matches!(decoder.read_image()?, DecodingResult::U16(values) if values.as_slice() == RGB16.as_slice())
    {
        return Err(io::Error::other("TIFF16 round trip changed samples").into());
    }
    Ok(())
}

/// Compare floating-point samples by their complete stored representation.
fn float_samples_match(samples: &[f32]) -> bool {
    samples.len() == FLOAT.len()
        && samples
            .iter()
            .zip(FLOAT)
            .all(|(actual, expected)| actual.to_bits() == expected.to_bits())
}

/// Preserve negative and HDR RGB32F samples through TIFF and `OpenEXR`.
fn float_roundtrips(output: &Path) -> ProbeResult {
    let tiff_path = output.join("float32.tiff");
    let mut encoder = TiffEncoder::new(File::create(&tiff_path)?)?;
    encoder.write_image::<colortype::RGB32Float>(2, 2, &FLOAT)?;
    let mut decoder = Decoder::new(File::open(tiff_path)?)?;
    if !matches!(decoder.read_image()?, DecodingResult::F32(values) if float_samples_match(&values))
    {
        return Err(io::Error::other("float TIFF round trip changed samples").into());
    }
    let exr_path = output.join("float32.exr");
    exr::prelude::write_rgb_file(&exr_path, 2, 2, |x, y| match (x, y) {
        (0, 0) => (-0.25_f32, 0.0_f32, 0.125_f32),
        (1, 0) => (1.0_f32, 2.5_f32, 65504.0_f32),
        (0, 1) => (f32::from_bits(0x3f00_0001), -1.0_f32, 1.25_f32),
        _ => (32.0_f32, 1.5_f32, 0.25_f32),
    })?;
    let exr_image = image::open(exr_path)?;
    if exr_image.color() != ColorType::Rgb32F
        || !float_samples_match(exr_image.into_rgb32f().as_raw())
    {
        return Err(io::Error::other("float EXR round trip changed samples").into());
    }
    Ok(())
}

/// Validate profiles, pass identical encodings through, or run an actual transform.
fn convert_rgb16(
    source_icc: &[u8],
    destination_icc: &[u8],
    pixels: &[[u16; 3]],
) -> Result<Vec<[u16; 3]>, Box<dyn Error>> {
    let source_profile = lcms2::Profile::new_icc(source_icc)?;
    let destination_profile = lcms2::Profile::new_icc(destination_icc)?;
    if source_icc == destination_icc {
        return Ok(pixels.to_vec());
    }
    let transform = lcms2::Transform::<[u16; 3], [u16; 3]>::new(
        &source_profile,
        lcms2::PixelFormat::RGB_16,
        &destination_profile,
        lcms2::PixelFormat::RGB_16,
        lcms2::Intent::RelativeColorimetric,
    )?;
    let mut converted = vec![[0_u16; 3]; pixels.len()];
    transform.transform_pixels(pixels, &mut converted);
    Ok(converted)
}

/// Exercise explicit profile passthrough and a separate real transfer conversion.
fn icc_identity(output: &Path) -> ProbeResult {
    let profile = lcms2::Profile::new_srgb();
    let bytes = profile.icc()?;
    fs::write(output.join("srgb.icc"), &bytes)?;
    let source: Vec<[u16; 3]> = (0_u16..=u16::MAX)
        .map(|value| [value, value.reverse_bits(), value.wrapping_add(1)])
        .collect();
    let destination = convert_rgb16(&bytes, &bytes, &source)?;
    if destination != source {
        return Err(io::Error::other("byte-identical ICC passthrough changed u16 samples").into());
    }
    fs::write(
        output.join("icc-noop.u16"),
        destination
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>(),
    )?;
    let white = lcms2::CIExyY {
        x: 0.3127_f64,
        y: 0.3290_f64,
        Y: 1.0_f64,
    };
    let primaries = lcms2::CIExyYTRIPLE {
        Red: lcms2::CIExyY {
            x: 0.64_f64,
            y: 0.33_f64,
            Y: 1.0_f64,
        },
        Green: lcms2::CIExyY {
            x: 0.30_f64,
            y: 0.60_f64,
            Y: 1.0_f64,
        },
        Blue: lcms2::CIExyY {
            x: 0.15_f64,
            y: 0.06_f64,
            Y: 1.0_f64,
        },
    };
    let gamma = lcms2::ToneCurve::new(1.0_f64);
    let linear = lcms2::Profile::new_rgb(&white, &primaries, &[&gamma, &gamma, &gamma])?.icc()?;
    fs::write(output.join("linear-srgb.icc"), &linear)?;
    let source = [
        [0_u16, 1_u16, 255_u16],
        [256_u16, 257_u16, 258_u16],
        [1023_u16, 4095_u16, 4096_u16],
        [16383_u16, 32769_u16, 65535_u16],
    ];
    let converted = convert_rgb16(&bytes, &linear, &source)?;
    if converted == source {
        return Err(io::Error::other("distinct transfer profiles did not transform").into());
    }
    fs::write(
        output.join("icc-linear.u16"),
        converted
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<u8>>(),
    )?;
    if lcms2::Profile::new_icc(b"not an ICC profile").is_ok()
        || image::load_from_memory(b"not an image").is_ok()
    {
        return Err(io::Error::other("malformed profile/image was accepted").into());
    }
    Ok(())
}

/// Run the dependency capabilities before the independent Python oracle.
#[test]
fn safe_raw_and_full_precision_codecs() -> ProbeResult {
    let source = env::var_os("PANCAKE_RAW_SAMPLE")
        .ok_or_else(|| io::Error::other("PANCAKE_RAW_SAMPLE must name the real pinned CR3"))?;
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/qualification");
    fs::create_dir_all(&output)?;
    decode_sensor(Path::new(&source), &output)?;
    rgb16_roundtrips(&output)?;
    float_roundtrips(&output)?;
    icc_identity(&output)?;
    Ok(())
}

/// Decode an actual Bayer DNG container with an independently generated mosaic.
#[test]
fn generated_bayer_dng_retains_all_sensor_samples() -> ProbeResult {
    let output = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/qualification");
    let bytes = fs::read(output.join("bayer.dng"))?;
    let source = rawler::rawsource::RawSource::new_from_shared_vec(std::sync::Arc::new(bytes));
    let raw = raw_pool()?
        .install(|| rawler::decode(&source, &rawler::decoders::RawDecodeParams::default()))?;
    let rawler::RawImageData::Integer(samples) = &raw.data else {
        return Err(io::Error::other("Bayer DNG returned non-integer sensor data").into());
    };
    let bytes: Vec<u8> = samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect();
    if raw.width != 32
        || raw.height != 24
        || raw.cpp != 1
        || bytes != fs::read(output.join("bayer-expected.u16"))?
    {
        return Err(io::Error::other("Bayer DNG changed shape or sensor samples").into());
    }
    fs::write(output.join("bayer-rust.u16"), bytes)?;
    Ok(())
}
