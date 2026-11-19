//! Local Linux file dialogs with native GTK widgets and native editable paths.

use std::cell::RefCell;
use std::ffi::c_ulong;
use std::path::PathBuf;
use std::rc::Rc;

use gdkx11::{X11Display, X11Window};
use gtk::atk::prelude::AtkObjectExt as _;
use gtk::glib::object::Cast as _;
use gtk::prelude::{
    BoxExt as _, DialogExt as _, EntryExt as _, FileChooserExt as _, GtkWindowExt as _,
    WidgetExt as _,
};
use gtk::{Dialog, Entry, FileChooserAction, FileChooserWidget, Label, Orientation, ResponseType};

use super::Picker;

/// Present a desktop panel and deliver one accepted path set or cancellation.
///
/// Slint continues dispatching its own input while the host services GTK's
/// main context. No nested modal GTK loop consumes the workspace's turn.
///
/// # Errors
///
/// Returns an unavailable desktop or lost path-entry error during setup.
pub(super) fn open(
    owner: Option<c_ulong>,
    kind: Picker<'_>,
    completion: impl FnOnce(Result<Vec<PathBuf>, String>) + 'static,
) -> Result<(), String> {
    let (dialog, chooser, entry) = build(kind)?;
    set_native_owner(&dialog, owner);
    let saving = matches!(kind, Picker::Save(_));
    let pending = Rc::new(RefCell::new(Some(completion)));
    let response = Rc::clone(&pending);
    let _response = dialog.connect_response(move |dialog, choice| {
        let Some(complete) = response.borrow_mut().take() else {
            return;
        };
        let outcome = if choice == ResponseType::Accept {
            selected(saving, &chooser, &entry)
        } else {
            Ok(Vec::new())
        };
        close(dialog, choice == ResponseType::DeleteEvent);
        complete(outcome);
    });
    let _destroy = dialog.connect_destroy(move |_| {
        if let Some(complete) = pending.borrow_mut().take() {
            complete(Ok(Vec::new()));
        }
    });
    dialog.show_all();
    dialog.present();
    Ok(())
}

/// Create the native dialog, its file-browser region and its editable path.
///
/// # Errors
///
/// Returns an unavailable desktop or lost path-entry error.
fn build(kind: Picker<'_>) -> Result<(Dialog, FileChooserWidget, Entry), String> {
    gtk::init()
        .map_err(|error| format!("Could not initialize the desktop file dialog: {error}"))?;
    let (title, label, accept, action) = match kind {
        Picker::Files => ("Open images", "File name", "Open", FileChooserAction::Open),
        Picker::Folder => (
            "Choose folder",
            "Directory:",
            "Select Folder",
            FileChooserAction::SelectFolder,
        ),
        Picker::Save(_) => (
            "Save image",
            "File name",
            "Save",
            FileChooserAction::SelectFolder,
        ),
    };
    let dialog = Dialog::builder()
        .title(title)
        .modal(true)
        .default_width(730)
        .default_height(490)
        .build();
    let _cancel = dialog.add_button("Cancel", ResponseType::Cancel);
    let _confirm = dialog.add_button(accept, ResponseType::Accept);
    dialog.set_default_response(ResponseType::Accept);

    let chooser = FileChooserWidget::new(action);
    chooser.set_local_only(true);
    chooser.set_select_multiple(matches!(kind, Picker::Files));

    let entry = Entry::new();
    entry
        .accessible()
        .ok_or("The file name entry was not available.")?
        .set_name(label);
    entry.set_activates_default(true);
    if let Picker::Save(name) = kind {
        entry.set_text(name);
    }
    let row = gtk::Box::new(Orientation::Horizontal, 8);
    row.pack_start(&Label::new(Some(label)), false, false, 0);
    row.pack_start(&entry, true, true, 0);
    let content = dialog.content_area();
    content.set_spacing(8);
    content.pack_start(&chooser, true, true, 0);
    content.pack_start(&row, false, false, 6);

    Ok((dialog, chooser, entry))
}

/// Bind the real GTK surface to the Slint workspace on native X11.
fn set_native_owner(dialog: &Dialog, owner: Option<c_ulong>) {
    if let Some(xid) = owner
        && let Some(display) = gtk::gdk::Display::default()
        && let Ok(display) = display.downcast::<X11Display>()
    {
        dialog.realize();
        if let Some(child) = dialog.window() {
            let parent = X11Window::foreign_new_for_display(&display, xid);
            child.set_transient_for(parent.upcast_ref());
        }
    }
}

/// Release the toolkit's modal input grab before unmapping and closing.
fn close(dialog: &Dialog, deleting: bool) {
    dialog.set_modal(false);
    dialog.hide();
    if !deleting {
        dialog.close();
    }
    if let Some(display) = gtk::gdk::Display::default() {
        display.sync();
    }
}

/// Read only a user-accepted native selection.
///
/// # Errors
///
/// Returns an error if no base directory is available for a relative path.
fn selected(
    saving: bool,
    chooser: &FileChooserWidget,
    entry: &Entry,
) -> Result<Vec<PathBuf>, String> {
    let typed = entry.text();
    if typed.is_empty() {
        return Ok(chooser.filenames());
    }
    let path = PathBuf::from(typed.as_str());
    if path.is_absolute() {
        return Ok(vec![path]);
    }
    let directory = if saving {
        chooser.filename().or_else(|| chooser.current_folder())
    } else {
        chooser.current_folder()
    };
    let directory = directory.ok_or("Select a folder before entering a relative path.")?;
    Ok(vec![directory.join(path)])
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::env;
    use std::error::Error;
    use std::ffi::OsStr;
    use std::process::Command;
    use std::rc::Rc;

    use super::{Picker, open};

    /// Select the assertion inside the isolated copy of the GUI test binary.
    const CHILD: &str = "PANCAKE_TEST_GTK_UNAVAILABLE_CHILD";

    /// Native panel setup failure leaves the caller responsible for recovery.
    /// # Errors
    /// Returns a launch error or the child test's native setup failure.
    #[test]
    fn unavailable_display_rejects_native_panel() -> Result<(), Box<dyn Error>> {
        if env::var_os(CHILD).as_deref() == Some(OsStr::new("1")) {
            return observe_unavailable_display();
        }
        let private = tempfile::tempdir()?;
        let (_, test_name) = concat!(module_path!(), "::unavailable_display_rejects_native_panel")
            .split_once("::")
            .ok_or("The GTK test name did not include its module.")?;
        let mut child = Command::new(env::current_exe()?);
        let _configured = child
            .env_clear()
            .envs(env::var_os("LD_LIBRARY_PATH").map(|paths| ("LD_LIBRARY_PATH", paths)))
            .envs(env::var_os("LLVM_PROFILE_FILE").map(|path| ("LLVM_PROFILE_FILE", path)))
            .env(CHILD, "1")
            .env("GDK_BACKEND", "x11")
            .env("HOME", private.path())
            .env("XDG_RUNTIME_DIR", private.path())
            .current_dir(private.path())
            .args(["--exact", test_name]);
        let output = child.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !output.status.success() || !stdout.contains("1 passed; 0 failed") {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "Headless GTK test failed: {}\n{stdout}\n{stderr}",
                output.status
            )
            .into());
        }
        Ok(())
    }

    /// Run the real GTK entry point with no display or desktop bus to contact.
    /// # Errors
    /// Returns an isolation failure or an unexpected panel setup outcome.
    fn observe_unavailable_display() -> Result<(), Box<dyn Error>> {
        if env::var_os("GDK_BACKEND").as_deref() != Some(OsStr::new("x11"))
            || [
                "DISPLAY",
                "WAYLAND_DISPLAY",
                "WAYLAND_SOCKET",
                "DBUS_SESSION_BUS_ADDRESS",
                "DBUS_STARTER_ADDRESS",
                "AT_SPI_BUS_ADDRESS",
            ]
            .into_iter()
            .any(|key| env::var_os(key).is_some())
        {
            return Err("The GTK child has a desktop endpoint or an unexpected backend.".into());
        }
        let completed = Rc::new(Cell::new(false));
        let completion = Rc::clone(&completed);
        let Err(error) = open(None, Picker::Files, move |_| completion.set(true)) else {
            return Err("GTK accepted a file panel without a display.".into());
        };
        if completed.get() || !error.starts_with("Could not initialize the desktop file dialog:") {
            return Err(format!("Unexpected headless GTK panel result: {error}").into());
        }
        Ok(())
    }
}
