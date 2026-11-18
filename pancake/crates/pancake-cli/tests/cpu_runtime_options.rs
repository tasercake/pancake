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
    let mut command = headless_command(directory);
    let _command = command.arg("--cpu");
    command
}

/// Base command for independently selected devices and overrides.
fn headless_command(directory: &Path) -> Command {
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

/// A malformed runtime environment fails without numerical submission.
///
/// # Errors
/// Returns fixture, process, or wrong rejection diagnostics.
#[cfg(unix)]
#[test]
fn non_unicode_runtime_override_fails_before_processing() -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    write_sources(directory.path())?;
    let output = cmd(directory.path())
        .env("PANCAKE_RUNTIME", std::ffi::OsStr::from_bytes(b"wgpu-\xff"))
        .args(["--output", "result.png"])
        .output()
        .map_err(|error| error.to_string())?;
    let diagnostic = String::from_utf8(output.stderr).map_err(|error| error.to_string())?;
    if output.status.success()
        || !output.stdout.is_empty()
        || directory.path().join("result.png").exists()
        || !diagnostic.contains("PANCAKE_RUNTIME must be valid UTF-8")
    {
        return Err(format!(
            "invalid override did not fail cleanly: {diagnostic}"
        ));
    }
    Ok(())
}

/// A loader without any accelerators permits real CPU fallback only for Auto.
///
/// # Errors
/// Returns isolation, process, numerical publication or backend evidence errors.
#[cfg(target_os = "linux")]
#[test]
fn missing_vulkan_device_falls_back_only_when_unforced() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = directory.path();
    write_sources(root)?;
    let cache = root.join("compute-cache");
    let disabled_driver = root.join("absent-icd.json");
    let base = || {
        let mut command = headless_command(root);
        let _command = command
            .env("VK_DRIVER_FILES", &disabled_driver)
            .env("VK_ICD_FILENAMES", &disabled_driver)
            .env("CUDA_VISIBLE_DEVICES", "")
            .env("XDG_RUNTIME_DIR", root)
            .env("PANCAKE_COMPUTE_CACHE_DIR", &cache)
            .args(["--output", "result.png"]);
        command
    };
    let selected = base()
        .args(["--device", "auto"])
        .env("CONFORMANCE_EVENTS_PATH", root.join("completion.jsonl"))
        .output()
        .map_err(|error| error.to_string())?;
    let diagnostic = String::from_utf8(selected.stderr).map_err(|error| error.to_string())?;
    if !selected.status.success() {
        return Err(format!(
            "Auto failed to finish on the actual CPU: {diagnostic}"
        ));
    }
    let published = root.join("result.png");
    let decoded = decode(&published)?;
    let event = std::fs::read_to_string(root.join("completion.jsonl"))
        .map_err(|error| error.to_string())?;
    if [decoded.width(), decoded.height()] != [13, 11]
        || !event.contains(r#""actual_backend":"cpu""#)
        || !event.contains(r#""physical_gpu":false"#)
        || !event.contains(r#""fallback_reason":"wgpu Vulkan found no physical GPU"#)
    {
        return Err(format!(
            "completion misrepresented the actual fallback: {event}"
        ));
    }
    let retained = std::fs::read(&published).map_err(|error| error.to_string())?;
    for (extra, expected) in [
        (&["--gpu"][..], "found no physical GPU"),
        (&["--gpu", "--gpu-id", "0"][..], "GPU index 0 is absent"),
    ] {
        let strict = base()
            .args(extra)
            .output()
            .map_err(|error| error.to_string())?;
        let message = String::from_utf8(strict.stderr).map_err(|error| error.to_string())?;
        if strict.status.success()
            || !strict.stdout.is_empty()
            || !message.contains(expected)
            || std::fs::read(&published).map_err(|error| error.to_string())? != retained
        {
            return Err(format!(
                "strict GPU request did not preserve the completed export: {message}"
            ));
        }
    }
    Ok(())
}
