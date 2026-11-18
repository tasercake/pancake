//! Profile persistence checks using actual independent directory contents.

use std::error::Error;

use super::{Device, Preferences, Store, Theme};

/// Report a failed contract as a normal test error.
///
/// # Errors
///
/// Returns the message when the comparison fails.
fn verify(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(message.into())
    }
}

/// Different namespaces never inherit the last chosen preferences.
///
/// # Errors
///
/// Returns storage and comparison failures.
#[test]
fn saves_preferences_to_only_the_active_profile() -> Result<(), Box<dyn Error>> {
    let work = tempfile::tempdir()?;
    let current = Store::new(work.path().join("current"));
    let other = Store::new(work.path().join("fresh"));
    let defaults = Preferences::default();
    verify(
        current.read()? == defaults,
        "new profile must start at defaults",
    )?;
    let changed = Preferences::from_choices("Dark", "GPU")?;
    current.save(changed)?;
    let observed = current.read()?;
    verify(observed.theme == Theme::Dark, "persisted theme differs")?;
    verify(observed.device == Device::Gpu, "persisted device differs")?;
    verify(other.read()? == defaults, "fresh profile inherited changes")?;
    current.save(defaults)?;
    verify(
        current.read()? == defaults,
        "second save did not replace settings",
    )
}

/// A malformed profile is preserved for recovery, and invalid choices do not save.
///
/// # Errors
///
/// Returns failures from profile creation, persistence and comparisons.
#[test]
fn reports_malformed_preferences() -> Result<(), Box<dyn Error>> {
    let work = tempfile::tempdir()?;
    let store = Store::new(work.path().to_path_buf());
    let raw = "{ broken";
    std::fs::write(store.path(), raw)?;
    verify(
        store.read().is_err(),
        "malformed preferences were silently accepted",
    )?;
    verify(
        std::fs::read_to_string(store.path())? == raw,
        "read changed malformed profile",
    )?;
    verify(
        Preferences::from_choices("Chroma", "GPU").is_err(),
        "unknown theme accepted",
    )?;
    verify(
        Preferences::from_choices("Light", "other").is_err(),
        "unknown device accepted",
    )
}
