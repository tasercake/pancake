//! Safe image-kernel qualification for the Pancake focus-stacking application.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

/// Slint frontend API, using the same pinned wgpu generation as image compute.
pub use slint;
/// Shared GPU resource API for Slint rendering and `CubeCL` compute.
pub use wgpu;

#[cfg(test)]
mod tests;

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
    let elements = validate_shape(width, height, pixels.len())?;
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
