//! Render the embedded workspace through Slint's pinned software renderer.

use std::error::Error;
use std::rc::Rc;

use num_traits::ToPrimitive as _;
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, PlatformError, PointerEventButton, WindowAdapter, WindowEvent};
use slint::{
    ComponentHandle as _, Image, LogicalPosition, PhysicalSize, Rgb8Pixel, SharedPixelBuffer,
};

/// An offscreen window using the same renderer as the software desktop backend.
struct Offscreen(Rc<MinimalSoftwareWindow>);

impl Platform for Offscreen {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        Ok(self.0.clone())
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
    slint::platform::set_platform(Box::new(Offscreen(adapter.clone())))?;
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
    workspace.show()?;

    let before = render(&adapter)?;
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
    let centered = render(&adapter)?;
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
    let dragged = render(&adapter)?;
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
