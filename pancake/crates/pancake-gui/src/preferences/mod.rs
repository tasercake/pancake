//! Isolated native user preference storage.

use std::error::Error;
use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[cfg(test)]
mod tests;

/// Repository-supported color schemes.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    /// Light desktop controls.
    Light,
    /// Dark desktop controls.
    #[default]
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
    /// Original accent requested by the user, independent of the current theme.
    #[serde(default)]
    pub accent: Accent,
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
        Ok(Self {
            theme,
            device,
            accent: Accent::default(),
        })
    }
}

/// Persisted RGB request. The applied color is resolved for the active theme.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Accent([u8; 3]);

impl Default for Accent {
    fn default() -> Self {
        Self([255; 3])
    }
}

impl Accent {
    /// Parse the editable six-digit hexadecimal RGB value.
    /// # Errors
    /// Returns a field diagnostic for any value outside that format.
    pub fn parse(text: &str) -> Result<Self, &'static str> {
        let digits = text
            .trim()
            .strip_prefix('#')
            .filter(|digits| {
                digits.len() == 6 && digits.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            .ok_or("Enter an accent as #RRGGBB.")?;
        let [_, red, green, blue] = u32::from_str_radix(digits, 16)
            .map_err(|_| "Enter an accent as #RRGGBB.")?
            .to_be_bytes();
        Ok(Self([red, green, blue]))
    }

    /// Display the original choice as canonical hexadecimal.
    #[must_use]
    pub fn hex(self) -> String {
        let [red, green, blue] = self.0;
        format!("#{red:02x}{green:02x}{blue:02x}")
    }

    /// Keep a shared accent visible on the controlling field and choose its ink.
    #[must_use]
    pub fn presentation(self, theme: Theme) -> ([u8; 3], [u8; 3]) {
        let (field, destination) = match theme {
            Theme::Dark => ([52; 3], 255_u8),
            Theme::Light => ([233; 3], 0_u8),
        };
        let fill = (0_u16..=255)
            .map(|amount| blend(self.0, destination, amount))
            .find(|&color| contrast(color, field) >= 4.5_f64)
            .unwrap_or([destination; 3]);
        let white = [255; 3];
        let black = [0; 3];
        let ink = if contrast(fill, black) >= contrast(fill, white) {
            black
        } else {
            white
        };
        (fill, ink)
    }
}

impl Serialize for Accent {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Accent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let requested = value.as_str().and_then(|text| Self::parse(text).ok());
        Ok(requested.unwrap_or_default())
    }
}

/// Interpolate integer sRGB channels, preserving the chosen hue in either theme.
fn blend(color: [u8; 3], target: u8, amount: u16) -> [u8; 3] {
    color.map(|channel| {
        let requested = u16::from(channel).saturating_mul(255_u16.saturating_sub(amount));
        let endpoint = u16::from(target).saturating_mul(amount);
        let [_, blended] =
            (requested.saturating_add(endpoint).saturating_add(127) / 255).to_be_bytes();
        blended
    })
}

/// Relative luminance contrast for two sRGB colors.
fn contrast(first: [u8; 3], second: [u8; 3]) -> f64 {
    let (first, second) = (luminance(first), luminance(second));
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

/// Convert sRGB to linear light before calculating relative luminance.
fn luminance(color: [u8; 3]) -> f64 {
    let [red, green, blue] = color.map(|channel| {
        let srgb = f64::from(channel) / 255.0_f64;
        if srgb <= 0.040_45_f64 {
            srgb / 12.92_f64
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    });
    red.mul_add(0.2126, green.mul_add(0.7152, blue * 0.0722))
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
