//! Local Linux file dialogs with native GTK widgets and native editable paths.

use std::path::PathBuf;

use gtk::atk::prelude::AtkObjectExt as _;
use gtk::prelude::{
    BoxExt as _, DialogExt as _, EntryExt as _, FileChooserExt as _, GtkWindowExt as _,
    WidgetExt as _,
};
use gtk::{Dialog, Entry, FileChooserAction, FileChooserWidget, Label, Orientation, ResponseType};

use super::Picker;

/// Open a real synchronous desktop panel on the UI thread.
///
/// # Errors
///
/// Returns unavailable desktop, lost path entry, or working-directory errors.
pub(super) fn choose(kind: Picker<'_>) -> Result<Vec<PathBuf>, String> {
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

    dialog.show_all();
    let response = dialog.run();
    let outcome = if response == ResponseType::Accept {
        selected(kind, &chooser, &entry)
    } else {
        Ok(Vec::new())
    };
    // Hide accessibility state now. A Slint host does not continuously run
    // GTK's main loop to process the window-close event after Dialog::run.
    dialog.hide();
    dialog.close();
    outcome
}

/// Read only a user-accepted native selection.
///
/// # Errors
///
/// Returns an error if no base directory is available for a relative path.
fn selected(
    kind: Picker<'_>,
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
    let directory = if matches!(kind, Picker::Save(_)) {
        chooser.filename().or_else(|| chooser.current_folder())
    } else {
        chooser.current_folder()
    };
    let directory = directory.ok_or("Select a folder before entering a relative path.")?;
    Ok(vec![directory.join(path)])
}
