//! Public export precision, atomic replacement, and cancellation contracts.

use std::error::Error;
use std::path::Path;
use std::{fs, io};

use image::ColorType;
use pancake::config::BitDepth;
use pancake::image_io::{ExportFormat, RgbImage, export, prepare_export};

/// Results carry fixture, filesystem, codec, and contract failures.
type TestResult = Result<(), Box<dyn Error>>;

/// Source codes exercise both bytes of each channel and all RGB positions.
const RGB16: [u16; 12] = [
    0, 257, 256, 32768, 580, 62211, 65025, 65535, 130, 35533, 17, 1,
];

/// Build a complete odd-precision test image through the real RGB API.
///
/// # Errors
/// Returns a frame construction error if the fixture shape or samples are invalid.
fn rgb16_frame() -> Result<RgbImage, String> {
    let samples = RGB16
        .map(|value| f32::from(value) / f32::from(u16::MAX))
        .to_vec();
    RgbImage::new(2, 2, samples)
}

/// Check a claim about the encoded files or observed filesystem state.
///
/// # Errors
/// Returns the supplied diagnostic if the checked condition is false.
fn require(condition: bool, diagnostic: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(diagnostic.to_owned()).into())
    }
}

/// Lossless PNG and TIFF preserve genuine u16 codes and share quantization.
///
/// # Errors
/// Returns a fixture, codec, or sample comparison failure.
#[test]
fn lossless_formats_keep_full_precision() -> TestResult {
    let dir = tempfile::tempdir()?;
    let frame = rgb16_frame()?;
    for suffix in ["PNG", "tif", "tiff"] {
        let path = dir.path().join(format!("résultat.{suffix}"));
        export(&path, &frame, BitDepth::Sixteen, 90, &[])?;
        let decoded = image::open(&path)?;
        require(
            decoded.color() == ColorType::Rgb16,
            &format!("{suffix} must be genuine RGB16"),
        )?;
        require(
            decoded.into_rgb16().into_raw() == RGB16,
            &format!("{suffix} changed RGB order or sample values"),
        )?;
    }
    for suffix in ["png", "tif"] {
        let path = dir.path().join(format!("eight.{suffix}"));
        export(&path, &frame, BitDepth::Eight, 90, &[])?;
        require(
            image::open(&path)?.into_rgb8().into_raw()
                == [0, 1, 1, 128, 2, 242, 253, 255, 1, 138, 0, 0],
            &format!("{suffix} RGB8 must round to nearest code"),
        )?;
    }
    Ok(())
}

/// JPEG16 requests retain a successful export with an observable depth warning.
///
/// # Errors
/// Returns a fixture, codec, or precision reporting failure.
#[test]
fn jpeg_declares_and_writes_eight_bit_samples() -> TestResult {
    let dir = tempfile::tempdir()?;
    let frame = rgb16_frame()?;
    for suffix in ["jpg", "jpeg"] {
        let path = dir.path().join(format!("encoded.{suffix}"));
        let prepared = prepare_export(&path, &frame, BitDepth::Sixteen, 95, &[])?;
        require(
            ExportFormat::from_path(&path)? == ExportFormat::Jpeg,
            "JPEG alias",
        )?;
        require(
            prepared
                .warning()
                .is_some_and(|message| message.contains("8-bit")),
            "requested JPEG16 needs a truthful precision diagnostic",
        )?;
        prepared.publish()?;
        let decoded = image::open(path)?;
        require(decoded.color() == ColorType::Rgb8, "decoded JPEG depth")?;
        require((decoded.width(), decoded.height()) == (2, 2), "JPEG shape")?;
    }
    Ok(())
}

/// EXR round-trips signed and out-of-gamut float values without quantization.
///
/// # Errors
/// Returns a fixture, codec, or floating-point comparison failure.
#[test]
fn exr_keeps_float_samples() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("hdr.exr");
    let samples = vec![-0.125_f32, 0.123_456_79, 7.25, 0.0, 0.75, 1.0];
    let frame = RgbImage::new(2, 1, samples.clone())?;
    let prepared = prepare_export(&path, &frame, BitDepth::Eight, 95, &[])?;
    require(
        prepared.warning().is_none(),
        "EXR retains higher float precision",
    )?;
    prepared.publish()?;
    let decoded = image::open(path)?;
    require(
        decoded.color() == ColorType::Rgb32F,
        "EXR channel representation",
    )?;
    let actual = decoded.into_rgb32f().into_raw();
    require(
        actual
            .into_iter()
            .map(f32::to_bits)
            .eq(samples.into_iter().map(f32::to_bits)),
        "lossless EXR values",
    )
}

/// Cancellation discards completed staging without replacing any prior export.
///
/// # Errors
/// Returns an error if staging, cleanup, or complete replacement fails.
#[test]
fn staging_is_invisible_until_commit_and_cleans_on_drop() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("existing.png");
    let frame = rgb16_frame()?;
    fs::write(&path, b"previous export")?;
    let cancelled = prepare_export(&path, &frame, BitDepth::Sixteen, 95, &[])?;
    require(
        fs::read(&path)? == b"previous export",
        "staging touched destination",
    )?;
    drop(cancelled);
    require(
        fs::read(&path)? == b"previous export",
        "cancel touched destination",
    )?;
    require(
        fs::read_dir(dir.path())?.count() == 1,
        "cancel leaked staged data",
    )?;

    let prepared = prepare_export(&path, &frame, BitDepth::Sixteen, 95, &[])?;
    prepared.publish()?;
    require(
        image::open(&path)?.into_rgb16().into_raw() == RGB16,
        "publishing must replace the previous file with a complete encoded image",
    )?;
    require(
        fs::read_dir(dir.path())?.count() == 1,
        "publish leaked staged data",
    )
}

/// Unsupported destinations and publication failures preserve existing data.
///
/// # Errors
/// Returns an error if an export succeeds unexpectedly or touches old data.
#[test]
fn failures_retain_destinations_and_remove_staging() -> TestResult {
    let dir = tempfile::tempdir()?;
    let frame = rgb16_frame()?;
    let invalid = dir.path().join("previous.unknown");
    fs::write(&invalid, b"previous")?;
    require(
        export(&invalid, &frame, BitDepth::Eight, 90, &[]).is_err(),
        "unknown formats must fail",
    )?;
    require(
        fs::read(&invalid)? == b"previous",
        "unknown format modified old file",
    )?;
    let missing = dir.path().join("missing").join("image.png");
    require(
        export(&missing, &frame, BitDepth::Eight, 90, &[]).is_err(),
        "missing parent must fail with no publication",
    )?;
    require(!missing.exists(), "failed export published output")?;
    let bad_jpeg = dir.path().join("quality.jpg");
    require(
        export(&bad_jpeg, &frame, BitDepth::Eight, 0, &[]).is_err(),
        "invalid JPEG quality must return an error",
    )?;
    let changed = dir.path().join("replacement.tiff");
    let staged = prepare_export(&changed, &frame, BitDepth::Eight, 90, &[])?;
    fs::create_dir(&changed)?;
    fs::write(changed.join("keep"), b"destination directory")?;
    require(
        staged.publish().is_err(),
        "publishing over a directory must fail",
    )?;
    require(
        fs::read(changed.join("keep"))? == b"destination directory",
        "publish damaged destination",
    )?;
    require(
        fs::read_dir(dir.path())?.count() == 2,
        "failed jobs leaked staging",
    )
}

/// A successful export destination cannot replace an input file or its hardlink.
///
/// # Errors
/// Returns a fixture creation error or an alias protection failure.
#[test]
fn forbids_source_aliases_even_during_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let source = dir.path().join("source.png");
    let alias = dir.path().join("alias.png");
    let late_alias = dir.path().join("late-alias.png");
    fs::write(&source, b"source bytes")?;
    fs::hard_link(&source, &alias)?;
    let sources = [source.clone()];
    let frame = rgb16_frame()?;
    for path in [&source, &alias] {
        require(
            export(path, &frame, BitDepth::Sixteen, 90, &sources).is_err(),
            "reject replace of source path or its alias",
        )?;
        require(fs::read(path)? == b"source bytes", "source alias changed")?;
    }
    let prepared = prepare_export(&late_alias, &frame, BitDepth::Sixteen, 90, &sources)?;
    fs::hard_link(&source, &late_alias)?;
    require(prepared.publish().is_err(), "recheck destination at commit")?;
    require(
        fs::read(&late_alias)? == b"source bytes",
        "late alias changed",
    )?;
    require(fs::read(source)? == b"source bytes", "source changed")?;
    require(
        fs::read_dir(dir.path())?.count() == 3,
        "alias guard leaked staged file",
    )?;
    require(
        ExportFormat::from_path(Path::new("no-suffix")).is_err(),
        "require format suffix",
    )
}

/// Retain original ICC bytes through each integer codec that accepts profiles.
///
/// # Errors
/// Returns an encoding, decoding, or metadata preservation failure.
#[test]
fn profiled_exports_preserve_original_icc_bytes() -> TestResult {
    use image::ImageEncoder;

    let directory = tempfile::tempdir()?;
    let input = directory.path().join("profiled-input.png");
    let profile = include_bytes!("data/srgb.png.icc");
    let source = fs::File::create(&input)?;
    let mut encoder = image::codecs::png::PngEncoder::new(source);
    encoder.set_icc_profile(profile.to_vec())?;
    encoder.write_image(&[11, 49, 137], 1, 1, image::ExtendedColorType::Rgb8)?;
    let frame = pancake::image_io::decode(&input)?;
    for (name, depth) in [
        ("metadata.png", BitDepth::Eight),
        ("metadata.jpg", BitDepth::Eight),
        ("metadata.tif", BitDepth::Eight),
        ("metadata.tiff", BitDepth::Sixteen),
    ] {
        let path = directory.path().join(name);
        export(&path, &frame, depth, 90, std::slice::from_ref(&input))?;
        let restored = pancake::image_io::decode(&path)?;
        if matches!(name, "metadata.tif" | "metadata.tiff") {
            let mut independent = tiff::decoder::Decoder::new(fs::File::open(&path)?)?;
            let read_back = independent.get_tag_u8_vec(tiff::tags::Tag::IccProfile);
            require(
                read_back
                    .as_ref()
                    .is_ok_and(|value| value == profile.as_slice()),
                &format!(
                    "{name} direct TIFF decoder could not read original profile: {read_back:?}"
                ),
            )?;
            require(
                (restored.width(), restored.height()) == (1, 1)
                    && restored
                        .samples()
                        .iter()
                        .copied()
                        .zip(frame.samples())
                        .all(|(decoded, source)| (decoded - source).abs() <= 1e-6),
                &format!("{name} changed sample values or geometry when preserving ICC metadata"),
            )?;
        }
        require(
            restored.icc_profile() == Some(profile.as_slice()),
            &format!("{name} did not preserve the embedded ICC bytes"),
        )?;
    }
    Ok(())
}

/// A retained image can overwrite an unrelated export after its source is deleted.
///
/// # Errors
/// Returns fixture errors, a blocked replacement, or a wrong decoded result.
#[test]
fn retained_image_can_publish_after_source_disappears() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("old-source.png");
    let destination = directory.path().join("current-result.png");
    let frame = rgb16_frame()?;
    fs::write(&source, b"source identity")?;
    fs::write(&destination, b"old visible result")?;
    let staged = prepare_export(
        &destination,
        &frame,
        BitDepth::Sixteen,
        90,
        std::slice::from_ref(&source),
    )?;
    fs::remove_file(&source)?;
    staged.publish()?;
    require(
        image::open(&destination)?.into_rgb16().into_raw() == RGB16,
        "missing retained source blocked or changed the export",
    )?;
    require(
        fs::read_dir(directory.path())?.count() == 1,
        "publish leaked a staging file",
    )
}

/// Publication failure at the commit boundary leaves the previous file intact.
///
/// # Errors
/// Returns a staging setup failure or violation of atomic publication.
#[cfg(unix)]
#[test]
fn losing_staging_before_publication_does_not_damage_destination() -> TestResult {
    let directory = tempfile::tempdir()?;
    let destination = directory.path().join("current-result.png");
    fs::write(&destination, b"visible previous result")?;
    let frame = rgb16_frame()?;
    let staged = prepare_export(&destination, &frame, BitDepth::Sixteen, 90, &[])?;
    let sibling = fs::read_dir(directory.path())?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|entry| entry.path())
        .find(|path| path != &destination)
        .ok_or_else(|| io::Error::other("missing staged file"))?;
    fs::remove_file(sibling)?;
    let error = staged
        .publish()
        .err()
        .ok_or_else(|| io::Error::other("publication succeeded with no staged file"))?;
    require(
        error.contains("current-result.png"),
        &format!("missing destination diagnostic: {error}"),
    )?;
    require(
        fs::read(&destination)? == b"visible previous result",
        "unsuccessful publication replaced the previous result",
    )?;
    require(
        fs::read_dir(directory.path())?.count() == 1,
        "publication leaked staging",
    )
}

/// Fail safely when a source's parent becomes a file, preserving the old export.
///
/// # Errors
/// Returns a fixture setup or failed source alias inspection contract.
#[cfg(unix)]
#[test]
fn source_inspection_error_preserves_existing_destination() -> TestResult {
    let directory = tempfile::tempdir()?;
    let parent = directory.path().join("source-parent");
    let source = parent.join("source.png");
    let destination = directory.path().join("current-result.png");
    fs::create_dir(&parent)?;
    fs::write(&source, b"current source")?;
    fs::write(&destination, b"visible previous result")?;
    let frame = rgb16_frame()?;
    let staged = prepare_export(
        &destination,
        &frame,
        BitDepth::Sixteen,
        90,
        std::slice::from_ref(&source),
    )?;
    fs::remove_file(&source)?;
    fs::remove_dir(&parent)?;
    fs::write(
        &parent,
        b"different object at the previous source directory",
    )?;
    let error = staged
        .publish()
        .err()
        .ok_or_else(|| io::Error::other("source inspection should fail"))?;
    require(
        error.contains("cannot inspect input") && error.contains("source-parent"),
        &format!("expected input inspection diagnostic: {error}"),
    )?;
    require(
        fs::read(&destination)? == b"visible previous result",
        "source inspection failure replaced the previous result",
    )?;
    require(
        fs::read_dir(directory.path())?.count() == 2,
        "alias inspection leaked staging",
    )
}
