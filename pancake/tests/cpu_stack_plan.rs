//! Native CPU coverage of every real stack method, reports, references and crop.

use std::path::{Path, PathBuf};

use pancake::config::{
    AlignmentMode, AlignmentReference, BitDepth, DevicePreference, JobConfig, JobSettings, Method,
};
use pancake::image_io::{RgbImage, decode, export};
use pancake::job::{Generation, JobControl, Options, run, run_controlled};
use pancake::runtime::Backend;

/// Exercise real in-memory results, exposure and focus streams, auto and slabs.
///
/// # Errors
///
/// Returns invalid resources, numerical results or public result metadata.
#[test]
fn every_method_preserves_a_textured_identity_stack() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let width = 23;
    let height = 17;
    let image = textured(width, height, 0)?;
    let sources = create_sources(directory.path(), [&image, &image, &image, &image, &image])?;
    let reference = decode(sources.first().ok_or("missing first input")?)?;
    for method in [
        Method::WeightedAverage,
        Method::DepthMap,
        Method::Laplacian,
        Method::Hybrid,
        Method::ExposureFusion,
    ] {
        let settings = JobSettings {
            method,
            auto: true,
            quality_report: true,
            slab_size: 3,
            slab_overlap: 1,
            device: DevicePreference::Cpu,
            ..JobSettings::default()
        };
        let request = JobConfig::new(&sources, None, settings)?;
        let mut completed = Vec::new();
        let result = run(request, &Options::default(), |snapshot| {
            completed.push(snapshot.completed);
        })
        .map_err(|error| error.to_string())?;
        require(
            completed == [1, 2, 3, 4, 5],
            "overlap counted an original more than once",
        )?;
        require(
            result.sources() == sources
                && result.settings() == &settings
                && result.backend() == Backend::Cpu
                && result.warnings().is_empty()
                && result.crop() == result.canvas()
                && result.safe_crop() == Some(result.canvas())
                && result.source_transform(5).is_none(),
            "in-memory result lost geometry or job metadata",
        )?;
        require(
            result
                .sharpness()
                .is_some_and(|value| value.is_finite() && value > 0.0),
            "textured output has no real sharpness report",
        )?;
        require(
            close_samples(result.image().samples(), reference.samples(), 0.000_015),
            "identity stack changed samples",
        )?;
    }
    Ok(())
}

/// Run actual registration models on a shifted source series and crop the output.
///
/// # Errors
///
/// Returns source preparation, registration or retained-result errors.
#[test]
fn registration_preserves_all_reference_conventions_and_crop() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let original = textured(63, 49, 0)?;
    let shifted = textured(63, 49, 3)?;
    let inputs = create_sources(directory.path(), [&original, &shifted, &original])?;
    for (mode, reference) in [
        (AlignmentMode::Translation, AlignmentReference::First),
        (AlignmentMode::Euclidean, AlignmentReference::Middle),
        (AlignmentMode::Similarity, AlignmentReference::Previous),
        (AlignmentMode::Affine, AlignmentReference::First),
        (AlignmentMode::Breathing, AlignmentReference::Previous),
    ] {
        let settings = JobSettings {
            method: Method::WeightedAverage,
            alignment: mode,
            alignment_reference: reference,
            device: DevicePreference::Cpu,
            auto_crop: true,
            max_precision: true,
            ..JobSettings::default()
        };
        let request = JobConfig::new(&inputs, None, settings)?;
        let result =
            run(request, &Options::default(), |_| {}).map_err(|error| error.to_string())?;
        let canvas = result.canvas();
        let crop = result.crop();
        let transform = result
            .source_transform(1)
            .ok_or("missing registered source")?;
        let matrix = transform.matrix();
        let [[_, _, shift_x], [_, _, shift_y]] = matrix;
        require(
            matrix.into_iter().flatten().all(f32::is_finite),
            "registered transform is not finite",
        )?;
        if reference == AlignmentReference::Middle {
            require(
                shift_x.abs() < 0.05 && shift_y.abs() < 0.05,
                "middle reference moved itself",
            )?;
        } else {
            require(
                (2.0..=4.0).contains(&shift_x) && shift_y.abs() < 1.0,
                "registration lost the independent three-pixel shift",
            )?;
        }
        require(
            crop.x() > 0
                && crop.width() < canvas.width()
                && crop.width() == result.image().width()
                && crop.height() == result.image().height()
                && result.safe_crop() == Some(crop),
            "auto-crop lost the rectangle retained with the full-precision result",
        )?;
    }
    Ok(())
}

/// Process differently sized originals both with and without registration.
///
/// # Errors
///
/// Returns invalid source, geometry or size-normalization error.
#[test]
fn mixed_inputs_use_the_first_frame_canvas() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let first = RgbImage::new(33, 25, [0.1, 0.5, 0.9].repeat(825))?;
    let second = RgbImage::new(31, 21, [0.1, 0.5, 0.9].repeat(651))?;
    let paths = create_sources(directory.path(), [&first, &second])?;
    for alignment in [AlignmentMode::Off, AlignmentMode::Translation] {
        let config = JobConfig::new(
            &paths,
            None,
            JobSettings {
                method: Method::DepthMap,
                alignment,
                device: DevicePreference::Cpu,
                ..JobSettings::default()
            },
        )?;
        let stack = run(config, &Options::default(), |_| {}).map_err(|error| error.to_string())?;
        require(
            stack.image().width() == first.width() && stack.image().height() == first.height(),
            "mixed-source job lost its first-frame target dimensions",
        )?;
        require(
            close_samples(stack.image().samples(), first.samples(), 0.000_016),
            "constant mixed frames gained false edges",
        )?;
        let [[scale_x, _, _], [_, scale_y, _]] = stack
            .source_transform(1)
            .ok_or("missing resampled source transform")?
            .matrix();
        require(
            (0.93..0.95).contains(&scale_x) && (0.83..0.85).contains(&scale_y),
            "reported geometry failed to describe scaled original coordinates",
        )?;
    }
    Ok(())
}

/// Construct deterministic nonrepeating luminance with a horizontal shift.
///
/// # Errors
///
/// Returns validation failures while constructing the raster.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "fixture axes and increments are small checked inputs"
)]
fn textured(width: u16, height: u16, offset: u16) -> Result<RgbImage, String> {
    let samples = (0..height)
        .flat_map(|y| {
            (0..width).flat_map(move |x| {
                let column = u32::from(x.saturating_sub(offset));
                let row = u32::from(y);
                let mixed = (column + 3).wrapping_mul(0x8d12_467b)
                    ^ (row + 1).wrapping_mul(0x42c3_7753)
                    ^ (column * row + 1).wrapping_mul(0x0313_bcf1);
                let [low, high, _, _] = mixed.to_le_bytes();
                [
                    f32::from(low) / 255.0,
                    f32::from(high) / 255.0,
                    f32::from(low ^ high) / 255.0,
                ]
            })
        })
        .collect();
    RgbImage::new(usize::from(width), usize::from(height), samples)
}

/// Save and return distinct immutable export sources in the expected input order.
///
/// # Errors
///
/// Returns an encoder or atomic publication error.
fn create_sources<const N: usize>(
    directory: &Path,
    images: [&RgbImage; N],
) -> Result<Vec<PathBuf>, String> {
    images
        .into_iter()
        .enumerate()
        .map(|(index, image)| {
            let path = directory.join(format!("source-{index}.png"));
            export(&path, image, BitDepth::Sixteen, 95, &[])?;
            Ok(path)
        })
        .collect()
}

/// Match all sample coordinates with the supplied absolute precision tolerance.
fn close_samples(actual: &[f32], expected: &[f32], tolerance: f32) -> bool {
    actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(value, reference)| (value - reference).abs() <= tolerance)
}

/// Preserve a useful failure message for unmet native integration requirements.
///
/// # Errors
///
/// Returns the supplied diagnostic.
fn require(condition: bool, diagnostic: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| diagnostic.to_owned())
}

/// One real active controller cannot admit a second executor.
///
/// # Errors
///
/// Returns stack or file errors and detects a second result replacing the first.
#[test]
fn shared_ticket_cannot_admit_a_second_stack() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let image = textured(13, 11, 0)?;
    let inputs = create_sources(directory.path(), [&image, &image])?;
    let original = directory.path().join("first-result.png");
    let contender = directory.path().join("second-result.png");
    let settings = JobSettings {
        method: Method::WeightedAverage,
        device: DevicePreference::Cpu,
        ..JobSettings::default()
    };
    let eligible = JobConfig::new(&inputs, Some(original.clone()), settings)?;
    let mut duplicate = Some(JobConfig::new(&inputs, Some(contender.clone()), settings)?);
    let control = JobControl::new(Generation::from_counter(1), inputs.len())?;
    let options = Options::default();
    let mut second_error = None;
    let first = run_controlled(eligible, &options, &control, |progress| {
        if progress.completed == 1
            && let Some(config) = duplicate.take()
        {
            second_error = run_controlled(config, &options, &control, |_| {})
                .err()
                .map(|error| error.to_string());
        }
    });
    require(
        original.is_file() && !contender.exists(),
        "second executor could supersede the job's sole result publication",
    )?;
    let first = first.map_err(|error| error.to_string())?;
    require(
        first.image().width() == 13,
        "admitted executor lost its image",
    )?;
    require(
        second_error.is_some_and(|error| error.contains("already owns an execution")),
        "duplicate execution was not rejected at admission",
    )
}

/// A ticket for a different number of sources cannot claim this request.
///
/// # Errors
///
/// Returns setup errors or an acknowledgement after the mismatched job starts.
#[test]
fn ticket_input_count_must_match_before_processing() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let image = textured(13, 11, 0)?;
    let inputs = create_sources(directory.path(), [&image, &image])?;
    let output = directory.path().join("out.png");
    let request = JobConfig::new(
        &inputs,
        Some(output.clone()),
        JobSettings {
            method: Method::WeightedAverage,
            device: DevicePreference::Cpu,
            ..JobSettings::default()
        },
    )?;
    let control = JobControl::new(Generation::from_counter(2), 1)?;
    let mut progress = 0;
    let failure = run_controlled(request, &Options::default(), &control, |snapshot| {
        progress = snapshot.completed;
    })
    .err()
    .ok_or("mismatched ticket ran to completion")?;
    require(
        progress == 0 && !output.exists() && failure.to_string().contains("input count"),
        "mismatched ticket reached numerical submission",
    )
}
