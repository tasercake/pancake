//! Shared separable resampling with pixel-center geometry and box decimation.

use cubecl::{prelude::*, std::tensor::TensorHandle};

use super::buffers::{ImageShape, elementwise, empty, read_image, upload};

/// Resize packed RGB samples to a new `[width, height]` grid.
///
/// Upsampling interpolates the nearest two centers, clamped at the border.
/// Downsampling integrates a box footprint rather than sampling only its center.
/// RGB order and normalized or HDR sample ranges survive resampling.
///
/// # Errors
/// Returns an error for invalid geometry, samples, budget or device readback.
pub fn resize(
    device: &cubecl::Device,
    source: [usize; 2],
    destination: [usize; 2],
    samples: &[f32],
    max_working_bytes: usize,
) -> Result<Vec<f32>, String> {
    let source = ImageShape::new(source)?;
    let destination = ImageShape::new(destination)?;
    source.validate(samples)?;
    let horizontal = ImageShape::new([destination.width, source.height])?;
    let budget_samples = source
        .samples
        .checked_mul(3)
        .and_then(|count| count.checked_add(horizontal.samples))
        .and_then(|count| {
            destination
                .samples
                .checked_mul(3)
                .and_then(|result| count.checked_add(result))
        })
        .ok_or("resize sample count exceeds host addressing")?;
    crate::validate_working_budget(budget_samples, size_of::<f32>(), max_working_bytes)?;
    if source == destination {
        return Ok(samples.to_vec());
    }
    let client = device.client();
    let input = upload(&client, samples);
    let row_result = axis_pass(&client, source, horizontal, input, true);
    let image = axis_pass(&client, horizontal, destination, row_result, false);
    read_image(&client, image, destination.samples)
}

/// Resample one axis, reusing its tensor if the dimension already matches.
fn axis_pass(
    client: &Client,
    source: ImageShape,
    destination: ImageShape,
    input: TensorHandle,
    horizontal: bool,
) -> TensorHandle {
    if source == destination {
        return input;
    }
    let result = empty(client, destination.samples);
    let (groups, dim) = elementwise(client, destination.samples);
    let downsample = if horizontal {
        destination.width < source.width
    } else {
        destination.height < source.height
    };
    interpolate::launch(
        client,
        groups,
        dim,
        input.into_arg(),
        result.clone().into_arg(),
        source.width,
        source.height,
        destination.width,
        destination.height,
        horizontal,
        downsample,
    );
    result
}

/// Map a sample coordinate on the resampled axis into an RGB index.
#[cube(expand_only)]
fn source_index(
    axis: usize,
    cross: usize,
    channel: usize,
    source_width: usize,
    #[doc = "Select the horizontal rather than vertical pixel coordinate."]
    #[comptime]
    horizontal: bool,
) -> usize {
    let pixel = if horizontal {
        cross * source_width + axis
    } else {
        axis * source_width + cross
    };
    pixel * 3 + channel
}

/// Integrate a source footprint when reducing; otherwise interpolate centers.
#[cube(launch, expand_only)]
fn interpolate(
    input: &Tensor<f32>,
    output: &mut Tensor<f32>,
    source_width: usize,
    source_height: usize,
    destination_width: usize,
    destination_height: usize,
    #[doc = "Select the horizontal rather than vertical pixel coordinate."]
    #[comptime]
    horizontal: bool,
    #[doc = "Use box decimation instead of linear interpolation."]
    #[comptime]
    downsample: bool,
) {
    if ABSOLUTE_POS >= output.len() {
        terminate!();
    }
    let pixel = ABSOLUTE_POS / 3;
    let channel = ABSOLUTE_POS % 3;
    let x = pixel % destination_width;
    let y = pixel / destination_width;
    let (axis, cross, source_size, destination_size) = if horizontal {
        (x, y, source_width, destination_width)
    } else {
        (y, x, source_height, destination_height)
    };
    let scale = f32::cast_from(source_size) / f32::cast_from(destination_size);
    if downsample {
        let start = f32::cast_from(axis) * scale;
        let end = (f32::cast_from(axis) + 1.0) * scale;
        let start_index = usize::cast_from(start.floor());
        let end_index = usize::cast_from(end.ceil()).min(source_size);
        let mut sum = 0.0_f32;
        for sample in start_index..end_index {
            let left = start.max(f32::cast_from(sample));
            let right = end.min(f32::cast_from(sample) + 1.0);
            let width = right - left;
            sum += input[source_index(sample, cross, channel, source_width, horizontal)] * width;
        }
        output[ABSOLUTE_POS] = sum / scale;
    } else {
        let center = ((f32::cast_from(axis) + 0.5) * scale - 0.5)
            .max(0.0)
            .min(f32::cast_from(source_size - 1));
        let low = usize::cast_from(center.floor());
        let high = (low + 1).min(source_size - 1);
        let fraction = center - f32::cast_from(low);
        let a = input[source_index(low, cross, channel, source_width, horizontal)];
        let b = input[source_index(high, cross, channel, source_width, horizontal)];
        output[ABSOLUTE_POS] = a + fraction * (b - a);
    }
}
