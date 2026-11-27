//! Native Pancake Stacker desktop application.

use std::cell::Cell;
use std::error::Error;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::rc::Rc;

use slint::{BackendSelector, Color, ComponentHandle, SharedString};
use slint_interpreter::{Compiler, ComponentDefinition, ComponentInstance, Value};

use preferences::{Accent, Preferences, Store};

pub mod branding;
pub mod delivery;
pub mod desktop_menu;
pub mod filmstrip;
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

/// Select two native choices without changing the stored accent.
/// # Errors
/// Returns invalid-choice or callback data errors.
fn change_choices(current: Preferences, args: &[Value]) -> Result<Preferences, String> {
    let [Value::String(theme), Value::String(device)] = args else {
        return Err("Preferences panel supplied invalid values.".into());
    };
    let chosen = Preferences::from_choices(theme.as_str(), device.as_str())?;
    Ok(Preferences {
        accent: current.accent,
        ..chosen
    })
}

/// Update the requested accent without changing the stored theme or device.
/// # Errors
/// Returns an invalid accent or callback payload error.
fn change_accent(current: Preferences, args: &[Value]) -> Result<Preferences, String> {
    let [Value::String(accent)] = args else {
        return Err("Preferences panel supplied an invalid accent.".into());
    };
    Ok(Preferences {
        accent: Accent::parse(accent.as_str())?,
        ..current
    })
}

/// Publish the persisted choices and the colors resolved for their theme.
/// # Errors
/// Returns errors from native property bindings.
fn apply_preferences(
    window: &ComponentInstance,
    preferences: Preferences,
) -> Result<(), Box<dyn Error>> {
    window.set_property("theme-index", i32::from(preferences.theme.index()).into())?;
    window.set_property("device-index", i32::from(preferences.device.index()).into())?;
    window.set_property(
        "accent-value",
        SharedString::from(preferences.accent.hex()).into(),
    )?;
    let ([red, green, blue], [ink_red, ink_green, ink_blue]) =
        preferences.accent.presentation(preferences.theme);
    window.set_property(
        "accent-surface",
        Color::from_rgb_u8(red, green, blue).into(),
    )?;
    window.set_property(
        "accent-ink",
        Color::from_rgb_u8(ink_red, ink_green, ink_blue).into(),
    )?;
    Ok(())
}

/// Preference transformations start from the last committed state.
type ChangePreference = fn(Preferences, &[Value]) -> Result<Preferences, String>;

/// Attach one preference action, keeping in-memory state and disk in sync.
/// # Errors
/// Returns errors from the corresponding native callback binding.
fn bind_preference(
    window: &ComponentInstance,
    profile: &Rc<(Store, Cell<Preferences>)>,
    name: &str,
    change: ChangePreference,
) -> Result<(), Box<dyn Error>> {
    let weak = window.as_weak();
    let profile = Rc::clone(profile);
    window.set_callback(name, move |args| {
        if let Some(window) = weak.upgrade() {
            let (store, active) = profile.as_ref();
            let result = change(active.get(), args)
                .map_err(Box::<dyn Error>::from)
                .and_then(|preferences| {
                    store.save(preferences)?;
                    active.set(preferences);
                    apply_preferences(&window, preferences)
                });
            match result {
                Ok(()) => show_error(&window, ""),
                Err(error) => {
                    // A native selector can change locally before the commit is accepted.
                    let theme = i32::from(active.get().theme.index());
                    let device = i32::from(active.get().device.index());
                    let _theme_restored = window.set_property("theme-index", theme.into());
                    let _device_restored = window.set_property("device-index", device.into());
                    show_error(&window, &format!("Could not save preferences: {error}"));
                }
            }
        }
        Value::Void
    })?;
    Ok(())
}

/// Apply stored settings to real selectors and persist later user changes.
/// # Errors
/// Returns errors for unavailable profile directories or invalid UI bindings.
fn configure_preferences(window: &ComponentInstance, store: Store) -> Result<(), Box<dyn Error>> {
    let preferences = match store.read() {
        Ok(preferences) => preferences,
        Err(error) => {
            show_error(window, &format!("Could not read preferences: {error}"));
            Preferences::default()
        }
    };
    apply_preferences(window, preferences)?;
    let profile = Rc::new((store, Cell::new(preferences)));
    bind_preference(window, &profile, "save-preferences", change_choices)?;
    bind_preference(window, &profile, "save-accent", change_accent)
}

/// Bind the Lightroom installer to the running macOS executable.
/// # Errors
/// Returns UI binding errors.
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
    let install = match std::env::current_exe()
        .and_then(|executable| lightroom::Installation::new(&executable, &modules))
    {
        Ok(install) => install,
        Err(error) => {
            window.set_property(
                "lightroom-status",
                SharedString::from(format!(
                    "Could not locate the Pancake Stacker executable: {error}"
                ))
                .into(),
            )?;
            return Ok(());
        }
    };
    let update_status =
        |window: &ComponentInstance, installed: bool| -> Result<(), Box<dyn Error>> {
            let status = if installed {
                "Plug-in installed. In Lightroom Classic, right-click photos and choose Export."
            } else {
                "Install the plug-in to open selected Lightroom photos in Pancake Stacker."
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
    #[cfg(target_os = "linux")]
    slint::set_xdg_app_id("pancake")?;
    branding::register_fonts()?;
    let window = compile_workspace()?.create()?;
    window.set_property("native-menu-bar", cfg!(not(target_os = "linux")).into())?;
    window.set_property("app-icon", branding::app_icon()?.into())?;
    window.set_property("wordmark", branding::wordmark()?.into())?;
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
