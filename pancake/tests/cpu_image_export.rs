//! Public export precision, atomic replacement, and cancellation contracts.

use std::error::Error;
use std::fs;
use std::path::Path;

use image::ColorType;
use pancake::config::BitDepth;
use pancake::image_io::{ExportFormat, RgbImage, export, prepare_export};

/// Source codes exercise both bytes of each channel and all RGB positions.
const RGB16: [u16; 12] = [0, 257, 256, 32768, 580, 62211, 65025, 65535, 130, 35533, 17, 1];

/// Build a complete odd-precision test image through the real RGB API.
fn rgb16_frame() -> Result<RgbImage, String> {
    let samples = RGB16
        .map(|value| f32::from(value) / f32::from(u16::MAX))
        .to_vec();
    RgbImage::new(2, 2, samples)
}

/// Lossless PNG and TIFF preserve genuine u16 codes and share quantization.
#[test]
fn lossless_formats_keep_full_precision() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let frame = rgb16_frame()?;
    for suffix in ["PNG", "tif", "tiff"] {
        let path = dir.path().join(format!("résultat.{suffix}"));
        export(&path, &frame, BitDepth::Sixteen, 90, &[])?;
        let decoded = image::open(&path)?;
        assert_eq!(decoded.color(), ColorType::Rgb16, "genuine {suffix} RGB16");
        assert_eq!(
            decoded.into_rgb16().into_raw(),
            RGB16,
            "{suffix} changed RGB order or sample values"
        );
    }
    for suffix in ["png", "tif"] {
        let path = dir.path().join(format!("eight.{suffix}"));
        export(&path, &frame, BitDepth::Eight, 90, &[])?;
        assert_eq!(
            image::open(&path)?.into_rgb8().into_raw(),
            [0, 1, 1, 128, 2, 242, 253, 255, 1, 138, 0, 0],
            "{suffix} RGB8 must round to nearest code"
        );
    }
    Ok(())
}

/// JPEG16 requests retain a successful export with an observable depth warning.
#[test]
fn jpeg_declares_and_writes_eight_bit_samples() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let frame = rgb16_frame()?;
    for suffix in ["jpg", "jpeg"] {
        let path = dir.path().join(format!("encoded.{suffix}"));
        let prepared = prepare_export(&path, &frame, BitDepth::Sixteen, 95, &[])?;
        assert_eq!(ExportFormat::from_path(&path)?, ExportFormat::Jpeg, "JPEG alias");
        assert!(
            prepared.warning().is_some_and(|message| message.contains("8-bit")),
            "requested JPEG16 needs a truthful precision diagnostic"
        );
        prepared.publish()?;
        let decoded = image::open(path)?;
        assert_eq!(decoded.color(), ColorType::Rgb8, "decoded JPEG depth");
        assert_eq!((decoded.width(), decoded.height()), (2, 2), "JPEG shape");
    }
    Ok(())
}

/// EXR round-trips signed and out-of-gamut float values without quantization.
#[test]
fn exr_keeps_float_samples() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("hdr.exr");
    let samples = vec![-0.125_f32, 0.123_456_79, 7.25, 0.0, 0.75, 1.0];
    let frame = RgbImage::new(2, 1, samples.clone())?;
    let prepared = prepare_export(&path, &frame, BitDepth::Eight, 95, &[])?;
    assert_eq!(prepared.warning(), None, "EXR retains higher float precision");
    prepared.publish()?;
    let decoded = image::open(path)?;
    assert_eq!(decoded.color(), ColorType::Rgb32F, "EXR channel representation");
    let actual: Vec<_> = decoded.into_rgb32f().into_raw().into_iter().map(f32::to_bits).collect();
    let expected: Vec<_> = samples.into_iter().map(f32::to_bits).collect();
    assert_eq!(actual, expected, "lossless EXR values");
    Ok(())
}

/// Cancellation discards completed staging without replacing any prior export.
#[test]
fn staging_is_invisible_until_commit_and_cleans_on_drop() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("existing.png");
    let frame = rgb16_frame()?;
    fs::write(&path, b"previous export")?;
    let cancelled = prepare_export(&path, &frame, BitDepth::Sixteen, 95, &[])?;
    assert_eq!(fs::read(&path)?, b"previous export", "staging touched destination");
    drop(cancelled);
    assert_eq!(fs::read(&path)?, b"previous export", "cancel touched destination");
    assert_eq!(fs::read_dir(dir.path())?.count(), 1, "cancel leaked staged data");

    let prepared = prepare_export(&path, &frame, BitDepth::Sixteen, 95, &[])?;
    prepared.publish()?;
    assert_eq!(
        image::open(&path)?.into_rgb16().into_raw(),
        RGB16,
        "publishing must replace the previous file with a complete encoded image"
    );
    assert_eq!(fs::read_dir(dir.path())?.count(), 1, "publish leaked staged data");
    Ok(())
}

/// Unsupported destinations and failed renames preserve existing data.
#[test]
fn failures_retain_destinations_and_remove_staging() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let frame = rgb16_frame()?;
    let invalid = dir.path().join("previous.unknown");
    fs::write(&invalid, b"previous")?;
    assert!(
        export(&invalid, &frame, BitDepth::Eight, 90, &[]).is_err(),
        "unknown formats must fail"
    );
    assert_eq!(fs::read(&invalid)?, b"previous", "unknown format modified old file");
    let missing = dir.path().join("missing").join("image.png");
    assert!(
        export(&missing, &frame, BitDepth::Eight, 90, &[]).is_err(),
        "missing parent must fail with no publication"
    );
    assert!(!missing.exists(), "failed export published output");
    let bad_jpeg = dir.path().join("quality.jpg");
    assert!(
        export(&bad_jpeg, &frame, BitDepth::Eight, 0, &[]).is_err(),
        "invalid JPEG quality must return an error"
    );
    let changed = dir.path().join("replacement.tiff");
    let staged = prepare_export(&changed, &frame, BitDepth::Eight, 90, &[])?;
    fs::create_dir(&changed)?;
    fs::write(changed.join("keep"), b"destination directory")?;
    assert!(staged.publish().is_err(), "rename over directory must fail");
    assert_eq!(fs::read(changed.join("keep"))?, b"destination directory", "publish damaged destination");
    assert_eq!(fs::read_dir(dir.path())?.count(), 2, "failed jobs leaked staging");
    Ok(())
}

/// A successful export destination cannot replace an input file or its hardlink.
#[test]
fn forbids_source_aliases_even_during_commit() -> Result<(), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source.png");
    let alias = dir.path().join("alias.png");
    let late_alias = dir.path().join("late-alias.png");
    fs::write(&source, b"source bytes")?;
    fs::hard_link(&source, &alias)?;
    let sources = [source.clone()];
    let frame = rgb16_frame()?;
    for path in [&source, &alias] {
        assert!(
            export(path, &frame, BitDepth::Sixteen, 90, &sources).is_err(),
            "reject replace of source path or its alias"
        );
        assert_eq!(fs::read(path)?, b"source bytes", "source alias changed");
    }
    let prepared = prepare_export(&late_alias, &frame, BitDepth::Sixteen, 90, &sources)?;
    fs::hard_link(&source, &late_alias)?;
    assert!(prepared.publish().is_err(), "recheck destination at commit");
    assert_eq!(fs::read(&late_alias)?, b"source bytes", "late alias changed");
    assert_eq!(fs::read(source)?, b"source bytes", "source changed");
    assert_eq!(fs::read_dir(dir.path())?.count(), 3, "alias guard leaked staged file");
    assert!(ExportFormat::from_path(Path::new("no-suffix")).is_err(), "require format suffix");
    Ok(())
}
