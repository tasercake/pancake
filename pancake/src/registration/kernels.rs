//! Portable image sampling and registration scoring.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

use super::geometry::{Affine, Bounds, Extent, Frame};

/// Limit one warp output allocation to one MiB of f32 samples.
const WARP_TILE: usize = 262_144;
/// Number of texture samples contributing to one correlation record.
const SCORE_BLOCK: usize = 64;
/// Number of sufficient statistics produced by each score block.
const MOMENTS: usize = 5;

/// Resample one RGB source into a reference-space rectangle.
///
/// The source upload remains resident across output tiles. The same bilinear
/// pixel-center sampler runs on CPU and GPU; borders extend the nearest sample.
///
/// # Errors
///
/// Returns a budget, coordinate, readback, or nonfinite sample error. The
/// caller must supply a device whose runtime is available to this build.
pub fn warp(
    device: &cubecl::Device,
    frame: Frame<'_>,
    transform: Affine,
    output: Bounds,
    working_budget: usize,
) -> Result<Vec<f32>, String> {
    let expected = output.extent.samples()?;
    let tile = expected.min(WARP_TILE);
    check_budget(frame.samples.len(), expected, tile, working_budget)?;
    let client = device.client();
    let input = upload(&client, frame.samples);
    let [[a, b, tx], [c, d, ty]] = transform.matrix();
    let mapping = upload(
        &client,
        &[a, b, tx, c, d, ty, f32::from(output.x), f32::from(output.y)],
    );
    let mut output_samples = Vec::with_capacity(expected);
    while output_samples.len() < expected {
        let start = output_samples.len();
        let count = expected.saturating_sub(start).min(tile);
        let result = TensorHandle::empty(&client, [count], f32::cube_type());
        let dimension = CubeDim::new(&client, count);
        let cubes = cubecl::calculate_cube_count_elemwise(&client, count, dimension);
        warp_rgb::launch(
            &client,
            cubes,
            dimension,
            input.clone().into_arg(),
            mapping.clone().into_arg(),
            result.clone().into_arg(),
            usize::from(frame.extent.width),
            usize::from(frame.extent.height),
            output.width(),
            start,
        );
        output_samples.extend(read(&client, result, count)?);
    }
    Ok(output_samples)
}

/// Check explicit caller, device, staging and returned-buffer sizes before use.
///
/// # Errors
///
/// Returns an error for an overflowing or oversized request.
fn check_budget(input: usize, output: usize, tile: usize, budget: usize) -> Result<(), String> {
    let count = input
        .checked_mul(2)
        .and_then(|samples| samples.checked_add(output))
        .and_then(|samples| samples.checked_add(tile.checked_mul(3)?))
        .and_then(|samples| samples.checked_add(32))
        .ok_or("registration working byte count exceeds address space")?;
    crate::validate_working_budget(count, size_of::<f32>(), budget)
}

/// Upload packed rows or short affine parameters as one typed device tensor.
fn upload(client: &Client, data: &[f32]) -> TensorHandle {
    TensorHandle::new_contiguous(
        [data.len()],
        client.create_from_slice(f32::as_bytes(data)),
        f32::cube_type(),
    )
}

/// Read exactly the declared number of finite device samples.
///
/// # Errors
///
/// Returns a device error or malformed readback error.
fn read(client: &Client, handle: TensorHandle, count: usize) -> Result<Vec<f32>, String> {
    let buffers = client
        .read_tensor(vec![handle.into_copy_descriptor()])
        .map_err(|error| error.to_string())?;
    let mut result = Vec::with_capacity(count);
    for bytes in buffers {
        result.extend(crate::decode_variance_values(&bytes)?);
    }
    if result.len() == count {
        Ok(result)
    } else {
        Err("registration readback did not return the expected sample count".to_owned())
    }
}

/// Prefilter a compact, one-channel registration image in normalized coordinates.
///
/// # Errors
///
/// Returns a budget, shape or readback error.
pub(super) fn make_luma(
    device: &cubecl::Device,
    frame: Frame<'_>,
    dimensions: Extent,
    budget: usize,
) -> Result<Vec<f32>, String> {
    let count = dimensions.area()?;
    check_budget(frame.samples.len(), count, count, budget)?;
    let client = device.client();
    let input = upload(&client, frame.samples);
    let output = TensorHandle::empty(&client, [count], f32::cube_type());
    let dim = CubeDim::new(&client, count);
    let cubes = cubecl::calculate_cube_count_elemwise(&client, count, dim);
    let scale_x = f32::from(frame.extent.width) / f32::from(dimensions.width);
    let scale_y = f32::from(frame.extent.height) / f32::from(dimensions.height);
    smooth_luma::launch(
        &client,
        cubes,
        dim,
        input.into_arg(),
        output.clone().into_arg(),
        usize::from(frame.extent.width),
        usize::from(frame.extent.height),
        usize::from(dimensions.width),
        scale_x,
        scale_y,
    );
    read(&client, output, count)
}

/// Device-owned prepared pair reused across all search iterations.
pub(super) struct Pair {
    /// Client that owns both thumbnails and short score buffers.
    client: Client,
    /// Luminance of the reference canvas.
    reference: TensorHandle,
    /// Luminance of the moving source in same-size coordinates.
    source: TensorHandle,
    /// Dimensions common to both thumbnails.
    extent: Extent,
}

impl Pair {
    /// Upload both validated, same-size thumbnails once per pair.
    pub(super) fn new(
        device: &cubecl::Device,
        reference: &[f32],
        source: &[f32],
        extent: Extent,
    ) -> Self {
        let client = device.client();
        Self {
            reference: upload(&client, reference),
            source: upload(&client, source),
            extent,
            client,
        }
    }

    /// Evaluate correlation for a small batch of candidate maps.
    ///
    /// # Errors
    ///
    /// Returns an error for oversized score buffers or failed GPU readback.
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "the fixed fractional border cannot consume an axis and stride is 1, 2, or 4"
    )]
    pub(super) fn correlation(&self, maps: &[Affine], stride: u16) -> Result<Vec<f32>, String> {
        let w = usize::from(self.extent.width);
        let h = usize::from(self.extent.height);
        let border_x = w / 8;
        let border_y = h / 8;
        let stride = usize::from(stride);
        let columns = (w - border_x * 2).div_ceil(stride);
        let rows = (h - border_y * 2).div_ceil(stride);
        let samples = columns
            .checked_mul(rows)
            .ok_or("registration correlation sample count exceeds address space")?;
        let blocks = samples.div_ceil(SCORE_BLOCK);
        let records = blocks
            .checked_mul(maps.len())
            .ok_or("too many registration candidates")?;
        let output_count = records
            .checked_mul(MOMENTS)
            .ok_or("registration correlation output exceeds address space")?;
        let layout = [
            w, h, border_x, border_y, stride, columns, samples, blocks, records,
        ]
        .map(|entry| u32::try_from(entry).map_err(|error| error.to_string()))
        .into_iter()
        .collect::<Result<Vec<u32>, _>>()?;
        let parameters = maps
            .iter()
            .flat_map(|map| map.matrix().into_iter().flatten())
            .collect::<Vec<_>>();
        let maps_device = upload(&self.client, &parameters);
        let metadata = TensorHandle::new_contiguous(
            [layout.len()],
            self.client.create_from_slice(u32::as_bytes(&layout)),
            u32::cube_type(),
        );
        let output = TensorHandle::empty(&self.client, [output_count], f32::cube_type());
        let dim = CubeDim::new(&self.client, records);
        let cubes = cubecl::calculate_cube_count_elemwise(&self.client, records, dim);
        correlation_moments::launch(
            &self.client,
            cubes,
            dim,
            self.reference.clone().into_arg(),
            self.source.clone().into_arg(),
            maps_device.into_arg(),
            metadata.into_arg(),
            output.clone().into_arg(),
        );
        let moments = read(&self.client, output, output_count)?;
        let sample_count = u32::try_from(samples).map_err(|error| error.to_string())?;
        let per_candidate = blocks
            .checked_mul(MOMENTS)
            .ok_or("oversize registration result")?;
        Ok(moments
            .chunks_exact(per_candidate)
            .map(|chunks| pearson(chunks, sample_count))
            .collect())
    }
}

/// Reduce block moments in fixed order and evaluate normalized correlation.
#[expect(
    clippy::as_conversions,
    reason = "the score is finite and clamped before narrowing"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "final correlation is clamped to [-1,1]"
)]
fn pearson(moments: &[f32], samples: u32) -> f32 {
    let mut totals = [0.0_f64; MOMENTS];
    let (groups, _) = moments.as_chunks::<MOMENTS>();
    for group in groups {
        for (total, &value) in totals.iter_mut().zip(group) {
            *total += f64::from(value);
        }
    }
    let [sum_a, sum_b, square_a, square_b, product] = totals;
    let count = f64::from(samples);
    let var_a = (-sum_a / count).mul_add(sum_a, square_a);
    let var_b = (-sum_b / count).mul_add(sum_b, square_b);
    if var_a <= 1e-7_f64 * count || var_b <= 1e-7_f64 * count {
        return 0.0;
    }
    let covariance = (-sum_a / count).mul_add(sum_b, product);
    (covariance / (var_a * var_b).sqrt()).clamp(-1.0, 1.0) as f32
}

/// Checked bilinear sampling in a clamped image, including single-pixel axes.
#[cube(expand_only)]
fn bilinear(
    input: &Tensor<f32>,
    x: f32,
    y: f32,
    width: usize,
    height: usize,
    channel: usize,
    channels: usize,
) -> f32 {
    let max_x = width - 1;
    let max_y = height - 1;
    let px = x.max(0.0).min(f32::cast_from(max_x));
    let py = y.max(0.0).min(f32::cast_from(max_y));
    let x0 = usize::cast_from(px);
    let y0 = usize::cast_from(py);
    let x1 = (x0 + 1).min(max_x);
    let y1 = (y0 + 1).min(max_y);
    let wx = px - f32::cast_from(x0);
    let wy = py - f32::cast_from(y0);
    let v00 = input[(y0 * width + x0) * channels + channel];
    let v10 = input[(y0 * width + x1) * channels + channel];
    let v01 = input[(y1 * width + x0) * channels + channel];
    let v11 = input[(y1 * width + x1) * channels + channel];
    let a = (1.0 - wx) * v00 + wx * v10;
    let b = (1.0 - wx) * v01 + wx * v11;
    (1.0 - wy) * a + wy * b
}

/// One f32 channel per lane, with ROI coordinates retained across tiles.
#[cube(launch, expand_only)]
fn warp_rgb(
    input: &Tensor<f32>,
    map: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    destination_width: usize,
    start: usize,
) {
    let channel_index = ABSOLUTE_POS + start;
    let pixel = channel_index / 3;
    let x = f32::cast_from(pixel % destination_width) + map[6];
    let y = f32::cast_from(pixel / destination_width) + map[7];
    let sx = map[0] * x + map[1] * y + map[2];
    let sy = map[3] * x + map[4] * y + map[5];
    output[ABSOLUTE_POS] = bilinear(input, sx, sy, width, height, channel_index % 3, 3);
}

/// Binomial five-tap low-pass coefficients.
#[cube(expand_only)]
fn binomial(index: usize) -> f32 {
    if index == 2 {
        6.0_f32
    } else if index % 2 == 1 {
        4.0_f32
    } else {
        1.0_f32
    }
}

/// Generate a low-pass luminance thumbnail directly from packed source RGB.
#[cube(launch, expand_only)]
fn smooth_luma(
    input: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    out_width: usize,
    scale_x: f32,
    scale_y: f32,
) {
    let center_x = f32::cast_from(ABSOLUTE_POS % out_width) + 0.5;
    let center_y = f32::cast_from(ABSOLUTE_POS / out_width) + 0.5;
    let mut value = 0.0_f32;
    for row in 0..5 {
        let sy = (center_y + f32::cast_from(row) - 2.0) * scale_y - 0.5;
        for col in 0..5 {
            let sx = (center_x + f32::cast_from(col) - 2.0) * scale_x - 0.5;
            let r = bilinear(input, sx, sy, width, height, 0, 3).clamp(0.0, 1.0);
            let g = bilinear(input, sx, sy, width, height, 1, 3).clamp(0.0, 1.0);
            let b = bilinear(input, sx, sy, width, height, 2, 3).clamp(0.0, 1.0);
            let gray = 0.299 * r + 0.587 * g + 0.114 * b;
            value += binomial(col) * binomial(row) * gray;
        }
    }
    output[ABSOLUTE_POS] = value / 256.0;
}

/// Output independent short reductions, one block per lane and candidate.
#[cube(launch, expand_only)]
fn correlation_moments(
    reference: &Tensor<f32>,
    source: &Tensor<f32>,
    maps: &Tensor<f32>,
    layout: &Tensor<u32>,
    output: &mut Tensor<f32>,
) {
    if ABSOLUTE_POS >= usize::cast_from(layout[8]) {
        terminate!();
    }
    let w = usize::cast_from(layout[0]);
    let h = usize::cast_from(layout[1]);
    let border_x = usize::cast_from(layout[2]);
    let border_y = usize::cast_from(layout[3]);
    let stride = usize::cast_from(layout[4]);
    let columns = usize::cast_from(layout[5]);
    let count = usize::cast_from(layout[6]);
    let blocks = usize::cast_from(layout[7]);
    let block = ABSOLUTE_POS % blocks;
    let candidate = (ABSOLUTE_POS / blocks) * 6;
    let start = block * SCORE_BLOCK;
    let end = (start + SCORE_BLOCK).min(count);
    let mut sum_a = 0.0_f32;
    let mut sum_b = 0.0_f32;
    let mut sum_aa = 0.0_f32;
    let mut sum_bb = 0.0_f32;
    let mut sum_ab = 0.0_f32;
    for pos in start..end {
        let col = border_x + (pos % columns) * stride;
        let row = border_y + (pos / columns) * stride;
        let x = f32::cast_from(col);
        let y = f32::cast_from(row);
        let sx = maps[candidate] * x + maps[candidate + 1] * y + maps[candidate + 2];
        let sy = maps[candidate + 3] * x + maps[candidate + 4] * y + maps[candidate + 5];
        let a = reference[row * w + col];
        let b = bilinear(source, sx, sy, w, h, 0, 1);
        sum_a += a;
        sum_b += b;
        sum_aa += a * a;
        sum_bb += b * b;
        sum_ab += a * b;
    }
    let record = ABSOLUTE_POS * MOMENTS;
    output[record] = sum_a;
    output[record + 1] = sum_b;
    output[record + 2] = sum_aa;
    output[record + 3] = sum_bb;
    output[record + 4] = sum_ab;
}
