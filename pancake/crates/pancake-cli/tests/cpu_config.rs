//! Filesystem contracts for the CLI and GUI's shared validated job requests.

use std::error::Error;
use std::fs;

use clap as _;
use pancake::config::{JobConfig, JobSettings};

/// Literal and pattern inputs resolve to a single canonical, sorted sequence.
///
/// # Errors
/// Returns errors creating fixture files or unexpected job configuration.
#[test]
fn canonical_input_order_preserves_distinct_paths() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let first = directory.path().join("a [literal].png");
    let second = directory.path().join("中📷 z.png");
    fs::write(&first, b"first")?;
    fs::hard_link(&first, &second)?;
    let inputs = [
        second.clone(),
        directory.path().join("*.png"),
        first.clone(),
        directory.path().join(".").join("a [literal].png"),
    ];
    let output = directory.path().join("result.png");
    let job = JobConfig::new(&inputs, Some(output.clone()), JobSettings::default())?;
    if job.inputs() != [first.canonicalize()?, second.canonicalize()?] {
        return Err("canonical ordering must preserve distinct hard-link names".into());
    }
    if job.output() != Some(output.as_path()) || job.settings().kernel_size != 3 {
        return Err("validated job lost output or default focus settings".into());
    }
    Ok(())
}

/// Rejecting a destination that resolves to an input leaves the source intact.
///
/// # Errors
/// Returns fixture setup errors or source/output aliasing failures.
#[test]
fn source_cannot_be_its_own_export_destination() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let first = directory.path().join("a.png");
    let second = directory.path().join("b.png");
    fs::write(&first, b"first")?;
    fs::write(&second, b"second")?;
    let job = JobConfig::new(
        &[first.clone(), second],
        Some(first.clone()),
        JobSettings::default(),
    );
    if !job
        .err()
        .is_some_and(|error| error.contains("also an input"))
    {
        return Err("an input export destination must be invalid".into());
    }
    if fs::read(first)? != b"first" {
        return Err("source bytes changed".into());
    }
    Ok(())
}
