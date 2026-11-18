//! Platform-owned file selection for the native workspace.

use std::path::PathBuf;

use slint_interpreter::ComponentInstance;

#[cfg(target_os = "linux")]
mod linux;

/// File selection purpose.
#[derive(Clone, Copy, Debug)]
pub enum Picker<'a> {
    /// Input images, selected individually or as a group.
    Files,
    /// A source or destination directory.
    Folder,
    /// An output path in a chosen directory.
    Save(&'a str),
}

/// Use the platform dialog with a cancellation result distinct from an error.
///
/// # Errors
///
/// Returns a platform dialog or path-resolution error.
#[cfg(target_os = "linux")]
pub fn select(_window: &ComponentInstance, kind: Picker<'_>) -> Result<Vec<PathBuf>, String> {
    linux::choose(kind)
}

/// Use the platform dialog with native window parenting.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn select(window: &ComponentInstance, kind: Picker<'_>) -> Result<Vec<PathBuf>, String> {
    use slint::ComponentHandle as _;

    let panel = rfd::FileDialog::new().set_parent(&window.window().window_handle());
    let selected = match kind {
        Picker::Files => panel
            .set_title("Open images")
            .pick_files()
            .unwrap_or_default(),
        Picker::Folder => panel
            .set_title("Choose folder")
            .pick_folder()
            .into_iter()
            .collect(),
        Picker::Save(name) => panel
            .set_title("Save image")
            .set_file_name(name)
            .save_file()
            .into_iter()
            .collect(),
    };
    Ok(selected)
}
