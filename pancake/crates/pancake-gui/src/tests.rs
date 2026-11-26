//! Render the embedded workspace through Slint's pinned software renderer.

use std::error::Error;
use std::fs;
use std::path::Path;
use std::rc::{Rc, Weak};
use std::sync::Mutex;
use std::thread;

use num_traits::ToPrimitive as _;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{
    Key, Platform, PlatformError, PointerEventButton, WindowAdapter, WindowEvent,
};
use slint::{
    Color, ComponentHandle as _, Image, LogicalPosition, PhysicalSize, Rgb8Pixel, SharedPixelBuffer,
};
use slint_interpreter::{ComponentInstance, Value};

use super::preferences::{Accent, Preferences, Store, Theme};

/// Own access to system fonts for the lifetime of one offscreen GUI thread.
static OFFSCREEN_UI: Mutex<()> = Mutex::new(());

/// Run one offscreen workspace with its own Slint context.
///
/// Keep font discovery, use, and thread-local destruction inside the lock.
///
/// # Errors
/// Returns a thread setup, synchronization, or workspace test error.
fn run_offscreen_test(exercise: fn() -> Result<(), Box<dyn Error>>) -> Result<(), Box<dyn Error>> {
    let _ownership = OFFSCREEN_UI.lock()?;
    let owner = thread::Builder::new()
        .name("pancake-offscreen-ui".into())
        .spawn(move || exercise().map_err(|error| error.to_string()))?;
    owner
        .join()
        .map_err(|_| "The offscreen GUI test thread panicked.")??;
    Ok(())
}

/// An offscreen window using the same renderer as the software desktop backend.
struct Offscreen(Weak<MinimalSoftwareWindow>);

impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let adapter = self.0.upgrade().ok_or_else(|| {
            PlatformError::Other("The offscreen window is no longer available.".into())
        })?;
        Ok(adapter)
    }
}

/// The embedded workspace's preferred width.
const WIDTH: usize = 1_170;
/// The embedded workspace's preferred height.
const HEIGHT: usize = 730;
/// A color distinct from the source and the toolkit chrome.
const RESULT: Rgb8Pixel = Rgb8Pixel {
    r: 237,
    g: 19,
    b: 63,
};
/// A color distinct from the result and the toolkit chrome.
const SOURCE: Rgb8Pixel = Rgb8Pixel {
    r: 13,
    g: 229,
    b: 91,
};

/// The image spans the canvas horizontally at fit zoom.
fn solid(color: Rgb8Pixel) -> Image {
    let mut buffer = SharedPixelBuffer::new(320, 40);
    buffer.make_mut_slice().fill(color);
    Image::from_rgb8(buffer)
}

/// Rasterize actual output pixels from the production component.
///
/// # Errors
///
/// Returns an error if the renderer did not draw the workspace.
fn render(window: &MinimalSoftwareWindow) -> Result<Vec<Rgb8Pixel>, &'static str> {
    let mut buffer = vec![Rgb8Pixel::default(); WIDTH * HEIGHT];
    let drawn = window.draw_if_needed(|renderer| {
        let _damage = renderer.render(&mut buffer, WIDTH);
    });
    drawn
        .then_some(buffer)
        .ok_or("The workspace did not redraw.")
}

/// Compare one selected viewport pixel against its expected image color.
///
/// # Errors
///
/// Returns a coordinate or color mismatch with its viewport context.
fn check_pixel(
    pixels: &[Rgb8Pixel],
    x: usize,
    y: usize,
    expected: Rgb8Pixel,
    context: &str,
) -> Result<(), String> {
    let rows = pixels.as_chunks::<WIDTH>().0;
    let actual = rows
        .get(y)
        .and_then(|row| row.get(x))
        .copied()
        .ok_or("The comparison probe is outside the rendered window.")?;
    if actual != expected {
        return Err(format!(
            "{context} at ({x}, {y}): expected {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
}

/// The split covers the canvas from its left edge and moves with a drag.
///
/// # Errors
///
/// Returns window, input dispatch, or framebuffer observation errors.
#[test]
fn comparison_clips_source_to_left_of_split() -> Result<(), Box<dyn Error>> {
    run_offscreen_test(exercise_comparison)
}

/// Show the comparison workspace, check its pixels, and verify its release.
/// # Errors
/// Returns a window, input dispatch, or framebuffer observation error.
fn exercise_comparison() -> Result<(), Box<dyn Error>> {
    let adapter = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    // The window holds the Slint context, which owns this platform.
    let platform_window = Rc::downgrade(&adapter);
    slint::platform::set_platform(Box::new(Offscreen(platform_window.clone())))?;
    adapter.set_size(PhysicalSize::new(
        u32::try_from(WIDTH)?,
        u32::try_from(HEIGHT)?,
    ));
    super::branding::register_fonts()?;
    let workspace = super::compile_workspace()?.create()?;
    workspace.set_property("native-menu-bar", false.into())?;
    workspace.set_property("preview", solid(RESULT).into())?;
    workspace.set_property("comparison-preview", solid(SOURCE).into())?;
    workspace.set_property("preview-width", 320_i32.into())?;
    workspace.set_property("preview-height", 40_i32.into())?;
    let component = workspace.as_weak();
    let observed = workspace
        .show()
        .map_err(Box::<dyn Error>::from)
        .and_then(|()| observe_comparison(&workspace, &adapter));
    // The offscreen fixture owns the window lifecycle. Slint retains a strong
    // component reference while the window is visible.
    let hidden = workspace.hide().map_err(Box::<dyn Error>::from);
    drop(workspace);
    drop(adapter);
    observed.and(hidden)?;
    if component.upgrade().is_some() {
        return Err("The closed comparison workspace is still retained.".into());
    }
    if platform_window.upgrade().is_some() {
        return Err("The closed offscreen window is still retained.".into());
    }
    Ok(())
}

/// Observe the initial split and its response to native pointer events.
/// # Errors
/// Returns an input dispatch or framebuffer observation error.
fn observe_comparison(
    workspace: &ComponentInstance,
    adapter: &MinimalSoftwareWindow,
) -> Result<(), Box<dyn Error>> {
    let before = render(adapter)?;
    let rows = before.as_chunks::<WIDTH>().0;
    let (top, row) = rows
        .iter()
        .enumerate()
        .find(|(_y, row)| row.contains(&RESULT))
        .ok_or("The result was not visible before comparison.")?;
    let y = top.saturating_add(6);
    let left = row
        .iter()
        .position(|&pixel| pixel == RESULT)
        .ok_or("Missing left edge.")?;
    let right = row
        .iter()
        .rposition(|&pixel| pixel == RESULT)
        .ok_or("Missing right edge.")?;
    let span = right.saturating_sub(left);
    if span <= 100 {
        return Err("The baseline result must occupy the canvas.".into());
    }
    let left_probe = left.saturating_add(span / 10);
    let middle_probe = right.saturating_sub(span.saturating_mul(3) / 8);
    let right_probe = right.saturating_sub(span / 10);

    workspace.set_property("compare", true.into())?;
    let centered = render(adapter)?;
    check_pixel(
        &centered,
        left_probe,
        y,
        SOURCE,
        "the default split must show the source at the left edge",
    )?;
    check_pixel(
        &centered,
        middle_probe,
        y,
        RESULT,
        "the default split must preserve the result to the right of halfway",
    )?;

    let pointer = |x: usize| -> Result<LogicalPosition, &'static str> {
        let coordinates = x
            .to_f32()
            .zip(y.to_f32())
            .ok_or("Invalid pointer coordinate.")?;
        Ok(LogicalPosition::new(coordinates.0, coordinates.1))
    };
    let start = pointer(left.saturating_add(span / 2))?;
    let end = pointer(right.saturating_sub(span / 4))?;
    let window = workspace.window();
    for event in [
        WindowEvent::PointerMoved { position: start },
        WindowEvent::PointerPressed {
            position: start,
            button: PointerEventButton::Left,
        },
        WindowEvent::PointerMoved { position: end },
        WindowEvent::PointerReleased {
            position: end,
            button: PointerEventButton::Left,
        },
    ] {
        let _delivered = window.dispatch_event_with_result(event)?;
    }
    let dragged = render(adapter)?;
    check_pixel(
        &dragged,
        left_probe,
        y,
        SOURCE,
        "dragging must keep the source at the left edge",
    )?;
    check_pixel(
        &dragged,
        middle_probe,
        y,
        SOURCE,
        "dragging must reveal the source between the old and new split",
    )?;
    check_pixel(
        &dragged,
        right_probe,
        y,
        RESULT,
        "the result beyond the dragged split must stay visible",
    )?;
    Ok(())
}

/// Observe the real fit command without letting button decoration schedule more frames.
/// # Errors
/// Returns a window, input, or rendered-image regression.
#[test]
fn viewer_fit_updates_without_decorative_animation() -> Result<(), Box<dyn Error>> {
    run_offscreen_test(exercise_viewer_fit)
}

/// Observe the fit command and verify that its workspace is released.
/// # Errors
/// Returns a window, input, or rendered-image regression.
fn exercise_viewer_fit() -> Result<(), Box<dyn Error>> {
    let adapter = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let platform_window = Rc::downgrade(&adapter);
    slint::platform::set_platform(Box::new(Offscreen(platform_window.clone())))?;
    adapter.set_size(PhysicalSize::new(
        u32::try_from(WIDTH)?,
        u32::try_from(HEIGHT)?,
    ));
    super::branding::register_fonts()?;
    let workspace = super::compile_workspace()?.create()?;
    workspace.set_property("native-menu-bar", false.into())?;
    let mut portrait = SharedPixelBuffer::new(40, 320);
    portrait.make_mut_slice().fill(RESULT);
    workspace.set_property("preview", Image::from_rgb8(portrait).into())?;
    workspace.set_property("preview-width", 40_i32.into())?;
    workspace.set_property("preview-height", 320_i32.into())?;
    let component = workspace.as_weak();
    let observed = workspace
        .show()
        .map_err(Box::<dyn Error>::from)
        .and_then(|()| observe_fit_command(&workspace, &adapter));
    let hidden = workspace.hide().map_err(Box::<dyn Error>::from);
    drop(workspace);
    drop(adapter);
    observed.and(hidden)?;
    if component.upgrade().is_some() || platform_window.upgrade().is_some() {
        return Err("The finished fit command retained its offscreen workspace.".into());
    }
    Ok(())
}

/// Recover the toolbar position from the photograph's actual rendered bounds.
/// # Errors
/// Returns rendered geometry, input dispatch, or view behavior errors.
fn observe_fit_command(
    workspace: &ComponentInstance,
    adapter: &MinimalSoftwareWindow,
) -> Result<(), Box<dyn Error>> {
    let original = render(adapter)?;
    let rows = original.as_chunks::<WIDTH>().0;
    let bottom = rows
        .iter()
        .rposition(|row| row.contains(&RESULT))
        .ok_or("The fit image did not reach its canvas edge.")?;
    let y = bottom / 2;
    let middle = rows.get(y).ok_or("Missing image row.")?;
    let canvas = Rgb8Pixel::new(25, 25, 25);
    let left = middle
        .iter()
        .position(|&pixel| pixel == canvas)
        .ok_or("Missing canvas background.")?;
    let position = |x: usize, y: usize| -> Result<LogicalPosition, &'static str> {
        let (x, y) = x
            .to_f32()
            .zip(y.to_f32())
            .ok_or("Invalid pointer position.")?;
        Ok(LogicalPosition::new(x, y))
    };
    let window = workspace.window();
    let _scrolled = window.dispatch_event_with_result(WindowEvent::PointerScrolled {
        position: position(left.saturating_add(32), y)?,
        delta_x: 0.0,
        delta_y: 1.0,
    })?;
    let zoomed = render(adapter).map_err(|error| format!("After viewer scroll: {error}"))?;
    let photo_pixels =
        |picture: &[Rgb8Pixel]| picture.iter().filter(|&&pixel| pixel == RESULT).count();
    if photo_pixels(&zoomed) <= photo_pixels(&original) {
        return Err("Scrolling did not change the real displayed photograph.".into());
    }

    // Fit is the first outlined control in the right-hand toolbar group.
    let toolbar_y = 22;
    let field_edge = Rgb8Pixel::new(127, 127, 127);
    let toolbar = rows.get(toolbar_y).ok_or("Missing workspace toolbar.")?;
    let first_field = toolbar
        .iter()
        .enumerate()
        .skip(WIDTH / 2)
        .find(|&(_, &pixel)| pixel == field_edge)
        .map(|(index, _)| index)
        .ok_or("Missing the toolbar's first viewer control.")?;
    let fit = position(first_field.saturating_add(14), toolbar_y)?;
    let _hover = window.dispatch_event_with_result(WindowEvent::PointerMoved { position: fit })?;
    let _hovered = render(adapter).map_err(|error| format!("After Fit hover: {error}"))?;
    if window.has_active_animations() {
        return Err("Hovering on Fit scheduled a decorative animation.".into());
    }
    let _press = window.dispatch_event_with_result(WindowEvent::PointerPressed {
        position: fit,
        button: PointerEventButton::Left,
    })?;
    let _release = window.dispatch_event_with_result(WindowEvent::PointerReleased {
        position: fit,
        button: PointerEventButton::Left,
    })?;
    let fitted = render(adapter).map_err(|error| format!("After Fit click: {error}"))?;
    if photo_pixels(&fitted) != photo_pixels(&original) || window.has_active_animations() {
        return Err(
            "A Fit click must restore the photograph with no trailing skin animation.".into(),
        );
    }

    // Fit returns focus to the canvas. Reverse tab order reaches its zoom toolbar.
    for key in [
        Key::Backtab,
        Key::Backtab,
        Key::Backtab,
        Key::Backtab,
        Key::Backtab,
        Key::Return,
    ] {
        let _pressed =
            window.dispatch_event_with_result(WindowEvent::KeyPressed { text: key.into() })?;
        let _released =
            window.dispatch_event_with_result(WindowEvent::KeyReleased { text: key.into() })?;
    }
    let actual =
        render(adapter).map_err(|error| format!("After toolbar keyboard access: {error}"))?;
    if !(12_000..=13_600).contains(&photo_pixels(&actual)) || window.has_active_animations() {
        return Err(
            "Keyboard activation must show the 40 × 320 photograph at 1:1 without animation."
                .into(),
        );
    }
    Ok(())
}

/// Read the actual status displayed by the application's preference controls.
///
/// # Errors
/// Returns the binding error if the workspace has no string status.
fn preference_error(workspace: &ComponentInstance) -> Result<slint::SharedString, Box<dyn Error>> {
    let Value::String(message) = workspace.get_property("error-message")? else {
        return Err("The workspace preference status was not a string.".into());
    };
    Ok(message)
}

/// Report invalid profile data and allow the same window to save after repair.
///
/// # Errors
/// Returns a fixture, binding, persistence, or observed state error.
#[test]
fn preference_callback_reports_storage_errors_and_recovers() -> Result<(), Box<dyn Error>> {
    run_offscreen_test(exercise_preference_recovery)
}

/// Exercise profile recovery through the actual dialog callbacks.
/// # Errors
/// Returns a fixture, binding, persistence, or observed state error.
fn exercise_preference_recovery() -> Result<(), Box<dyn Error>> {
    let sandbox = tempfile::tempdir()?;
    let profile = sandbox.path().to_path_buf();
    let destination = profile.join("preferences.json");
    let malformed = b"{broken";
    fs::write(&destination, malformed)?;

    let adapter = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let platform_window = Rc::downgrade(&adapter);
    slint::platform::set_platform(Box::new(Offscreen(platform_window.clone())))?;
    super::branding::register_fonts()?;
    let workspace = super::compile_workspace()?.create()?;
    super::configure_preferences(&workspace, Store::new(profile.clone()))?;
    if !preference_error(&workspace)?.starts_with("Could not read preferences:")
        || workspace.get_property("theme-index")? != Value::from(1_i32)
        || workspace.get_property("device-index")? != Value::from(0_i32)
        || fs::read(&destination)? != malformed
    {
        return Err("An invalid profile must keep its bytes and display default choices.".into());
    }

    fs::remove_file(&destination)?;
    fs::create_dir(&destination)?;
    let protected = destination.join("other data");
    fs::write(&protected, b"preserved")?;
    let choices = [Value::String("Dark".into()), Value::String("CPU".into())];
    let _failed_save = workspace.invoke("save-preferences", &choices)?;
    if !preference_error(&workspace)?.starts_with("Could not save preferences:")
        || fs::read(&protected)? != b"preserved"
    {
        return Err("The failed save must report its error and preserve existing data.".into());
    }

    fs::remove_file(protected)?;
    fs::remove_dir(destination)?;
    let _saved = workspace.invoke("save-preferences", &choices)?;
    if !preference_error(&workspace)?.is_empty()
        || Store::new(profile.clone()).read()? != Preferences::from_choices("Dark", "CPU")?
    {
        return Err(
            "A successful retry must save the choices and clear the previous error.".into(),
        );
    }

    observe_accent_preferences(&workspace, &Store::new(profile))?;

    let component = workspace.as_weak();
    drop(workspace);
    drop(adapter);
    if component.upgrade().is_some() || platform_window.upgrade().is_some() {
        return Err(
            "The repaired profile window retained its callback or platform adapter.".into(),
        );
    }
    Ok(())
}

/// The real preference callbacks keep an accent across theme changes.
/// # Errors
/// Returns a persistence, color or status binding error.
fn observe_accent_preferences(
    workspace: &ComponentInstance,
    store: &Store,
) -> Result<(), Box<dyn Error>> {
    let white = Value::from(Color::from_rgb_u8(255, 255, 255));
    if workspace.get_property("accent-surface")? != white {
        return Err("The initial graphite workspace must use a white accent.".into());
    }
    let _invalid = workspace.invoke("save-accent", &[Value::String("#bad".into())])?;
    if !preference_error(workspace)?.ends_with("Enter an accent as #RRGGBB.")
        || store.read()?.accent != Accent::default()
        || workspace.get_property("accent-surface")? != white
    {
        return Err("A mistyped accent must preserve the applied and persisted colors.".into());
    }

    let accent = Accent::parse("#061329")?;
    let _accepted = workspace.invoke("save-accent", &[Value::String(accent.hex().into())])?;
    if store.read()?.accent != accent || !preference_error(workspace)?.is_empty() {
        return Err("The committed accent must survive a preference read.".into());
    }
    let dark = workspace.get_property("accent-surface")?;
    let _changed_theme = workspace.invoke(
        "save-preferences",
        &[Value::String("Light".into()), Value::String("CPU".into())],
    )?;
    let ([red, green, blue], _) = accent.presentation(Theme::Light);
    let light = workspace.get_property("accent-surface")?;
    if light == dark
        || light != Value::from(Color::from_rgb_u8(red, green, blue))
        || store.read()?.accent != accent
    {
        return Err(
            "Theme changes must resolve contrast without replacing the requested accent.".into(),
        );
    }
    Ok(())
}

/// The installed plug-in owns its scripts and points to the app bundle.
/// # Errors
/// Returns filesystem errors or a mismatch in the installed payload.
#[test]
fn lightroom_install_can_repair_an_installed_copy() -> Result<(), Box<dyn Error>> {
    let sandbox = tempfile::tempdir()?;
    let app = sandbox.path().join("Applications/Pancake's Copy.app");
    let executable = app.join("Contents/MacOS/pancake-gui");
    fs::create_dir_all(executable.parent().ok_or("Missing executable parent")?)?;
    fs::write(app.join("Contents/Info.plist"), b"bundle")?;
    fs::write(&executable, b"executable")?;
    let modules = sandbox.path().join("Adobe/Lightroom/Modules");
    let plugin = modules.join("Pancake.lrplugin");
    let install = super::lightroom::Installation::new(&executable, &modules)?;
    if install.is_current() {
        return Err("The plug-in should be absent before installation".into());
    }
    install.write()?;
    if !install.is_current() || plugin.join("Info.lua").symlink_metadata()?.is_symlink() {
        return Err("Lightroom needs a real copy of the complete plug-in".into());
    }
    let connection = fs::read_to_string(plugin.join("Connection.txt"))?;
    if Path::new(&connection) != app.canonicalize()? {
        return Err("The plug-in must point to the installed app bundle".into());
    }
    let presets = plugin.join("presets");
    let edited = presets.join("Pancake (With Lightroom Edits).lrtemplate");
    if !presets.join("Pancake (Originals).lrtemplate").is_file() || !edited.is_file() {
        return Err("Lightroom needs both installed export presets".into());
    }
    fs::remove_file(&edited)?;
    if install.is_current() {
        return Err("A missing preset requires repair".into());
    }
    install.write()?;
    if !install.is_current() || !edited.is_file() {
        return Err("Reinstall must restore the missing preset".into());
    }
    let loose = sandbox.path().join("pancake-gui");
    fs::write(&loose, b"executable")?;
    if super::lightroom::Installation::new(&loose, &modules).is_ok() {
        return Err("A loose build executable cannot install a Lightroom connection".into());
    }
    Ok(())
}
