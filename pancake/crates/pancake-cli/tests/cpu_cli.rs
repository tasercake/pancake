//! Public command-line tests exercise the built binary without a display.

use std::error::Error;
use std::process::{Command, Output};

use clap as _;
use pancake::image_io::decode;

/// Invoke the current test build with all display variables removed.
///
/// # Errors
/// Returns a failed process launch or output read.
fn run(args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
}

/// Preserve an explanation when an external process violates its contract.
///
/// # Errors
/// Returns the diagnostic when the condition fails.
fn require(condition: bool, diagnostic: impl Into<Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(diagnostic.into())
    }
}

/// A user can discover the actual option surface without a display.
///
/// # Errors
/// Returns a process launch or malformed help error.
#[test]
fn help_is_headless() -> Result<(), Box<dyn Error>> {
    let output = run(&["--help"])?;
    let stdout = String::from_utf8(output.stdout)?;
    require(output.status.success(), "help must succeed")?;
    for option in [
        "--input",
        "--output",
        "--method",
        "--align",
        "--alignment-mode",
        "--alignment-ref",
        "--kernel-size",
        "--pyramid-levels",
        "--scale-factor",
        "--max-scale-step",
        "--max-precision",
        "--slab-size",
        "--slab-overlap",
        "--auto",
        "--auto-crop",
        "--quality",
        "--quality-report",
        "--bit-depth",
        "--gpu-id",
        "--rotation-scale",
        "--cpu",
        "--gpu",
        "--device",
    ] {
        require(
            stdout.contains(option),
            format!("help must document {option}"),
        )?;
    }
    for (heading, option) in [
        ("Sources:", "--input"),
        ("Stack:", "--method"),
        ("Alignment:", "--alignment-mode"),
        ("Output:", "--bit-depth"),
        ("Compute:", "--device"),
    ] {
        let mut group = stdout
            .lines()
            .skip_while(|line| *line != heading)
            .skip(1)
            .take_while(|line| line.is_empty() || line.starts_with(' '));
        require(
            group.any(|line| line.trim_start().starts_with(option)),
            format!("{heading} help must document {option}"),
        )?;
    }
    require(
        !stdout.contains('\x1b'),
        "redirected help must be plain text",
    )?;
    require(output.stderr.is_empty(), "help must write to stdout")?;
    Ok(())
}

/// Missing or unsupported options fail before any processing starts.
///
/// # Errors
/// Returns a process launch, acknowledgement, or diagnostic failure.
#[test]
fn parser_diagnostics_identify_the_invalid_request() -> Result<(), Box<dyn Error>> {
    let arguments: &[(&[&str], &str)] = &[
        (&[], "--input"),
        (&["--input", "a.png"], "--output"),
        (&["--output", "out.png"], "--input"),
        (
            &[
                "--input", "a.png", "--output", "out.png", "--method", "unknown",
            ],
            "unknown",
        ),
        (
            &[
                "--input",
                "a.png",
                "--output",
                "out.png",
                "--bit-depth",
                "12",
            ],
            "12",
        ),
        (
            &["--input", "a.png", "--output", "out.png", "--cpu", "--gpu"],
            "--gpu",
        ),
    ];
    for (args, expected) in arguments {
        let output = run(args)?;
        let stderr = String::from_utf8(output.stderr)?;
        require(
            output.status.code() == Some(2_i32),
            format!("{args:?}: exit status"),
        )?;
        require(stderr.contains(expected), format!("{args:?}: {stderr}"))?;
    }
    Ok(())
}

/// Numeric and cross-option constraints reject the actual offending value.
///
/// # Errors
/// Returns a wrong exit status or a missing diagnostic.
#[test]
fn validation_diagnostics_identify_the_invalid_value() -> Result<(), Box<dyn Error>> {
    for ([option, value], expected) in [
        (["--max-scale-step", "1"], "--max-scale-step"),
        (["--max-scale-step", "-0.1"], "-0.1"),
        (["--slab-size", "1"], "--slab-size"),
        (["--slab-overlap", "1"], "--slab-overlap"),
    ] {
        let output = run(&[
            "--input", "a.png", "b.png", "--output", "out.png", option, value,
        ])?;
        let stderr = String::from_utf8(output.stderr)?;
        require(
            output.status.code() == Some(2_i32),
            format!("{option} {value}: exit status"),
        )?;
        require(
            stderr.contains(expected),
            format!("{option} {value}: {stderr}"),
        )?;
    }
    Ok(())
}

/// File discovery must fail for a missing pattern and for a deduplicated singleton.
///
/// # Errors
/// Returns a fixture creation, process launch, or diagnostic failure.
#[test]
fn file_diagnostics_distinguish_no_matches_and_duplicate_paths() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("frame[0].png");
    std::fs::write(&input, b"input")?;
    let output = directory.path().join("out.png");

    let rejected = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .args(["--input"])
        .arg(&input)
        .arg(&input)
        .arg("--output")
        .arg(&output)
        .output()?;
    require(!rejected.status.success(), "duplicate inputs must fail")?;
    require(
        String::from_utf8(rejected.stderr)?.contains("found 1"),
        "existing bracketed name is a literal",
    )?;

    let rejected = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .args(["--input"])
        .arg(directory.path().join("missing-*.png"))
        .arg("--output")
        .arg(&output)
        .output()?;
    require(!rejected.status.success(), "a missing pattern must fail")?;
    require(
        String::from_utf8(rejected.stderr)?.contains("matched no files"),
        "missing pattern should be identified before processing",
    )?;
    require(!output.exists(), "rejection must leave output unpublished")
}

/// A malformed image reaches the real decoder, fails with its source path, and preserves export.
///
/// # Errors
/// Returns fixture creation, process launch, or an incorrect decode outcome.
#[test]
fn malformed_inputs_are_rejected_by_the_decoder() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let output = directory.path().join("out.png");
    std::fs::write(&output, b"previous export")?;
    for extension in ["png", "dng", "npy", "unknown"] {
        let first = directory.path().join(format!("a.{extension}"));
        let second = directory.path().join(format!("b.{extension}"));
        std::fs::write(&first, b"not a supported raster")?;
        std::fs::write(&second, b"not a supported raster")?;
        let result = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
            .arg("--input")
            .arg(&first)
            .arg(&second)
            .arg("--output")
            .arg(&output)
            .arg("--cpu")
            .output()?;
        let stderr = String::from_utf8(result.stderr)?;
        require(!result.status.success(), "a real malformed input must fail")?;
        require(
            stderr.contains("could not decode input") && stderr.contains(&format!("a.{extension}")),
            format!("decoder must identify the first bad source: {stderr}"),
        )?;
        require(
            std::fs::read(&output)? == b"previous export",
            "a failed decode changed the previous export",
        )?;
    }
    Ok(())
}

/// A relative glob selects valid visible sources and publishes a completed stack.
///
/// # Errors
/// Returns fixture creation, process launch, or an unexpected exit/publication.
#[test]
fn relative_glob_processes_the_selected_sources() -> Result<(), Box<dyn Error>> {
    // Two pixels of RGB8 with a standard PNG signature and checksummed chunks.
    let png = [
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x7b,
        0x40, 0xe8, 0xdd, 0x00, 0x00, 0x00, 0x0f, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x60,
        0xa8, 0xff, 0xff, 0xbf, 0x9e, 0x01, 0x00, 0x0a, 0x79, 0x02, 0xfd, 0xfe, 0x08, 0x7e, 0xbd,
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("first.png"), png)?;
    std::fs::write(directory.path().join("second.png"), png)?;
    std::fs::write(
        directory.path().join(".excluded.png"),
        b"hidden malformed file",
    )?;
    let output = directory.path().join("out.tif");
    let result = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .current_dir(directory.path())
        .args([
            "--input",
            "*.png",
            "--output",
            "out.tif",
            "--cpu",
            "--method",
            "weighted_average",
        ])
        .output()?;
    let diagnostic = String::from_utf8(result.stderr)?;
    require(
        result.status.success(),
        format!("real stack failed: {diagnostic}"),
    )?;
    let observation = String::from_utf8(result.stdout)?;
    require(
        observation.contains("progress 2/2"),
        "real frames did not finish",
    )?;
    let result = decode(&output)?;
    let first = decode(&directory.path().join("first.png"))?;
    require(
        result.samples() == first.samples(),
        "published stack changed the expected identity image",
    )
}

/// An actual closed diagnostic pipe gives a recoverable process error.
///
/// # Errors
/// Returns pipe creation, process launch or incorrect exit status.
#[test]
fn closed_console_pipe_fails_cleanly() -> Result<(), Box<dyn Error>> {
    let (reader, writer) = std::io::pipe()?;
    drop(reader);
    let help = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .arg("--help")
        .stdout(writer)
        .status()?;
    require(
        help.code() == Some(1_i32),
        "closed help stream must exit with a recoverable error",
    )?;

    let (reader, writer) = std::io::pipe()?;
    drop(reader);
    let error = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .stderr(writer)
        .status()?;
    require(
        error.code() == Some(1_i32),
        "closed diagnostic stream must exit with a recoverable error",
    )
}
