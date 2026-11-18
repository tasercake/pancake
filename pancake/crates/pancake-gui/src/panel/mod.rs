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
pub fn select(window: &ComponentInstance, kind: Picker<'_>) -> Result<Vec<PathBuf>, String> {
    let selected = linux::choose(kind);
    restore_focus(window);
    selected
}

/// Return focus from a native GTK popup to its actual Slint owner.
#[cfg(target_os = "linux")]
pub fn restore_focus(window: &ComponentInstance) {
    use slint::ComponentHandle as _;
    use slint::winit_030::WinitWindowAccessor as _;

    service_events();
    if let Some(display) = gtk::gdk::Display::default() {
        display.sync();
    }
    let _focused = window
        .window()
        .with_winit_window(slint::winit_030::winit::window::Window::focus_window);
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

/// Dispatch pending native panel events when the Slint event loop owns the UI.
///
/// When a modal GTK dialog closes, its native accessibility tree still needs
/// to process window removal. A bounded main-context drain integrates both
/// toolkits without keeping a second UI thread or retaining the old dialog.
#[cfg(target_os = "linux")]
pub fn service_events() {
    if gtk::is_initialized_main_thread() {
        let context = gtk::glib::MainContext::default();
        for _ in 0_u8..32 {
            if !context.pending() {
                break;
            }
            let _handled = context.iteration(false);
        }
    }
}

/// Other supported desktops present native file panels through a shared loop.
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub const fn service_events() {}
