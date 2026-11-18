//! Native source session and frame selection.

use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;

use pancake::image_io::decode;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use slint_interpreter::{ComponentInstance, Value};

use super::preview::Preview;
use super::show_error;
use super::sources::Sources;

/// One displayed workspace with ordered source paths.
#[derive(Debug, Default)]
struct Session {
    /// Imported paths in canonical processing order.
    sources: Sources,
}

impl Session {
    /// Render names from the actual source collection.
    ///
    /// # Errors
    ///
    /// Returns a Slint binding error.
    fn refresh_list(&self, window: &ComponentInstance) -> Result<(), Box<dyn Error>> {
        let names = self
            .sources
            .paths()
            .iter()
            .map(|path| {
                path.file_name().map_or_else(
                    || path.display().to_string().into(),
                    |name| SharedString::from(name.to_string_lossy().as_ref()),
                )
            })
            .collect::<Vec<_>>();
        let model = ModelRc::new(VecModel::from(names));
        window.set_property("frames", model.into())?;
        Ok(())
    }

    /// Decode a real source using the shared precise image IO, present only the
    /// bounded preview, and let the view retain its framing.
    ///
    /// # Errors
    ///
    /// Returns missing source, decode, geometry, or dynamic UI binding errors.
    fn show_source(&self, window: &ComponentInstance, index: usize) -> Result<(), Box<dyn Error>> {
        let path = self
            .sources
            .paths()
            .get(index)
            .ok_or("Source no longer exists.")?;
        let preview = Preview::new(&decode(path)?)?;
        window.set_property("preview", preview.picture.into())?;
        window.set_property("preview-width", Value::Number(f64::from(preview.width)))?;
        window.set_property("preview-height", Value::Number(f64::from(preview.height)))?;
        window.set_property("preview-x", 0_i32.into())?;
        window.set_property("preview-y", 0_i32.into())?;
        window.set_property("selected-result-method", SharedString::default().into())?;
        window.set_property("selected-source", i32::try_from(index)?.into())?;
        Ok(())
    }
}

/// Register real UI events and import the paths delivered to the application.
///
/// # Errors
///
/// Returns dynamic view binding or initialization errors.
pub(super) fn configure(
    window: &ComponentInstance,
    paths: &[PathBuf],
) -> Result<(), Box<dyn Error>> {
    let mut session = Session::default();
    if let Err(error) = session.sources.add(paths) {
        show_error(window, &error);
    }
    session.refresh_list(window)?;
    if !session.sources.paths().is_empty()
        && let Err(error) = session.show_source(window, 0)
    {
        show_error(window, &error.to_string());
    }

    let session = Rc::new(RefCell::new(session));
    let weak = window.as_weak();
    window.set_callback("select-source", move |args| {
        if let Some(window) = weak.upgrade()
            && let Err(error) = select_source(&window, &session.borrow(), args)
        {
            show_error(&window, &error.to_string());
        }
        Value::Void
    })?;
    Ok(())
}

/// Translate only a valid visible source ordinal to a real view change.
///
/// # Errors
///
/// Returns stale ordinal, decoding, or view binding errors.
fn select_source(
    window: &ComponentInstance,
    session: &Session,
    args: &[Value],
) -> Result<(), Box<dyn Error>> {
    let [value] = args else {
        return Err("Source selector supplied no index.".into());
    };
    let index = i32::try_from(value.clone()).map_err(|_| "Source index is not an integer.")?;
    session.show_source(window, usize::try_from(index)?)?;
    show_error(window, "");
    Ok(())
}
