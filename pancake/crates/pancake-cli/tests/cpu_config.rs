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

/// Exhausting the scan budget prevents even an unmatched pattern from enumerating a directory.
///
/// # Errors
/// Returns fixture setup or an unexpected resource-limit diagnostic.
#[test]
fn unmatched_entries_still_consume_scan_budget() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let first = directory.path().join("frame.png");
    fs::write(&first, b"frame")?;
    let mut arguments = vec![first; 16_383];
    arguments.push(directory.path().join("absent-*.png"));
    let outcome = JobConfig::new(&arguments, None, JobSettings::default());
    if !outcome
        .err()
        .is_some_and(|error| error.contains("too many input path entries"))
    {
        return Err("glob discarded unmatched entries without checking the budget".into());
    }
    Ok(())
}

/// Nested glob components may select files across subdirectories.
///
/// # Errors
/// Returns fixture setup or an unexpected match set.
#[test]
fn nested_glob_matches_each_component() -> Result<(), Box<dyn Error>> {
    let directory = tempfile::tempdir()?;
    let first = directory.path().join("focus1");
    let second = directory.path().join("focus2");
    let ignored = directory.path().join("focus3");
    fs::create_dir(&first)?;
    fs::create_dir(&second)?;
    fs::write(&ignored, b"not a directory")?;
    let first = first.join("frame1.png");
    let second = second.join("frame2.png");
    fs::write(&first, b"first")?;
    fs::write(&second, b"second")?;
    let arguments = [directory.path().join("focus[1-3]").join("frame?.png")];
    let job = JobConfig::new(&arguments, None, JobSettings::default())?;
    if job.inputs() != [first.canonicalize()?, second.canonicalize()?] || job.output().is_some() {
        return Err("nested input pattern did not resolve to both existing files".into());
    }
    Ok(())
}
