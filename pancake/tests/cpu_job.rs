//! Real shared numerical stack, filesystem publication, and optional observer.

use std::path::{Path, PathBuf};

use pancake::config::{BitDepth, DevicePreference, JobConfig, JobSettings, Method};
use pancake::image_io::{RgbImage, decode, export};
use pancake::job::{Generation, JobControl, JobError, JobState, Options, run, run_controlled};

/// A completed weighted stack preserves identity, publishes and then logs.
///
/// # Errors
///
/// Returns invalid input, execution, I/O or invariant errors.
#[test]
fn stack_completion_follows_atomic_export() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("out.png");
    let events = directory.path().join("events.jsonl");
    let config = request(&inputs, &output)?;
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let mut progress = Vec::new();
    let result = run(config, &options, |state| progress.push(state.completed))
        .map_err(|error| error.to_string())?;
    require(
        progress == [1, 2],
        "progress did not count the accumulated sources",
    )?;
    let exported = decode(&output)?;
    let first = decode(inputs.first().ok_or("first input missing")?)?;
    require(
        exported.samples() == first.samples(),
        "sixteen-bit identity samples changed",
    )?;
    require(
        result.image().width() == first.width(),
        "result did not retain canvas dimensions",
    )?;
    let log = std::fs::read_to_string(events).map_err(|error| error.to_string())?;
    let event: serde_json::Value = serde_json::from_str(&log).map_err(|error| error.to_string())?;
    require(
        event.get("event").and_then(serde_json::Value::as_str) == Some("stack_complete")
            && event
                .get("actual_backend")
                .and_then(serde_json::Value::as_str)
                == Some("cpu")
            && event.get("completed").and_then(serde_json::Value::as_bool) == Some(true)
            && event
                .get("physical_gpu")
                .and_then(serde_json::Value::as_bool)
                == Some(false),
        "event does not describe the completed CPU job",
    )
}

/// Corruption and mid-job cancellation must preserve existing output bytes.
///
/// # Errors
///
/// Returns output, event or lifecycle invariant errors.
#[test]
fn interrupted_execution_retains_an_existing_export() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("previous.png");
    let events = directory.path().join("events.jsonl");
    std::fs::write(&output, b"previous export").map_err(|error| error.to_string())?;
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let control = JobControl::new(Generation::from_counter(1), inputs.len())?;
    let mut accepted = false;
    let outcome = run_controlled(request(&inputs, &output)?, &options, &control, |progress| {
        if progress.completed == 1 {
            accepted = control.cancel() == Ok(true);
        }
    });
    require(
        accepted,
        "cancellation request was rejected during real accumulation",
    )?;
    require(
        matches!(outcome, Err(JobError::Cancelled)),
        "initial job ignored cancellation",
    )?;
    require(
        control.snapshot()?.state == JobState::Idle,
        "cancelled job did not settle",
    )?;
    let damaged = inputs.last().ok_or("last source missing")?;
    std::fs::write(damaged, b"truncated image").map_err(|error| error.to_string())?;
    let outcome = run(request(&inputs, &output)?, &options, |_| {});
    let failure = outcome
        .err()
        .ok_or("damaged source unexpectedly succeeded")?;
    require(
        failure.to_string().contains("frame-b.png"),
        "damaged source diagnostic did not identify the source",
    )?;
    let retained = std::fs::read(output).map_err(|error| error.to_string())?;
    require(
        retained == b"previous export" && !events.exists(),
        "interrupted jobs changed a previous export or emitted completion",
    )
}

/// Prepare two tiny genuine sixteen-bit PNG files with a textured identity target.
///
/// # Errors
///
/// Returns invalid raster, allocation or encoding errors.
fn sources(directory: &Path) -> Result<[PathBuf; 2], String> {
    let inputs = [directory.join("frame-a.png"), directory.join("frame-b.png")];
    let samples = (0_u16..216)
        .map(|sample| f32::from(sample) / 216.0)
        .collect();
    let original = RgbImage::new(9, 8, samples)?;
    for path in &inputs {
        export(path, &original, BitDepth::Sixteen, 95, &[])?;
    }
    Ok(inputs)
}

/// Select forced CPU weighted processing with lossless integer export.
///
/// # Errors
///
/// Returns source resolution and configuration errors.
fn request(inputs: &[PathBuf], output: &Path) -> Result<JobConfig, String> {
    JobConfig::new(
        inputs,
        Some(output.to_owned()),
        JobSettings {
            bit_depth: BitDepth::Sixteen,
            device: DevicePreference::Cpu,
            method: Method::WeightedAverage,
            ..JobSettings::default()
        },
    )
}

/// Turn an unmet expectation into a test failure.
///
/// # Errors
///
/// Returns a message when the expectation is false.
fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}
