//! Device-resident Gaussian reduction and exact-grid Laplacian reconstruction.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

use super::super::buffers::{ImageShape, elementwise, empty};

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

/// Apply separable binomial `[1,4,6,4,1]` weights and decimate on the even grid.
///
/// # Errors
/// Returns an error if intermediate sample counts exceed host addressing.
pub(super) fn reduce(
    client: &Client,
    input: &TensorHandle,
    fine: ImageShape,
    coarse: ImageShape,
    channels: usize,
) -> Result<TensorHandle, String> {
    let horizontal_samples = coarse
        .width
        .checked_mul(fine.height)
        .and_then(|pixels| pixels.checked_mul(channels))
        .ok_or("pyramid horizontal sample count overflow")?;
    let reduced_samples = coarse
        .area
        .checked_mul(channels)
        .ok_or("pyramid reduced sample count overflow")?;
    let horizontal = empty(client, horizontal_samples);
    let reduced = empty(client, reduced_samples);
    let (blocks, threads) = elementwise(client, horizontal_samples);
    half_axis::launch(
        client,
        blocks,
        threads,
        input.clone().into_arg(),
        horizontal.clone().into_arg(),
        fine.width,
        fine.height,
        coarse.width,
        channels,
        1,
    );
    let (blocks, threads) = elementwise(client, reduced_samples);
    half_axis::launch(
        client,
        blocks,
        threads,
        horizontal.into_arg(),
        reduced.clone().into_arg(),
        coarse.width,
        fine.height,
        coarse.width,
        channels,
        0,
    );
    Ok(reduced)
}

/// Preserve constants exactly by filtering deviations from the central sample.
#[cube(launch, expand_only)]
fn half_axis(
    input: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    output_width: usize,
    channels: usize,
    horizontal: u32,
) {
    let sample = ABSOLUTE_POS;
    if sample >= output.len() {
        terminate!();
    }
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
    output[sample] = origin + (far + near * 4.0) * 0.0625;
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
