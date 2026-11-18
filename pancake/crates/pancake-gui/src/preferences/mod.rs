//! Isolated native user preference storage.

use std::error::Error;
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Repository-supported color schemes.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light desktop controls.
    #[default]
    Light,
    /// Dark desktop controls.
    Dark,
}

impl Theme {
    /// Convert to the native theme selector's index.
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Light => 0,
            Self::Dark => 1,
        }
    }
}

/// Persistent compute preference, resolved by the shared core on each job.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Device {
    /// Choose a supported runtime using the configured policy.
    #[default]
    Automatic,
    /// Explicit CPU execution.
    Cpu,
    /// Request a qualified GPU.
    Gpu,
}

impl Device {
    /// Convert to the native device selector's index.
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::Automatic => 0,
            Self::Cpu => 1,
            Self::Gpu => 2,
        }
    }
}

/// Settings changed and observed in the Options panel.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Preferences {
    /// Selected color scheme.
    pub theme: Theme,
    /// Selected compute policy.
    pub device: Device,
}

impl Preferences {
    /// Convert two actual committed native selector values.
    ///
    /// # Errors
    ///
    /// Returns an error for a value outside the selector's choices.
    pub fn from_choices(theme: &str, device: &str) -> Result<Self, String> {
        let theme = match theme {
            "Light" => Theme::Light,
            "Dark" => Theme::Dark,
            _ => return Err(format!("Unknown theme {theme:?}.")),
        };
        let device = match device {
            "Automatic" => Device::Automatic,
            "CPU" => Device::Cpu,
            "GPU" => Device::Gpu,
            _ => return Err(format!("Unknown processing device {device:?}.")),
        };
        Ok(Self { theme, device })
    }
}

/// Files rooted in one application profile.
#[derive(Debug)]
pub struct Store {
    /// Profile chosen at application startup.
    directory: PathBuf,
}

impl Store {
    /// Open a profile without changing its contents.
    #[must_use]
    pub const fn new(directory: PathBuf) -> Self {
        Self { directory }
    }

    /// Resolve the configured profile, or the platform configuration directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform has no configuration directory.
    pub fn current() -> Result<Self, &'static str> {
        let directory = std::env::var_os("CONFORMANCE_UI_SETTINGS_DIR")
            .or_else(|| std::env::var_os("PANCAKE_STATE_DIR"))
            .map_or_else(
                || dirs::config_dir().map(|parent| parent.join("Pancake")),
                |value| Some(value.into()),
            )
            .ok_or("Could not locate the application settings folder.")?;
        Ok(Self::new(directory))
    }

    /// Return the active profile directory shared with other desktop settings.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Location of the independently persisted preferences file.
    fn path(&self) -> PathBuf {
        self.directory.join("preferences.json")
    }

    /// Read only the active profile and reject malformed or oversized data.
    ///
    /// # Errors
    ///
    /// Returns read errors and reports invalid JSON or excessive input length.
    pub fn read(&self) -> Result<Preferences, Box<dyn Error>> {
        let source = match File::open(self.path()) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Preferences::default()),
            Err(error) => return Err(error.into()),
        };
        let mut content = Vec::new();
        let _read = source.take(65_537).read_to_end(&mut content)?;
        if content.len() > 65_536 {
            return Err("Preferences exceed 64 KiB.".into());
        }
        Ok(serde_json::from_slice(&content)?)
    }

    /// Atomically replace the profile settings after the new file is complete.
    ///
    /// # Errors
    ///
    /// Returns errors if settings serialization or persistence fails.
    pub fn save(&self, preferences: Preferences) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(&self.directory)?;
        let mut pending = tempfile::NamedTempFile::new_in(&self.directory)?;
        serde_json::to_writer(pending.as_file_mut(), &preferences)?;
        pending.as_file().sync_all()?;
        let _published_file = pending.persist(self.path())?;
        Ok(())
    }
}
