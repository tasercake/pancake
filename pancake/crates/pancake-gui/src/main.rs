//! Native Pancake desktop application.

use std::error::Error;
use std::io::{self, Write as _};
use std::path::PathBuf;

use slint::{BackendSelector, ComponentHandle, SharedString};
use slint_interpreter::{Compiler, ComponentDefinition, ComponentInstance, Value};

use preferences::{Preferences, Store};

pub mod panel;
pub mod preferences;
pub mod preview;
mod session;
pub mod sources;

/// Repository-owned declarative presentation.
const WINDOW: &str = include_str!("workspace.slint");

/// Reports recoverable failures in the actual workspace.
pub(crate) fn show_error(window: &ComponentInstance, message: &str) {
    if let Err(error) = window.set_property("error-message", SharedString::from(message).into()) {
        let _written = writeln!(io::stderr().lock(), "{message}: {error}");
    }
}

/// Commit two actual choices to the active preference profile.
///
/// # Errors
///
/// Returns invalid-choice or profile-write errors.
fn save_preferences(store: &Store, args: &[Value]) -> Result<(), Box<dyn Error>> {
    let [Value::String(theme), Value::String(device)] = args else {
        return Err("Preferences panel supplied invalid values.".into());
    };
    store.save(Preferences::from_choices(theme.as_str(), device.as_str())?)
}

/// Apply stored settings to real selectors and persist later user changes.
///
/// # Errors
///
/// Returns errors for an unavailable profile directory or invalid UI bindings.
fn configure_preferences(window: &ComponentInstance) -> Result<(), Box<dyn Error>> {
    let store = Store::current()?;
    let preferences = match store.read() {
        Ok(preferences) => preferences,
        Err(error) => {
            show_error(window, &format!("Could not read preferences: {error}"));
            Preferences::default()
        }
    };
    window.set_property(
        "theme-index",
        Value::Number(f64::from(preferences.theme.index())),
    )?;
    window.set_property(
        "device-index",
        Value::Number(f64::from(preferences.device.index())),
    )?;
    let weak = window.as_weak();
    window.set_callback("save-preferences", move |args| {
        if let Some(window) = weak.upgrade() {
            match save_preferences(&store, args) {
                Ok(()) => show_error(&window, ""),
                Err(error) => show_error(&window, &format!("Could not save preferences: {error}")),
            }
        }
        Value::Void
    })?;
    Ok(())
}

/// Create the native window from the embedded, reviewed UI source.
///
/// # Errors
///
/// Returns renderer, UI compilation, or application startup errors.
fn main() -> Result<(), Box<dyn Error>> {
    BackendSelector::new()
        .backend_name("winit".into())
        .select()?;
    let window = compile_workspace()?.create()?;
    window.set_property(
        "app-version",
        SharedString::from(env!("CARGO_PKG_VERSION")).into(),
    )?;
    configure_preferences(&window)?;
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let _worker_poll = session::configure(&window, &paths, pancake::job::Options::from_env()?)?;
    window.run()?;
    Ok(())
}

/// Compile the pinned toolkit widgets and embedded workspace as one component.
///
/// # Errors
///
/// Returns the embedded source diagnostics if it cannot be compiled.
fn compile_workspace() -> Result<ComponentDefinition, String> {
    let mut compiler = Compiler::default();
    compiler.set_style("fluent".into());
    compiler.set_edition((1, 19));
    let built = spin_on::spin_on(compiler.build_from_source(WINDOW.into(), PathBuf::new()));
    built.component("Workspace").ok_or_else(|| {
        built
            .diagnostics()
            .map(|diagnostic| diagnostic.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })
}

/// Smoke-check the same declarative component compiled on application startup.
///
/// # Errors
///
/// Returns real parser and schema diagnostics.
#[test]
fn embedded_workspace_compiles() -> Result<(), String> {
    compile_workspace().map(drop)
}
