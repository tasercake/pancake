//! Device-resident Gaussian reduction and exact-grid Laplacian reconstruction.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

use super::super::buffers::{ImageShape, empty};
use super::elementwise;

/// Include the original grid, retaining odd trailing rows and columns.
///
/// # Errors
/// Returns an error if a reduced grid is not addressable.
pub(super) fn grids(origin: ImageShape, reductions: usize) -> Result<Vec<ImageShape>, String> {
    let mut grids = vec![origin];
    let mut last = origin;
    for _ in 0..reductions {
        if last.width == 1 && last.height == 1 {
            break;
        }
        last = ImageShape::new([last.width.div_ceil(2), last.height.div_ceil(2)])?;
        grids.push(last);
    }
    Ok(grids)
}

/// Reduce the RGB and scalar planes together with separable binomial weights.
///
/// # Errors
/// Returns an error if intermediate geometry or launch addressing is invalid.
pub(super) fn reduce(
    client: &Client,
    image: &TensorHandle,
    weights: &TensorHandle,
    fine: ImageShape,
    coarse: ImageShape,
) -> Result<(TensorHandle, TensorHandle), String> {
    let intermediate = ImageShape::new([coarse.width, fine.height])?;
    let (horizontal_image, horizontal_weights) =
        half_pass(client, image, weights, fine, intermediate, true)?;
    half_pass(
        client,
        &horizontal_image,
        &horizontal_weights,
        intermediate,
        coarse,
        false,
    )
}

/// Dispatch one axis for both input planes without changing the sample order.
///
/// # Errors
/// Returns an error if the combined dispatch cannot use device indexing.
fn half_pass(
    client: &Client,
    image: &TensorHandle,
    weights: &TensorHandle,
    input: ImageShape,
    output: ImageShape,
    horizontal: bool,
) -> Result<(TensorHandle, TensorHandle), String> {
    let samples = output
        .samples
        .checked_add(output.area)
        .ok_or("pyramid work item count overflow")?;
    let _addressable = u32::try_from(samples).map_err(|_| "pyramid dispatch exceeds u32")?;
    let reduced_image = empty(client, output.samples);
    let reduced_weights = empty(client, output.area);
    let (blocks, threads) = elementwise(client, samples);
    half_axis::launch(
        client,
        blocks,
        threads,
        image.clone().into_arg(),
        weights.clone().into_arg(),
        reduced_image.clone().into_arg(),
        reduced_weights.clone().into_arg(),
        input.width,
        input.height,
        output.width,
        u32::from(horizontal),
    );
    Ok((reduced_image, reduced_weights))
}

/// Reduce independently indexed RGB and scalar buffers in a single dispatch.
#[cube(launch, expand_only)]
fn half_axis(
    image: &Tensor<f32>,
    weights: &Tensor<f32>,
    reduced_image: &mut Tensor<f32>,
    reduced_weights: &mut Tensor<f32>,
    width: usize,
    height: usize,
    output_width: usize,
    horizontal: u32,
) {
    let sample = ABSOLUTE_POS;
    if sample < reduced_image.len() {
        reduced_image[sample] =
            half_value(image, sample, width, height, output_width, 3, horizontal);
    } else {
        let weight = sample - reduced_image.len();
        if weight < reduced_weights.len() {
            reduced_weights[weight] =
                half_value(weights, weight, width, height, output_width, 1, horizontal);
        }
    }
}

/// Preserve constants exactly by filtering deviations from the central sample.
#[cube(expand_only)]
fn half_value(
    input: &Tensor<f32>,
    sample: usize,
    width: usize,
    height: usize,
    output_width: usize,
    channels: usize,
    horizontal: u32,
) -> f32 {
    let x = (sample / channels) % output_width;
    let y = (sample / channels) / output_width;
    let center = if horizontal != 0 { 2 * x } else { 2 * y };
    let limit = if horizontal != 0 {
        width - 1
    } else {
        height - 1
    };
    let base = if horizontal != 0 { y * width } else { x };
    let stride = if horizontal != 0 {
        usize::cast_from(1_u32)
    } else {
        width
    };
    let lane = sample % channels;
    let origin = input[(base + center * stride) * channels + lane];

    let lower_near = clamp_before(center, 1);
    let lower_far = clamp_before(center, 2);
    let upper_near = clamp_after(center, 1, limit);
    let upper_far = clamp_after(center, 2, limit);
    let far = (input[(base + lower_far * stride) * channels + lane] - origin)
        + (input[(base + upper_far * stride) * channels + lane] - origin);
    let near = (input[(base + lower_near * stride) * channels + lane] - origin)
        + (input[(base + upper_near * stride) * channels + lane] - origin);
    origin + (far + near * 4.0) * 0.0625
}

/// Neighbor at or before an index, clamped to the leading border.
#[cube(expand_only)]
pub(super) fn clamp_before(index: usize, distance: usize) -> usize {
    usize::max(index, distance) - distance
}

/// Neighbor at or after an index, clamped to the trailing border.
#[cube(expand_only)]
pub(super) fn clamp_after(index: usize, distance: usize, last: usize) -> usize {
    usize::min(index + distance, last)
}

/// Bilinearly sample a half-resolution RGB level on the matching even grid.
#[cube(expand_only)]
pub(super) fn expanded(
    coarse: &Tensor<f32>,
    x: usize,
    y: usize,
    channel: usize,
    coarse_width: usize,
    coarse_height: usize,
) -> f32 {
    let x0 = x / 2;
    let y0 = y / 2;
    let x1 = clamp_after(x0, 1, coarse_width - 1);
    let y1 = clamp_after(y0, 1, coarse_height - 1);
    let top_left = coarse[(y0 * coarse_width + x0) * 3 + channel];
    let top_right = coarse[(y0 * coarse_width + x1) * 3 + channel];
    let bottom_left = coarse[(y1 * coarse_width + x0) * 3 + channel];
    let bottom_right = coarse[(y1 * coarse_width + x1) * 3 + channel];
    let wx = f32::cast_from(x % 2) * 0.5;
    let wy = f32::cast_from(y % 2) * 0.5;
    let top = top_left + (top_right - top_left) * wx;
    let bottom = bottom_left + (bottom_right - bottom_left) * wx;
    top + (bottom - top) * wy
}

/// Expand the reconstructed coarse image and add one stored Laplacian band.
pub(super) fn reconstruct(
    client: &Client,
    band: &TensorHandle,
    coarse: TensorHandle,
    fine_grid: ImageShape,
    coarse_grid: ImageShape,
) -> TensorHandle {
    let output = empty(client, fine_grid.samples);
    let (blocks, threads) = elementwise(client, fine_grid.samples);
    restore::launch(
        client,
        blocks,
        threads,
        band.clone().into_arg(),
        coarse.into_arg(),
        output.clone().into_arg(),
        fine_grid.width,
        coarse_grid.width,
        coarse_grid.height,
    );
    output
}

/// Restore a multiscale band at its original pixel coordinates.
#[cube(launch, expand_only)]
fn restore(
    band: &Tensor<f32>,
    coarse: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    coarse_width: usize,
    coarse_height: usize,
) {
    let sample = ABSOLUTE_POS;
    if sample >= output.len() {
        terminate!();
    }
    let x = (sample / 3) % width;
    let y = (sample / 3) / width;
    output[sample] = band[sample] + expanded(coarse, x, y, sample % 3, coarse_width, coarse_height);
}
