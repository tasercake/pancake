//! Spatial focus and exposure measures, independent of the launch runtime.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

use super::super::buffers::{ImageShape, empty};
use super::elementwise;
use super::pyramid::{clamp_after, clamp_before};

/// Produce focus weights or Mertens-style contrast and saturation weights.
///
/// Focus weights are spatially pooled over a separable local square. Exposure
/// weights are softened by the Gaussian pyramid rather than the focus window.
pub(super) fn weights(
    client: &Client,
    rgb: &TensorHandle,
    grid: ImageShape,
    radius: usize,
    exposure: bool,
) -> TensorHandle {
    let (blocks, threads) = elementwise(client, grid.area);
    let point = empty(client, grid.area);
    measure::launch(
        client,
        blocks.clone(),
        threads,
        rgb.clone().into_arg(),
        point.clone().into_arg(),
        grid.width,
        grid.height,
        u32::from(exposure),
    );
    if exposure {
        return point;
    }

    let horizontal = empty(client, grid.area);
    let spatial = empty(client, grid.area);
    box_filter::launch(
        client,
        blocks.clone(),
        threads,
        point.into_arg(),
        horizontal.clone().into_arg(),
        grid.width,
        grid.height,
        radius,
        1,
    );
    box_filter::launch(
        client,
        blocks,
        threads,
        horizontal.into_arg(),
        spatial.clone().into_arg(),
        grid.width,
        grid.height,
        radius,
        0,
    );
    spatial
}

/// Read Rec. 601 luminance from the RGB source order.
#[cube(expand_only)]
fn luma(rgb: &Tensor<f32>, pixel: usize) -> f32 {
    let address = 3 * pixel;
    rgb[address] * 0.299 + rgb[address + 1] * 0.587 + rgb[address + 2] * 0.114
}

/// Measure a modified Laplacian for focus, or contrast and saturation for exposure.
#[cube(launch, expand_only)]
fn measure(
    rgb: &Tensor<f32>,
    result: &mut Tensor<f32>,
    width: usize,
    height: usize,
    exposure: u32,
) {
    let pixel = ABSOLUTE_POS;
    if pixel >= result.len() {
        terminate!();
    }
    let x = pixel % width;
    let y = pixel / width;
    let center = luma(rgb, pixel);
    let left = luma(rgb, y * width + clamp_before(x, 1));
    let right = luma(rgb, y * width + clamp_after(x, 1, width - 1));
    let above = luma(rgb, clamp_before(y, 1) * width + x);
    let below = luma(rgb, clamp_after(y, 1, height - 1) * width + x);
    let dx = (center - left) + (center - right);
    let dy = (center - above) + (center - below);
    if exposure != 0 {
        let red = rgb[pixel * 3];
        let green = rgb[pixel * 3 + 1];
        let blue = rgb[pixel * 3 + 2];
        let contrast = f32::abs(dx + dy);
        let rg = red - green;
        let gb = green - blue;
        let br = blue - red;
        let saturation = f32::sqrt((rg * rg + gb * gb + br * br) / 3.0);
        let weight = if contrast >= 0.000_001 {
            contrast * saturation
        } else {
            0.0_f32
        };
        result[pixel] = weight + 0.000_000_000_001;
    } else {
        result[pixel] = f32::abs(dx) + f32::abs(dy);
    }
}

/// Average point responses along one axis without allocating a window per pixel.
#[cube(launch, expand_only)]
fn box_filter(
    before: &Tensor<f32>,
    after: &mut Tensor<f32>,
    width: usize,
    height: usize,
    radius: usize,
    horizontal: u32,
) {
    let pixel = ABSOLUTE_POS;
    if pixel >= after.len() {
        terminate!();
    }
    let center = if horizontal != 0 {
        pixel % width
    } else {
        pixel / width
    };
    let base = if horizontal != 0 {
        (pixel / width) * width
    } else {
        pixel % width
    };
    let stride = if horizontal != 0 {
        usize::cast_from(1_u32)
    } else {
        width
    };
    let last = if horizontal != 0 {
        width - 1
    } else {
        height - 1
    };
    let mut sum = before[pixel];
    for offset in 1..=radius {
        sum += before[base + clamp_before(center, offset) * stride];
        sum += before[base + clamp_after(center, offset, last) * stride];
    }
    after[pixel] = sum / f32::cast_from(2 * radius + 1);
}
