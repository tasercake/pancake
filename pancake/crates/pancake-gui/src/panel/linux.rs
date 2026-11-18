//! Local Linux file dialogs with native GTK widgets and native editable paths.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk::atk::prelude::AtkObjectExt as _;
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
    kind: Picker<'_>,
    completion: impl FnOnce(Result<Vec<PathBuf>, String>) + 'static,
) -> Result<(), String> {
    let (dialog, chooser, entry) = build(kind)?;
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
        if choice == ResponseType::DeleteEvent {
            dialog.hide();
        } else {
            close(dialog);
        }
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

/// Unmap the actual dialog before requesting the toolkit's native close.
fn close(dialog: &Dialog) {
    dialog.hide();
    dialog.close();
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
