//! Filesystem contracts for the CLI and GUI's shared validated job requests.

use std::error::Error;
use std::fs;

use pancake::config::{JobConfig, JobSettings};

/// Literal and pattern inputs resolve to a single canonical, sorted sequence.
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
    assert_eq!(
        job.inputs(),
        &[first.canonicalize()?, second.canonicalize()?],
        "ordered filenames, including hard-link names, must be preserved"
    );
    assert_eq!(
        job.output(),
        Some(output.as_path()),
        "export destination must be preserved"
    );
    assert_eq!(
        job.settings().kernel_size,
        3,
        "default settings must be available to both frontends"
    );
    Ok(())
}

/// Rejecting a destination that resolves to an input leaves the source intact.
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
    assert!(
        job.err()
            .is_some_and(|error| error.contains("also an input")),
        "an input destination is invalid"
    );
    assert_eq!(fs::read(first)?, b"first", "source bytes changed");
    Ok(())
}
