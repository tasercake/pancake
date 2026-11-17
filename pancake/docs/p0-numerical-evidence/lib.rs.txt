//! Safe image-kernel qualification for the Pancake focus-stacking application.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

/// Slint frontend API, using the same pinned wgpu generation as image compute.
pub use slint;
/// Shared GPU resource API for Slint rendering and `CubeCL` compute.
pub use wgpu;

#[cfg(test)]
mod tests;

/// Default upper bound for logical kernel working data: 512 MiB.
///
/// This bounds image/staging requests, not driver/JIT overhead or measured RSS.
pub const DEFAULT_WORKING_BUDGET_BYTES: usize = 536_870_912;

/// Apply a horizontal `[1, 2, 1] / 4` blur to a row-major u16 grayscale plane.
///
/// Border samples repeat the nearest pixel. Division rounds down. Samples use
/// u32 device storage and accumulation, so all 16 input bits remain intact on
/// backends without native 16-bit buffer support. CPU and GPU execute one kernel.
///
/// # Errors
///
/// Returns an error for zero dimensions, a u32 area overflow, mismatched input,
/// failed device readback or an out-of-range device result.
pub fn blur_horizontal_u16(
    device: &cubecl::Device,
    width: usize,
    height: usize,
    pixels: &[u16],
) -> Result<Vec<u16>, String> {
    blur_horizontal_u16_bounded(device, width, height, pixels, DEFAULT_WORKING_BUDGET_BYTES)
}

/// Apply the shared blur with an explicit logical working-data byte budget.
///
/// The conservative 20 bytes per pixel include caller samples, host widening,
/// two device buffers, readback and returned samples. Pools and JIT/driver
/// overhead are reported separately; this is not a physical-memory estimate.
///
/// # Errors
///
/// Returns shape, budget, runtime readback or sample-range errors. Budget and
/// shape checks finish before creating a runtime or allocating kernel buffers.
pub fn blur_horizontal_u16_bounded(
    device: &cubecl::Device,
    width: usize,
    height: usize,
    pixels: &[u16],
    max_working_bytes: usize,
) -> Result<Vec<u16>, String> {
    let elements = validate_shape(width, height, pixels.len())?;
    validate_working_budget(elements, 20, max_working_bytes)?;
    let client = device.client();
    let widened: Vec<u32> = pixels.iter().copied().map(u32::from).collect();
    let input = TensorHandle::new_contiguous(
        [elements],
        client.create_from_slice(u32::as_bytes(&widened)),
        u32::cube_type(),
    );
    let output = TensorHandle::empty(&client, [elements], u32::cube_type());
    let cube_dim = CubeDim::new(&client, elements);
    let cube_count = cubecl::calculate_cube_count_elemwise(&client, elements, cube_dim);
    horizontal_blur::launch(
        &client,
        cube_count,
        cube_dim,
        input.into_arg(),
        output.clone().into_arg(),
        width,
    );
    finish_readback(client.read_tensor(vec![output.into_copy_descriptor()]))
}

/// Validate plane dimensions without starting a runtime or allocating its buffers.
///
/// # Errors
///
/// Returns an error for empty dimensions, an unaddressable area or a wrong count.
fn validate_shape(width: usize, height: usize, count: usize) -> Result<usize, String> {
    if width == 0 || height == 0 {
        return Err("image dimensions must be nonzero".to_owned());
    }
    let area = width
        .checked_mul(height)
        .ok_or("image area exceeds u32 addressing")?;
    let _addressable = u32::try_from(area).map_err(|_| "image area exceeds u32 addressing")?;
    if count != area {
        return Err("pixel count does not match image dimensions".to_owned());
    }
    Ok(area)
}

/// Propagate a runtime error or decode its returned buffers without panicking.
///
/// # Errors
///
/// Returns the runtime error or a byte-length/sample-range decoding error.
fn finish_readback<E: std::fmt::Display>(
    readback: Result<Vec<cubecl::bytes::Bytes>, E>,
) -> Result<Vec<u16>, String> {
    let buffers = readback.map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    for bytes in buffers {
        output.extend(decode_pixels(&bytes)?);
    }
    Ok(output)
}

/// Decode native-endian u32 storage, checking byte length and sample range.
///
/// # Errors
///
/// Returns an error for an incomplete u32 sample or a value above 65535.
fn decode_pixels(bytes: &[u8]) -> Result<Vec<u16>, String> {
    let (samples, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() {
        return Err("device readback length is not a multiple of four".to_owned());
    }
    samples
        .iter()
        .map(|sample| {
            u16::try_from(u32::from_ne_bytes(*sample))
                .map_err(|_| "device result exceeds u16 range".to_owned())
        })
        .collect()
}

/// Device kernel: checked launch bounds every input read and output write.
///
/// The host validates width and u32 area. Each input is at most 65535, so the
/// largest sum is 262140. Clamped indices remain within the same valid row.
#[cube(launch, expand_only)]
fn horizontal_blur(input: &Tensor<u32>, output: &mut Tensor<u32>, width: usize) {
    let column = ABSOLUTE_POS % width;
    let left = if column == 0 {
        ABSOLUTE_POS
    } else {
        ABSOLUTE_POS - 1
    };
    let right = if column + 1 == width {
        ABSOLUTE_POS
    } else {
        ABSOLUTE_POS + 1
    };
    output[ABSOLUTE_POS] = (input[left] + 2 * input[ABSOLUTE_POS] + input[right]) / 4;
}

/// Return one population variance per row using a centered two-pass reduction.
///
/// The first sample is the origin for both passes. Bright flat inputs therefore
/// remain exactly zero, avoiding cancellation from subtracting large squares.
/// CPU and GPU execute this same f32 kernel; u16 samples widen without loss.
/// A conservative 22 bytes per input pixel bounds samples/staging/device output.
///
/// # Errors
///
/// Returns shape/budget errors before runtime initialization, then runtime or
/// malformed/nonfinite readback errors. This does not certify every reduction
/// size or replace the later complete numerical acceptance matrix.
pub fn row_variance_u16(
    device: &cubecl::Device,
    width: usize,
    height: usize,
    pixels: &[u16],
    max_working_bytes: usize,
) -> Result<Vec<f32>, String> {
    let elements = validate_shape(width, height, pixels.len())?;
    validate_working_budget(elements, 22, max_working_bytes)?;
    let client = device.client();
    let widened: Vec<u32> = pixels.iter().copied().map(u32::from).collect();
    let input = TensorHandle::new_contiguous(
        [elements],
        client.create_from_slice(u32::as_bytes(&widened)),
        u32::cube_type(),
    );
    let output = TensorHandle::empty(&client, [height], f32::cube_type());
    let cube_dim = CubeDim::new(&client, height);
    let cube_count = cubecl::calculate_cube_count_elemwise(&client, height, cube_dim);
    centered_variance::launch(
        &client,
        cube_count,
        cube_dim,
        input.into_arg(),
        output.clone().into_arg(),
        width,
    );
    finish_variances(client.read_tensor(vec![output.into_copy_descriptor()]))
}

/// Check logical working bytes before allocating or initializing a device.
///
/// # Errors
///
/// Returns an overflow or caller budget error. This excludes runtime overhead.
fn validate_working_budget(
    elements: usize,
    bytes_per_element: usize,
    available: usize,
) -> Result<(), String> {
    let required = elements
        .checked_mul(bytes_per_element)
        .ok_or("kernel working byte count exceeds usize addressing")?;
    if required > available {
        return Err("kernel working-data budget exceeded".to_owned());
    }
    Ok(())
}

/// Preserve runtime errors and validate every returned variance buffer.
///
/// # Errors
///
/// Returns a runtime error, incomplete f32 sample or nonfinite result error.
fn finish_variances<E: std::fmt::Display>(
    readback: Result<Vec<cubecl::bytes::Bytes>, E>,
) -> Result<Vec<f32>, String> {
    let buffers = readback.map_err(|error| error.to_string())?;
    let mut output = Vec::new();
    for bytes in buffers {
        output.extend(decode_variance_values(&bytes)?);
    }
    Ok(output)
}

/// Decode finite native-endian f32 values without assuming buffer alignment.
///
/// # Errors
///
/// Returns an incomplete-sample or nonfinite-value error.
fn decode_variance_values(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let (samples, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() {
        return Err("variance readback length is not a multiple of four".to_owned());
    }
    samples
        .iter()
        .map(|sample| {
            let value = f32::from_ne_bytes(*sample);
            if !value.is_finite() {
                return Err("device variance result is nonfinite".to_owned());
            }
            Ok(value)
        })
        .collect()
}

/// Checked launch clips inactive output lanes and every source tensor access.
///
/// Active rows are in range because the host validates the u32 area and count.
/// Centered sums avoid losing the constant bright origin before reduction.
#[cube(launch, expand_only)]
fn centered_variance(input: &Tensor<u32>, output: &mut Tensor<f32>, width: usize) {
    let row = ABSOLUTE_POS * width;
    let base = f32::cast_from(input[row]);
    let mut centered_sum = 0.0_f32;
    for column in 0..width {
        centered_sum += f32::cast_from(input[row + column]) - base;
    }
    let mean = centered_sum / f32::cast_from(width);
    let mut square_sum = 0.0_f32;
    for column in 0..width {
        let difference = f32::cast_from(input[row + column]) - base - mean;
        square_sum += difference * difference;
    }
    output[ABSOLUTE_POS] = square_sum / f32::cast_from(width);
}
