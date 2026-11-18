//! Public command-line tests exercise the built binary without a display.

use std::error::Error;
use std::process::{Command, Output};

/// Invoke the current test build with all display variables removed.
fn run(args: &[&str]) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .args(args)
        .env_remove("DISPLAY")
        .env_remove("WAYLAND_DISPLAY")
        .output()
}

/// A user can discover the actual option surface without a display.
#[test]
fn help_is_headless() -> Result<(), Box<dyn Error>> {
    let output = run(&["--help"])?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(output.status.success(), "help must succeed");
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
        assert!(stdout.contains(option), "help must document {option}");
    }
    assert!(output.stderr.is_empty(), "help must write to stdout");
    Ok(())
}

/// Missing or unsupported options fail before any processing starts.
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
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(stderr.contains(expected), "{args:?}: {stderr}");
    }
    Ok(())
}

/// Numeric and cross-option constraints reject the actual offending value.
#[test]
fn validation_diagnostics_identify_the_invalid_value() -> Result<(), Box<dyn Error>> {
    for (arguments, expected) in [
        (["--max-scale-step", "1"], "--max-scale-step"),
        (["--max-scale-step", "-0.1"], "-0.1"),
        (["--slab-size", "1"], "--slab-size"),
        (["--slab-overlap", "1"], "--slab-overlap"),
    ] {
        let output = run(&[
            "--input",
            "a.png",
            "b.png",
            "--output",
            "out.png",
            arguments.first().copied().ok_or("missing option")?,
            arguments.last().copied().ok_or("missing value")?,
        ])?;
        let stderr = String::from_utf8(output.stderr)?;
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(stderr.contains(expected), "{arguments:?}: {stderr}");
    }
    Ok(())
}

/// File discovery must fail for a missing pattern and for a deduplicated singleton.
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
    assert!(!rejected.status.success(), "duplicate inputs must fail");
    assert!(
        String::from_utf8(rejected.stderr)?.contains("found 1"),
        "existing bracketed name is a literal"
    );

    let rejected = Command::new(env!("CARGO_BIN_EXE_pancake-cli"))
        .args(["--input"])
        .arg(directory.path().join("missing-*.png"))
        .arg("--output")
        .arg(&output)
        .output()?;
    assert!(!rejected.status.success(), "a missing pattern must fail");
    assert!(
        String::from_utf8(rejected.stderr)?.contains("matched no files"),
        "missing pattern should be identified before processing"
    );
    assert!(!output.exists(), "rejection must leave output unpublished");
    Ok(())
}
