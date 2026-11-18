//! Render the embedded workspace through Slint's pinned software renderer.

use std::error::Error;
use std::fs;
use std::rc::{Rc, Weak};

use num_traits::ToPrimitive as _;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, PointerEventButton, WindowAdapter, WindowEvent};
use slint::{
    ComponentHandle as _, Image, LogicalPosition, PhysicalSize, Rgb8Pixel, SharedPixelBuffer,
};
use slint_interpreter::{ComponentInstance, Value};

use super::preferences::{Preferences, Store};

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
    let adapter = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    // The window holds the Slint context, which owns this platform.
    let platform_window = Rc::downgrade(&adapter);
    slint::platform::set_platform(Box::new(Offscreen(platform_window.clone())))?;
    adapter.set_size(PhysicalSize::new(
        u32::try_from(WIDTH)?,
        u32::try_from(HEIGHT)?,
    ));
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
    let sandbox = tempfile::tempdir()?;
    let profile = sandbox.path().to_path_buf();
    let destination = profile.join("preferences.json");
    let malformed = b"{broken";
    fs::write(&destination, malformed)?;

    let adapter = MinimalSoftwareWindow::new(RepaintBufferType::NewBuffer);
    let platform_window = Rc::downgrade(&adapter);
    slint::platform::set_platform(Box::new(Offscreen(platform_window.clone())))?;
    let workspace = super::compile_workspace()?.create()?;
    super::configure_preferences(&workspace, Store::new(profile.clone()))?;
    if !preference_error(&workspace)?.starts_with("Could not read preferences:")
        || workspace.get_property("theme-index")? != Value::from(0_i32)
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
        || Store::new(profile).read()? != Preferences::from_choices("Dark", "CPU")?
    {
        return Err(
            "A successful retry must save the choices and clear the previous error.".into(),
        );
    }

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
