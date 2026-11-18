//! File exports from full-precision results and actual rendered viewport pixels.

use std::error::Error;
use std::path::{Path, PathBuf};

use num_traits::ToPrimitive as _;
use pancake::config::BitDepth;
use pancake::image_io::{RgbImage, prepare_export};
use slint::ComponentHandle as _;
use slint_interpreter::{ComponentInstance, Value};

#[cfg(test)]
mod tests;

/// Prepare all files before publishing any replacement destination.
///
/// # Errors
///
/// Returns an encoding or publication error. A failed preparation publishes
/// no part of the batch, including when an existing output name is a directory.
pub(super) fn batch(
    directory: &Path,
    image: &RgbImage,
    sources: &[PathBuf],
    quality: u8,
) -> Result<(), String> {
    let pending = [
        ("Pancake.jpg", BitDepth::Eight),
        ("Pancake.png", BitDepth::Sixteen),
        ("Pancake.tif", BitDepth::Sixteen),
    ]
    .into_iter()
    .map(|(name, depth)| prepare_export(&directory.join(name), image, depth, quality, sources))
    .collect::<Result<Vec<_>, _>>()?;
    for output in pending {
        output.publish()?;
    }
    Ok(())
}

/// Copy the current viewer rectangle from the actual Slint renderer's output.
///
/// The window snapshot includes controls. The viewer sends its live geometry
/// in logical pixels, which is converted to the renderer's physical pixels.
///
/// # Errors
///
/// Returns a renderer, rectangle, shape or allocation error.
pub(super) fn viewport(
    window: &ComponentInstance,
    args: &[Value],
) -> Result<RgbImage, Box<dyn Error>> {
    let [
        Value::Number(x),
        Value::Number(y),
        Value::Number(width),
        Value::Number(height),
    ] = args
    else {
        return Err("The viewer did not provide a visible viewport rectangle.".into());
    };
    let scale = f64::from(window.window().scale_factor());
    let physical = |logical: f64| {
        logical
            .mul_add(scale, 0.0)
            .round()
            .to_usize()
            .ok_or("The viewport extends beyond the desktop render target.")
    };
    let [x, y, width, height] = [
        physical(*x)?,
        physical(*y)?,
        physical(*width)?,
        physical(*height)?,
    ];
    let snapshot = window.window().take_snapshot()?;
    let stride = usize::try_from(snapshot.width())?;
    let rows = usize::try_from(snapshot.height())?;
    let end_x = x.checked_add(width).ok_or("Viewport column overflow.")?;
    let end_y = y.checked_add(height).ok_or("Viewport row overflow.")?;
    if width == 0 || height == 0 || end_x > stride || end_y > rows {
        return Err("The viewport is outside the rendered window.".into());
    }

    let channels = width
        .checked_mul(height)
        .and_then(|area| area.checked_mul(3))
        .ok_or("Viewport sample count overflow.")?;
    let mut samples = Vec::new();
    samples.try_reserve_exact(channels)?;
    for row in snapshot
        .as_slice()
        .chunks_exact(stride)
        .skip(y)
        .take(height)
    {
        let pixels = row
            .get(x..end_x)
            .ok_or("The viewport scanline was incomplete.")?;
        for pixel in pixels {
            samples.extend([component(pixel.r), component(pixel.g), component(pixel.b)]);
        }
    }
    Ok(RgbImage::new(width, height, samples)?)
}

/// Convert eight-bit displayed color to a normalized export code value.
fn component(color: u8) -> f32 {
    f32::from(color).mul_add(1.0 / 255.0, 0.0)
}
