//! Checked RGB geometry and contiguous `CubeCL` buffer ownership.

use cubecl::{bytes::Bytes, prelude::*, std::tensor::TensorHandle};

/// Validated packed RGB dimensions, storing pre-checked element counts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ImageShape {
    /// Width in pixels.
    pub(super) width: usize,
    /// Height in pixels.
    pub(super) height: usize,
    /// Number of spatial samples in a single channel.
    pub(super) area: usize,
    /// Number of f32 samples in an interleaved RGB image.
    pub(super) samples: usize,
}

impl ImageShape {
    /// Validate positive geometry and bounded device addressing.
    /// # Errors
    /// Returns an error for empty dimensions or an unaddressable sample count.
    pub(super) fn new([width, height]: [usize; 2]) -> Result<Self, String> {
        if width == 0 || height == 0 {
            return Err("image dimensions must be nonzero".to_owned());
        }
        let (area, samples) = width
            .checked_mul(height)
            .and_then(|area| area.checked_mul(3).map(|samples| (area, samples)))
            .ok_or("image area exceeds device addressing")?;
        let _addressable =
            u32::try_from(samples).map_err(|_| "image area exceeds u32 addressing")?;
        // Pixel midpoint arithmetic in resize must keep half-pixel precision.
        if width > 4_194_304 || height > 4_194_304 {
            return Err("image dimensions exceed exact sample-coordinate range".to_owned());
        }
        Ok(Self {
            width,
            height,
            area,
            samples,
        })
    }

    /// Reject malformed or nonfinite inputs before making a runtime submission.
    /// # Errors
    /// Returns an error for the wrong sample count or a nonfinite sample.
    pub(super) fn validate(self, samples: &[f32]) -> Result<(), String> {
        if samples.len() != self.samples {
            return Err("sample count does not match packed RGB dimensions".to_owned());
        }
        if samples.iter().any(|sample| !sample.is_finite()) {
            return Err("image contains a nonfinite sample".to_owned());
        }
        Ok(())
    }
}

/// Allocate device storage that the initial kernel or upload will fully populate.
pub(super) fn empty(client: &Client, count: usize) -> TensorHandle {
    TensorHandle::empty(client, [count], f32::cube_type())
}

/// Transfer exactly one contiguous f32 array using an owned upload buffer.
pub(super) fn upload(client: &Client, samples: &[f32]) -> TensorHandle {
    TensorHandle::new_contiguous(
        [samples.len()],
        client.create(Bytes::from_bytes_vec(f32::as_bytes(samples).to_vec())),
        f32::cube_type(),
    )
}

/// Launch geometry shared by the checked per-pixel and per-channel kernels.
pub(super) fn elementwise(client: &Client, count: usize) -> (CubeCount, CubeDim) {
    let dim = CubeDim::new(client, count);
    (
        cubecl::calculate_cube_count_elemwise(client, count, dim),
        dim,
    )
}

/// Copy one completed device tensor and validate the full f32 readback.
/// # Errors
/// Returns a runtime, wrong-size or nonfinite result error.
pub(super) fn read_image(
    client: &Client,
    tensor: TensorHandle,
    expected_samples: usize,
) -> Result<Vec<f32>, String> {
    let buffers = client
        .read_tensor(vec![tensor.into_copy_descriptor()])
        .map_err(|error| error.to_string())?;
    let [bytes] = buffers.as_slice() else {
        return Err("device returned the wrong number of image buffers".to_owned());
    };
    let (samples, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() || samples.len() != expected_samples {
        return Err("device returned a truncated or oversized image".to_owned());
    }
    let output = samples
        .iter()
        .map(|sample| f32::from_ne_bytes(*sample))
        .collect::<Vec<_>>();
    if output.iter().any(|sample| !sample.is_finite()) {
        return Err("device returned a nonfinite image sample".to_owned());
    }
    Ok(output)
}
