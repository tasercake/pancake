//! Native source reveal and Linux file/context menus.

/// Reveal the actual source in the platform's file manager.
///
/// # Errors
///
/// Returns an inaccessible source path or desktop service error.
pub fn reveal(path: &std::path::Path) -> Result<(), String> {
    let full = path
        .canonicalize()
        .map_err(|error| format!("Could not locate {}: {error}", path.display()))?;
    reveal_native(&full)
}

/// Use the Linux file-manager service and fall back to the directory handler.
///
/// # Errors
///
/// Returns an error if neither native route can open the folder.
#[cfg(target_os = "linux")]
fn reveal_native(path: &std::path::Path) -> Result<(), String> {
    use gtk::gio::{self, prelude::FileExt as _};
    use gtk::glib::variant::ToVariant as _;

    let uri = gio::File::for_path(path).uri();
    if let Ok(bus) = gio::bus_get_sync(gio::BusType::Session, gio::Cancellable::NONE)
        && bus
            .call_sync(
                Some("org.freedesktop.FileManager1"),
                "/org/freedesktop/FileManager1",
                "org.freedesktop.FileManager1",
                "ShowItems",
                Some(&(vec![uri.as_str()], "").to_variant()),
                None,
                gio::DBusCallFlags::NONE,
                3_000,
                gio::Cancellable::NONE,
            )
            .is_ok()
    {
        return Ok(());
    }
    let parent = path.parent().ok_or("Source has no containing folder.")?;
    let folder = gio::File::for_path(parent).uri();
    gio::AppInfo::launch_default_for_uri(&folder, gio::AppLaunchContext::NONE)
        .map_err(|error| format!("Could not open {}: {error}", parent.display()))
}

/// Ask Finder to select the actual source.
///
/// # Errors
///
/// Returns an open command failure or rejected Finder request.
#[cfg(target_os = "macos")]
fn reveal_native(path: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| format!("Could not open Finder: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Finder did not reveal {}: {status}",
            path.display()
        ));
    }
    Ok(())
}

/// Ask Explorer to select the actual source.
///
/// # Errors
///
/// Returns an error if the native file manager could not launch.
#[cfg(target_os = "windows")]
fn reveal_native(path: &std::path::Path) -> Result<(), String> {
    let mut select = std::ffi::OsString::from("/select,");
    select.push(path);
    let _explorer = std::process::Command::new("explorer.exe")
        .arg(select)
        .spawn()
        .map_err(|error| format!("Could not launch File Explorer: {error}"))?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub use linux::{FileAction, SourceAction, open_file, open_source, show_file, show_source};

/// Same-process GTK menu implementation for Linux.
#[cfg(target_os = "linux")]
mod linux {
    use std::cell::RefCell;
    use std::path::PathBuf;
    use std::rc::Rc;

    use atk::prelude::AtkObjectExt as _;
    use gtk::gdk::{
        self,
        prelude::{DeviceExt as _, SeatExt as _},
    };
    use gtk::glib::{MainLoop, idle_add_local_once};
    use gtk::prelude::{GtkMenuExt as _, GtkMenuItemExt as _, MenuShellExt as _, WidgetExt as _};
    use gtk::{Menu, MenuItem, SeparatorMenuItem};

    /// Commands the File menu can dispatch to the workspace.
    #[derive(Clone, Debug)]
    pub enum FileAction {
        /// Choose one or more source images.
        OpenFiles,
        /// Choose a source directory.
        OpenFolder,
        /// Ask the workspace to open a recent source directory.
        OpenRecent(PathBuf),
        /// Clear the active profile's recent folders.
        ClearRecent,
        /// Request workspace close through its normal close workflow.
        Close,
    }

    /// Commands for the source beneath the pointer.
    #[derive(Clone, Copy, Debug)]
    pub enum SourceAction {
        /// Remove the selected source from the workspace.
        Remove,
        /// Reveal this source in the file manager.
        Reveal,
    }

    /// Actions within the native File menu, including its Recent section.
    #[derive(Clone, Debug)]
    enum FileChoice {
        /// Expand the recent-folders section of the menu.
        ShowRecent,
        /// Dispatch an application action after the menu closes.
        Dispatch(FileAction),
    }

    /// Display the real file menu until the user invokes a command or dismisses it.
    ///
    /// # Errors
    ///
    /// Returns GTK or display errors.
    pub fn show_file(recent: &[PathBuf]) -> Result<Option<FileAction>, String> {
        let first = file_popup(None)?;
        let selected = if matches!(first, Some(FileChoice::ShowRecent)) {
            file_popup(Some(recent))?
        } else {
            first
        };
        Ok(selected.and_then(|choice| match choice {
            FileChoice::Dispatch(action) => Some(action),
            FileChoice::ShowRecent => None,
        }))
    }

    /// Open the file menu and dispatch one native action when it closes.
    ///
    /// # Errors
    ///
    /// Returns an unavailable GTK desktop or pointer.
    pub fn open_file(
        recent: &[PathBuf],
        completion: impl FnOnce(Option<FileAction>) + 'static,
    ) -> Result<(), String> {
        let initial = build_file_popup(None)?;
        let expanded = build_file_popup(Some(recent))?;
        let anchor = Anchor::pointer()?;
        initial.present(&anchor, {
            let anchor = anchor.clone();
            move |choice| match choice {
                Some(FileChoice::ShowRecent) => {
                    expanded.present(&anchor, move |expanded_choice| {
                        completion(expanded_choice.and_then(workspace_action));
                    });
                }
                other => completion(other.and_then(workspace_action)),
            }
        });
        Ok(())
    }

    /// Convert a completed native menu action to a workspace operation.
    fn workspace_action(choice: FileChoice) -> Option<FileAction> {
        match choice {
            FileChoice::Dispatch(action) => Some(action),
            FileChoice::ShowRecent => None,
        }
    }

    /// Open a native File popup, with or without the expanded recent entries.
    ///
    /// GTK classifies attached submenu owners as menu containers. An action
    /// that expands entries on demand retains the real, operable menu-item role.
    ///
    /// # Errors
    ///
    /// Returns GTK or display errors.
    fn file_popup(recent: Option<&[PathBuf]>) -> Result<Option<FileChoice>, String> {
        build_file_popup(recent)?.show()
    }

    /// Build actual GTK items for the desired level of the File menu.
    ///
    /// # Errors
    ///
    /// Returns a GTK initialization error.
    fn build_file_popup(recent: Option<&[PathBuf]>) -> Result<Popup<FileChoice>, String> {
        let popup = Popup::new()?;
        popup.action(
            "Open Images…",
            "import.files",
            FileChoice::Dispatch(FileAction::OpenFiles),
        );
        popup.action(
            "Open Folder…",
            "import.folder",
            FileChoice::Dispatch(FileAction::OpenFolder),
        );
        if let Some(entries) = recent {
            popup.menu.append(&SeparatorMenuItem::new());
            if entries.is_empty() {
                let empty = item("No Recent Folders", "recent.empty");
                empty.set_sensitive(false);
                popup.menu.append(&empty);
            } else {
                for path in entries {
                    let title = folder_label(entries, path);
                    popup.action(
                        &title,
                        "",
                        FileChoice::Dispatch(FileAction::OpenRecent(path.clone())),
                    );
                }
                popup.action(
                    "Clear Menu",
                    "recent.clear",
                    FileChoice::Dispatch(FileAction::ClearRecent),
                );
            }
        } else {
            popup.action("Open Recent", "recent.open", FileChoice::ShowRecent);
        }
        popup.menu.append(&SeparatorMenuItem::new());
        popup.action(
            "Close Window",
            "window.close",
            FileChoice::Dispatch(FileAction::Close),
        );
        Ok(popup)
    }

    /// Use a full path when two recent folders share the same leaf name.
    fn folder_label<'a>(
        entries: &[PathBuf],
        path: &'a std::path::Path,
    ) -> std::borrow::Cow<'a, str> {
        let name = path.file_name();
        let duplicated = entries
            .iter()
            .any(|other| other != path && other.file_name() == name);
        if duplicated {
            path.to_string_lossy()
        } else {
            name.unwrap_or(path.as_os_str()).to_string_lossy()
        }
    }

    /// Display the real source context menu.
    ///
    /// # Errors
    ///
    /// Returns GTK or display errors.
    pub fn show_source() -> Result<Option<SourceAction>, String> {
        build_source_popup()?.show()
    }

    /// Open the source context menu and deliver one selected action.
    ///
    /// # Errors
    ///
    /// Returns an unavailable GTK desktop or pointer.
    pub fn open_source(
        completion: impl FnOnce(Option<SourceAction>) + 'static,
    ) -> Result<(), String> {
        let popup = build_source_popup()?;
        popup.present(&Anchor::pointer()?, completion);
        Ok(())
    }

    /// Build the actual action rows for one selected source.
    ///
    /// # Errors
    ///
    /// Returns a GTK initialization error.
    fn build_source_popup() -> Result<Popup<SourceAction>, String> {
        let popup = Popup::new()?;
        popup.action(
            "Show in File Manager",
            "frames.reveal",
            SourceAction::Reveal,
        );
        popup.action("Remove Source", "frames.remove_one", SourceAction::Remove);
        Ok(popup)
    }

    /// The native pointer location and GDK context used to place a menu.
    #[derive(Clone)]
    struct Anchor {
        /// Root native window whose screen coordinates the rectangle uses.
        root: gdk::Window,
        /// Pointer position in native screen coordinates.
        rect: gdk::Rectangle,
        /// GDK device context for the native popup grab.
        context: gdk::Event,
    }

    impl Anchor {
        /// Find the pointer on the current GTK desktop.
        ///
        /// # Errors
        ///
        /// Returns an unavailable native display or pointer.
        fn pointer() -> Result<Self, String> {
            let pointer = gdk::Display::default()
                .and_then(|display| display.default_seat())
                .and_then(|seat| seat.pointer())
                .ok_or("Could not locate the desktop pointer.")?;
            let (screen, x, y) = pointer.position();
            let root = screen
                .root_window()
                .ok_or("Could not locate the desktop window.")?;
            let mut context = gdk::Event::new(gdk::EventType::ButtonRelease);
            context.set_device(Some(&pointer));
            context.set_screen(Some(&screen));
            Ok(Self {
                root,
                rect: gdk::Rectangle::new(x, y, 1, 1),
                context,
            })
        }

        /// Present a real native menu at the pointer through GTK.
        fn popup(&self, menu: &Menu) {
            menu.show_all();
            menu.popup_at_rect(
                &self.root,
                &self.rect,
                gdk::Gravity::SouthWest,
                gdk::Gravity::NorthWest,
                Some(&self.context),
            );
        }
    }

    /// A native GTK menu and the single action selected in its local event loop.
    struct Popup<Action> {
        /// Menu that owns the visible GTK items.
        menu: Menu,
        /// Application command chosen by one real GTK activation.
        action: Rc<RefCell<Option<Action>>>,
    }

    impl<Action: Clone + 'static> Popup<Action> {
        /// Build a desktop menu on the current UI thread.
        ///
        /// # Errors
        ///
        /// Returns an unavailable GTK desktop error.
        fn new() -> Result<Self, String> {
            gtk::init().map_err(|error| format!("Could not initialize desktop menu: {error}"))?;
            Ok(Self {
                menu: Menu::new(),
                action: Rc::new(RefCell::new(None)),
            })
        }

        /// Add a real action item to this menu or one of its submenus.
        fn action(&self, label: &str, id: &str, action: Action) {
            let row = item(label, id);
            let result = Rc::clone(&self.action);
            let _activation = row.connect_activate(move |_| {
                *result.borrow_mut() = Some(action.clone());
            });
            self.menu.append(&row);
        }

        /// Anchor the menu at the real pointer and return after GTK closes it.
        ///
        /// # Errors
        ///
        /// Returns an unavailable display or pointer.
        fn show(self) -> Result<Option<Action>, String> {
            let anchor = Anchor::pointer()?;
            let main_loop = MainLoop::new(None, false);
            let end = main_loop.clone();
            let _deactivated = self.menu.connect_deactivate(move |_| end.quit());
            anchor.popup(&self.menu);
            if self.menu.is_mapped() {
                main_loop.run();
            }
            let chosen = self.action.borrow_mut().take();
            Ok(chosen)
        }

        /// Present the popup without entering a nested GTK main loop.
        fn present(self, anchor: &Anchor, completion: impl FnOnce(Option<Action>) + 'static) {
            // Keep the native widget alive through deactivation, then release
            // the reference once after the final activation event has finished.
            let pending = Rc::new(RefCell::new(Some((self.menu.clone(), completion))));
            let release = Rc::clone(&pending);
            let selected = Rc::clone(&self.action);
            let _handler = self.menu.connect_deactivate(move |_| {
                if let Some((menu, complete)) = release.borrow_mut().take() {
                    let selection = Rc::clone(&selected);
                    let _source = idle_add_local_once(move || {
                        let chosen = selection.borrow_mut().take();
                        drop(menu);
                        complete(chosen);
                    });
                }
            });
            anchor.popup(&self.menu);
            if !self.menu.is_mapped()
                && let Some((_, complete)) = pending.borrow_mut().take()
            {
                let _source = idle_add_local_once(move || complete(None));
            }
        }
    }

    /// Give the native menu row a stable accessibility ID and a readable label.
    fn item(label: &str, id: &str) -> MenuItem {
        let row = MenuItem::with_label(label);
        if !id.is_empty()
            && let Some(accessible) = row.accessible()
        {
            accessible.set_accessible_id(id);
        }
        row
    }
}
