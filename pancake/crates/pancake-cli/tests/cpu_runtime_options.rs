//! Public headless diagnostics for execution settings, progress and export.

use std::path::Path;
use std::process::Command;

use clap as _;
use pancake::config::BitDepth;
use pancake::image_io::{RgbImage, SampleEncoding, decode, export};

/// Environment mistakes fail before numerical processing and preserve exports.
///
/// # Errors
///
/// Returns a fixture, process, diagnostic or existing-export integrity error.
#[test]
fn invalid_or_conflicting_runtime_preserves_the_export() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    write_sources(directory.path())?;
    let destination = directory.path().join("previous.png");
    std::fs::write(&destination, b"previous export").map_err(|error| error.to_string())?;
    for (runtime, expected) in [
        ("unknown-runtime", ["PANCAKE_RUNTIME", "unknown-runtime"]),
        ("wgpu", ["wgpu", "conflicts with explicit CPU"]),
    ] {
        let outcome = cmd(directory.path())
            .env("PANCAKE_RUNTIME", runtime)
            .args(["--output", "previous.png"])
            .output()
            .map_err(|error| error.to_string())?;
        let message = String::from_utf8(outcome.stderr).map_err(|error| error.to_string())?;
        if outcome.status.success() || !expected.into_iter().all(|term| message.contains(term)) {
            return Err(format!(
                "runtime {runtime} returned {:?}: {message}",
                outcome.status
            ));
        }
        let retained = std::fs::read(&destination).map_err(|error| error.to_string())?;
        if retained != b"previous export" || !outcome.stdout.is_empty() {
            return Err(format!(
                "rejected runtime {runtime} changed the output or emitted progress"
            ));
        }
    }
    Ok(())
}

/// A successful numeric stack reports measured sharpness and actual JPEG depth.
///
/// # Errors
///
/// Returns a fixture, process, decode or missing diagnostic error.
#[test]
fn jpeg_depth_warning_and_quality_describe_a_real_export() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    write_sources(directory.path())?;
    let outcome = cmd(directory.path())
        .args([
            "--output",
            "stack.jpg",
            "--bit-depth",
            "16",
            "--quality-report",
        ])
        .output()
        .map_err(|error| error.to_string())?;
    let diagnostics = String::from_utf8(outcome.stderr).map_err(|error| error.to_string())?;
    let output = String::from_utf8(outcome.stdout).map_err(|error| error.to_string())?;
    if !outcome.status.success()
        || !diagnostics.contains("16-bit request will be exported as 8-bit JPEG")
    {
        return Err(format!(
            "JPEG export did not explain its actual depth: {diagnostics}"
        ));
    }
    let sharpness = output
        .lines()
        .find_map(|line| line.strip_prefix("sharpness "))
        .ok_or("measured sharpness is missing")?
        .parse::<f32>()
        .map_err(|error| error.to_string())?;
    if !output.contains("progress 2/2") || !sharpness.is_finite() || sharpness <= 0.0 {
        return Err(format!(
            "measured sharpness or progress was missing: {output}"
        ));
    }
    let decoded = decode(&directory.path().join("stack.jpg"))?;
    if (decoded.width(), decoded.height(), decoded.sample_encoding())
        != (13, 11, SampleEncoding::U8)
    {
        return Err(
            "published JPEG did not have the requested geometry and eight-bit precision".to_owned(),
        );
    }
    Ok(())
}

/// Create one common headless command with isolated observer and runtime options.
fn cmd(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_pancake-cli"));
    let _command = command
        .current_dir(directory)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("PANCAKE_RUNTIME")
        .env_remove("CONFORMANCE_EVENTS_PATH")
        .args([
            "--input",
            "first.png",
            "second.png",
            "--cpu",
            "--method",
            "weighted_average",
        ]);
    command
}

/// Use a real, textured sixteen-bit two-frame stack.
///
/// # Errors
///
/// Returns a raster validation or publication error.
fn write_sources(directory: &Path) -> Result<(), String> {
    let samples = (0_u16..143)
        .flat_map(|index| {
            if index.is_multiple_of(3) {
                [0.1, 0.55, 0.9]
            } else {
                [0.8, 0.1, 0.2]
            }
        })
        .collect();
    let image = RgbImage::new(13, 11, samples)?;
    for name in ["first.png", "second.png"] {
        export(&directory.join(name), &image, BitDepth::Sixteen, 95, &[])?;
    }
    Ok(())
}
