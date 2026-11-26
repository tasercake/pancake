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

/// The profile accepts valid bytes up to its bound and diagnoses oversize data.
///
/// # Errors
/// Returns a fixture, storage, or comparison failure.
#[test]
fn rejects_an_oversized_preferences_document_without_truncating_it() -> Result<(), Box<dyn Error>> {
    let work = tempfile::tempdir()?;
    let store = Store::new(work.path().to_path_buf());
    let choices = Preferences::from_choices("Dark", "CPU")?;
    let mut raw = serde_json::to_vec(&choices)?;
    raw.resize(65_536, b' ');
    std::fs::write(store.path(), &raw)?;
    let observed = store.read()?;
    verify(
        observed == choices && observed.device.index() == 1,
        "exactly 64 KiB should retain the explicit CPU selector",
    )?;

    raw.push(b' ');
    std::fs::write(store.path(), &raw)?;
    verify(
        store
            .read()
            .is_err_and(|error| error.to_string() == "Preferences exceed 64 KiB."),
        "valid JSON above the storage limit must fail rather than parse a truncated prefix",
    )?;
    verify(
        std::fs::read(store.path())? == raw,
        "oversize input was altered during a read",
    )
}

/// Existing profiles keep their explicit theme while fresh profiles get graphite.
/// # Errors
/// Returns a persistence or comparison failure.
#[test]
fn graphite_is_the_default_for_new_profiles_and_legacy_light_is_preserved()
-> Result<(), Box<dyn Error>> {
    let work = tempfile::tempdir()?;
    let store = Store::new(work.path().to_path_buf());
    verify(
        store.read()?.theme == Theme::Dark,
        "new profiles must default to graphite",
    )?;
    std::fs::write(store.path(), r#"{"theme":"light","device":"cpu"}"#)?;
    verify(
        store.read()?.theme == Theme::Light,
        "an explicit legacy light theme must be preserved",
    )?;
    verify(
        store.read()?.device == Device::Cpu,
        "an explicit legacy device must be preserved",
    )
}

/// Restore invalid accents with the default and preserve unrelated choices.
/// # Errors
/// Returns a parse, persistence, or comparison failure.
#[test]
fn accent_round_trips_and_invalid_values_recover_independently() -> Result<(), Box<dyn Error>> {
    let work = tempfile::tempdir()?;
    let store = Store::new(work.path().to_path_buf());
    let saved = Preferences {
        accent: super::Accent::parse("  #ab12Cd  ")?,
        ..Preferences::from_choices("Light", "CPU")?
    };
    store.save(saved)?;
    verify(
        store.read()? == saved,
        "the selected accent must round-trip",
    )?;
    verify(
        saved.accent.hex() == "#ab12cd",
        "the editable accent must use canonical hexadecimal",
    )?;
    for value in [r##""#bad""##, "32", "null", r#"{"bad":true}"#] {
        std::fs::write(
            store.path(),
            format!(r#"{{"theme":"light","device":"cpu","accent":{value}}}"#),
        )?;
        let observed = store.read()?;
        verify(
            observed == Preferences::from_choices("Light", "CPU")?,
            "invalid accents must recover without losing the other choices",
        )?;
    }
    for invalid in ["#fff", "123456", "#g00000", "#80808000"] {
        verify(
            super::Accent::parse(invalid).is_err(),
            "explicit accent entry must validate RGB format",
        )?;
    }
    Ok(())
}

/// Check both themes with neutral, saturated, and clearly dark accent choices.
/// # Errors
/// Returns color parsing or self-contrast errors.
#[test]
fn accent_separates_from_fields_and_keeps_readable_foreground() -> Result<(), Box<dyn Error>> {
    use super::{Accent, contrast};
    for requested in [
        "#000000", "#ffffff", "#ff0000", "#0000ff", "#0af08c", "#666666",
    ] {
        let accent = Accent::parse(requested)?;
        for (theme, field) in [(Theme::Dark, [52; 3]), (Theme::Light, [233; 3])] {
            let (fill, ink) = accent.presentation(theme);
            verify(
                contrast(fill, field) >= 4.5_f64 && contrast(fill, ink) >= 4.5_f64,
                "the applied accent must contrast with fields and its text",
            )?;
        }
    }
    verify(
        Accent::default().presentation(Theme::Dark) == ([255; 3], [0; 3]),
        "graphite must preserve the white default with black label text",
    )
}
