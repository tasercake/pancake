//! Command-line mapping and diagnostic stream contracts.

use super::{Arguments, report};
use clap::Parser;
use pancake::config::{AlignmentMode, DevicePreference};
use std::process::ExitCode;

/// A broken pipe must exit with failure instead of panicking or claiming success.
///
/// # Panics
/// Panics if a failed write continues reporting the requested success code.
#[test]
fn inaccessible_diagnostic_stream() {
    let mut closed: &mut [u8] = &mut [];
    let result = report(&mut closed, format_args!("actual error"), ExitCode::SUCCESS);
    assert_eq!(
        result,
        ExitCode::FAILURE,
        "a failed write must fail the process"
    );
}

/// Command-line device choices select the shared validated setting.
///
/// # Errors
/// Returns parser, fixture, or settings mapping failures.
#[test]
fn argument_device_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let first = directory.path().join("a.png");
    let second = directory.path().join("b.png");
    let output = directory.path().join("output.png");
    std::fs::write(&first, b"a")?;
    std::fs::write(&second, b"b")?;
    let fixture = [
        "pancake-cli".as_ref(),
        "--input".as_ref(),
        first.as_os_str(),
        second.as_os_str(),
        "--output".as_ref(),
        output.as_os_str(),
    ];
    for (flags, expected) in [
        (&[][..], DevicePreference::Auto),
        (&["--cpu"][..], DevicePreference::Cpu),
        (&["--gpu"][..], DevicePreference::Gpu),
        (&["--device", "cpu"][..], DevicePreference::Cpu),
    ] {
        let arguments = fixture
            .into_iter()
            .chain(flags.iter().map(std::ffi::OsStr::new));
        let job = Arguments::try_parse_from(arguments)?.into_job()?;
        if job.settings().device != expected {
            return Err(format!("device mapping failed for {flags:?}").into());
        }
    }
    for (flags, expected) in [
        (&[][..], AlignmentMode::Off),
        (&["--align"][..], AlignmentMode::Translation),
        (
            &["--align", "--rotation-scale"][..],
            AlignmentMode::Similarity,
        ),
        (&["--alignment-mode", "affine"][..], AlignmentMode::Affine),
    ] {
        let arguments = fixture
            .into_iter()
            .chain(flags.iter().map(std::ffi::OsStr::new));
        let job = Arguments::try_parse_from(arguments)?.into_job()?;
        if job.settings().alignment != expected {
            return Err(format!("alignment mapping failed for {flags:?}").into());
        }
    }
    Ok(())
}
