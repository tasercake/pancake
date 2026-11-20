//! Native Pancake desktop application.

use std::error::Error;
use std::io::{self, Write as _};
use std::path::PathBuf;

use slint::{BackendSelector, ComponentHandle, SharedString};
use slint_interpreter::{Compiler, ComponentDefinition, ComponentInstance, Value};

use preferences::{Preferences, Store};

pub mod delivery;
pub mod desktop_menu;
pub mod history;
#[cfg(any(target_os = "macos", test))]
pub mod lightroom;
pub mod panel;
pub mod preferences;
pub mod preview;
pub mod session;
pub mod sources;

#[cfg(test)]
mod tests;

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
fn configure_preferences(window: &ComponentInstance, store: Store) -> Result<(), Box<dyn Error>> {
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

/// Bind the Lightroom installer to the current macOS app bundle.
/// # Errors
/// Returns executable resolution or UI binding errors.
#[cfg(target_os = "macos")]
fn configure_lightroom(window: &ComponentInstance) -> Result<(), Box<dyn Error>> {
    let Some(home) = dirs::home_dir() else {
        window.set_property(
            "lightroom-status",
            SharedString::from("Could not find your Lightroom Modules folder.").into(),
        )?;
        return Ok(());
    };
    let modules = home.join("Library/Application Support/Adobe/Lightroom/Modules");
    let Ok(install) = lightroom::Installation::new(&std::env::current_exe()?, &modules) else {
        window.set_property(
            "lightroom-status",
            SharedString::from("Open the Pancake.app bundle to install the plug-in.").into(),
        )?;
        return Ok(());
    };
    let update_status =
        |window: &ComponentInstance, installed: bool| -> Result<(), Box<dyn Error>> {
            let status = if installed {
                "Plug-in installed. In Lightroom Classic, use File > Plug-in Extras."
            } else {
                "Install the plug-in to open selected Lightroom photos in Pancake."
            };
            window.set_property("lightroom-status", SharedString::from(status).into())?;
            window.set_property("lightroom-installed", installed.into())?;
            Ok(())
        };
    update_status(window, install.is_current())?;
    window.set_property("lightroom-installable", true.into())?;
    let weak = window.as_weak();
    window.set_callback("install-lightroom", move |_| {
        if let Some(window) = weak.upgrade() {
            match install
                .write()
                .map_err(Into::into)
                .and_then(|()| update_status(&window, true))
            {
                Ok(()) => show_error(&window, ""),
                Err(error) => show_error(
                    &window,
                    &format!("Could not install Lightroom plug-in: {error}"),
                ),
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
    let mut options = pancake::job::Options::from_env()?;
    configure_backend(&mut options)?;
    let window = compile_workspace()?.create()?;
    window.set_property("native-menu-bar", cfg!(not(target_os = "linux")).into())?;
    window.set_property(
        "app-version",
        SharedString::from(env!("CARGO_PKG_VERSION")).into(),
    )?;
    let store = Store::current()?;
    let profile = store.directory().to_path_buf();
    configure_preferences(&window, store)?;
    #[cfg(target_os = "macos")]
    configure_lightroom(&window)?;
    let mut args = std::env::args_os().skip(1).peekable();
    let edited_renders = args.next_if_eq("--lightroom-renders").is_some();
    let paths: Vec<PathBuf> = args.map(PathBuf::from).collect();
    let _worker_poll = session::configure(&window, &paths, &profile, options, !edited_renders)?;
    window.run()?;
    Ok(())
}

/// Share one physical wgpu device with Slint and the worker when available.
///
/// A user-selected Slint backend remains explicit. A desktop without suitable
/// GPU/API access uses software presentation, independently of the job engine.
///
/// # Errors
///
/// Returns failure initializing the selected presentation backend.
fn configure_backend(options: &mut pancake::job::Options) -> Result<(), Box<dyn Error>> {
    if std::env::var_os("SLINT_BACKEND").is_some() {
        BackendSelector::new().select()?;
        return Ok(());
    }
    if let Ok(runtime) = pancake::runtime::prepare_wgpu_runtime(None, options.working_bytes)
        && let Some(setup) = runtime.wgpu_setup()
    {
        BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("femtovg-wgpu".into())
            .require_wgpu_30(slint::wgpu_30::WGPUConfiguration::Manual {
                instance: setup.instance.clone(),
                adapter: setup.adapter.clone(),
                device: setup.device.clone(),
                queue: setup.queue.clone(),
            })
            .select()?;
        options.prepared_wgpu = Some(runtime);
    } else {
        BackendSelector::new()
            .backend_name("winit".into())
            .renderer_name("software".into())
            .select()?;
    }
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
    let built = spin_on::spin_on(compiler.build_from_source(
        WINDOW.into(),
        PathBuf::from("builtin:/pancake/workspace.slint"),
    ));
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
