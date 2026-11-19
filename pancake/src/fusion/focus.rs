//! Streaming weighted-average and sharpest-source focus fusion.

use std::fmt;

use cubecl::{prelude::*, std::tensor::TensorHandle};

use crate::registration::ResidentRgb;

use super::buffers::{ImageShape, elementwise, empty, read_image, upload};

/// Focus algorithms with streaming, constant-frame-count memory requirements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusMethod {
    /// Weight samples by their locally aggregated Laplacian energy.
    WeightedAverage,
    /// Select the source with the largest local modified-Laplacian response.
    DepthMap,
}

/// GPU/CPU state for one focus stack. Scratch lives across frames; each
/// submission completes before accepting a subsequent application operation.
#[derive(Debug)]
pub struct FocusAccumulator {
    /// The runtime selected for this job.
    device: cubecl::Device,
    /// The pixel grid shared by inputs, maps and result.
    shape: ImageShape,
    /// The accumulation rule.
    method: FocusMethod,
    /// Aggregation size selected by the job (normalized to an odd width at use).
    kernel_size: u16,
    /// Device tensors exist only once the first valid frame has arrived.
    state: Result<Option<FocusState>, String>,
}

/// Allocations reused in every frame. A successful add commits a frame to the
/// running result and synchronizes owned outputs before returning.
struct FocusState {
    /// Client issuing all buffer operations.
    client: Client,
    /// Single-channel input reused between energy and horizontal aggregation.
    luma: TensorHandle,
    /// Spatial contrast energy.
    energy: TensorHandle,
    /// Current frame focus map.
    weights: TensorHandle,
    /// Running total weight or largest score so far.
    scores: TensorHandle,
    /// Running weighted mean or the pixel values corresponding to best scores.
    image: TensorHandle,
    /// Gaussian coefficients for weighted averaging; unused by box filtering.
    coefficients: TensorHandle,
}

impl fmt::Debug for FocusState {
    /// Report the running image allocation without exposing runtime internals.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FocusState")
            .field("image", &self.image)
            .finish_non_exhaustive()
    }
}

impl FocusAccumulator {
    /// Validate a reference grid, the focus window, and a logical memory budget.
    ///
    /// The worst-case focus working set is at most 68 image bytes per pixel
    /// (caller frame, staging, RGB result, four single-channel arrays, and
    /// retained frame), plus space for Gaussian coefficients. Runtime pools and
    /// compiler overhead are outside this logical budget.
    ///
    /// # Errors
    /// Returns an error for invalid dimensions, a kernel size outside 1–63 or
    /// an insufficient working-data budget. No runtime is started here.
    pub fn new(
        device: &cubecl::Device,
        width: usize,
        height: usize,
        method: FocusMethod,
        kernel_size: u32,
        max_working_bytes: usize,
    ) -> Result<Self, String> {
        let shape = ImageShape::new([width, height])?;
        if !(1..=63).contains(&kernel_size) {
            return Err(format!("focus kernel size {kernel_size} is outside 1–63"));
        }
        let kernel_size = u16::try_from(kernel_size).map_err(|error| error.to_string())?;
        let overhead = usize::from(kernel_size | 1).saturating_mul(8);
        let remaining = max_working_bytes.saturating_sub(overhead);
        crate::validate_working_budget(shape.area, 68, remaining)?;
        Ok(Self {
            device: device.clone(),
            shape,
            method,
            kernel_size,
            state: Ok(None),
        })
    }

    /// Incorporate a finite packed RGB frame. Identity inputs retain their
    /// original f32 precision. A tie in depth maps keeps the earlier source.
    ///
    /// Submit from one worker on a qualified device. The return marks a real
    /// frame boundary at which no work from this method remains pending.
    ///
    /// # Errors
    /// Returns a sample validation or runtime error. A failed runtime
    /// submission invalidates the accumulator.
    pub fn add_frame(&mut self, samples: &[f32]) -> Result<(), String> {
        self.add_normalized_frame(samples, None)
    }

    /// Use validated host samples or the current frame's existing registration input.
    ///
    /// # Errors
    /// Returns a sample, layout, device ownership, or runtime completion error.
    pub(crate) fn add_normalized_frame(
        &mut self,
        samples: &[f32],
        resident: Option<ResidentRgb>,
    ) -> Result<(), String> {
        if resident.is_none() {
            self.shape.validate(samples)?;
        }
        let previous = self.state.as_mut().map_err(|error| error.clone())?;
        let first = previous.is_none();
        // Reject incompatible input before allocating the state that marks the first frame.
        let resident = resident
            .map(|input| {
                let client = previous
                    .as_ref()
                    .map_or_else(|| self.device.client(), |state| state.client.clone());
                input.into_tensor(&client, self.shape.width, self.shape.height)
            })
            .transpose()?;
        let state = previous.get_or_insert_with(|| {
            FocusState::new(self.device.client(), self.shape, self.kernel_size | 1)
        });
        let frame = resident.unwrap_or_else(|| upload(&state.client, samples));
        if let Err(error) = state.process(self.shape, &frame, self.method, self.kernel_size, first)
        {
            self.state = Err(error.clone());
            return Err(error);
        }
        Ok(())
    }

    /// Return the full-precision fused RGB image without display conversion.
    ///
    /// # Errors
    /// Returns an error for an empty stack or an invalid runtime readback.
    pub fn finish(self) -> Result<Vec<f32>, String> {
        let state = self.state?.ok_or("focus accumulator has no frames")?;
        read_image(&state.client, state.image, self.shape.samples)
    }
}

impl FocusState {
    /// Allocate reusable input, accumulation and separable-filter buffers.
    fn new(client: Client, shape: ImageShape, window: u16) -> Self {
        let coefficients = upload(&client, &gaussian_coefficients(window));
        Self {
            coefficients,
            luma: empty(&client, shape.area),
            energy: empty(&client, shape.area),
            weights: empty(&client, shape.area),
            scores: empty(&client, shape.area),
            image: empty(&client, shape.samples),
            client,
        }
    }

    /// Run all stages on the same stream and wait for the updated result.
    /// # Errors
    /// Returns a device fault or failed launch for any required result buffer.
    fn process(
        &self,
        shape: ImageShape,
        frame: &TensorHandle,
        method: FocusMethod,
        kernel_size: u16,
        first: bool,
    ) -> Result<(), String> {
        let client = &self.client;
        let depth = method == FocusMethod::DepthMap;
        let window = usize::from(kernel_size | 1);
        let (groups, dim) = elementwise(client, shape.area);
        launch_energy(client, shape, frame, &self.luma, &self.energy, depth);
        if depth {
            let coarse = usize::from(kernel_size.saturating_mul(3).saturating_add(1) | 1);
            self.filter(shape, 2, true, false);
            self.filter(shape, window / 2, true, true);
            self.filter(shape, coarse / 2, true, true);
        } else {
            self.filter(shape, window / 2, false, false);
        }
        combine::launch(
            client,
            groups,
            dim,
            frame.clone().into_arg(),
            self.weights.clone().into_arg(),
            self.scores.clone().into_arg(),
            self.image.clone().into_arg(),
            u32::from(depth),
            u32::from(first),
        );
        cubecl::future::block_on(client.sync_buffers([&self.image.handle, &self.scores.handle]))
            .map_err(|error| error.to_string())
    }

    /// Filter one scale separably. Multiple depth scales add to the same map.
    fn filter(&self, shape: ImageShape, radius: usize, box_filter: bool, accumulate: bool) {
        let (groups, dim) = elementwise(&self.client, shape.area);
        aggregate::launch(
            &self.client,
            groups.clone(),
            dim,
            self.energy.clone().into_arg(),
            self.coefficients.clone().into_arg(),
            self.luma.clone().into_arg(),
            shape.width,
            shape.height,
            radius,
            1,
            u32::from(box_filter),
            0,
        );
        aggregate::launch(
            &self.client,
            groups,
            dim,
            self.luma.clone().into_arg(),
            self.coefficients.clone().into_arg(),
            self.weights.clone().into_arg(),
            shape.width,
            shape.height,
            radius,
            0,
            u32::from(box_filter),
            u32::from(accumulate),
        );
    }
}

/// Measure mean squared luminance Laplacian.
///
/// Use one ordered device sum per row. Return normalized squared contrast in
/// the image's sample units. A flat image yields zero.
///
/// # Errors
/// Returns shape, sample, memory-budget, or runtime readback errors.
pub fn sharpness(
    device: &cubecl::Device,
    width: usize,
    height: usize,
    samples: &[f32],
    max_working_bytes: usize,
) -> Result<f32, String> {
    let shape = ImageShape::new([width, height])?;
    shape.validate(samples)?;
    crate::validate_working_budget(shape.area, 56, max_working_bytes)?;
    let client = device.client();
    let input = upload(&client, samples);
    let luma = empty(&client, shape.area);
    let energy = empty(&client, shape.area);
    launch_energy(&client, shape, &input, &luma, &energy, false);
    let rows = empty(&client, shape.height);
    let (groups, dim) = elementwise(&client, shape.height);
    sum_rows::launch(
        &client,
        groups,
        dim,
        energy.into_arg(),
        rows.clone().into_arg(),
        shape.width,
    );
    let values = read_image(&client, rows, shape.height)?;
    let count = u32::try_from(shape.area).map_err(|error| error.to_string())?;
    let mean = values.iter().copied().map(f64::from).sum::<f64>() / f64::from(count);
    // Each row's finite f32 sum is already bounded. Averaging nonnegative
    // energy per image pixel cannot exceed the largest finite row sum.
    Ok(clamp_mean(mean))
}

/// Narrow the nonnegative image mean to its original sample precision.
#[expect(
    clippy::cast_possible_truncation,
    reason = "The finite f64 mean is bounded by its f32 row sums."
)]
#[expect(
    clippy::as_conversions,
    reason = "Rust has no From<f64> conversion for the bounded f32 mean."
)]
const fn clamp_mean(mean: f64) -> f32 {
    mean as f32
}

/// Populate luma and focus energy using reusable buffers and the same stream.
fn launch_energy(
    client: &Client,
    shape: ImageShape,
    input: &TensorHandle,
    luma: &TensorHandle,
    energy: &TensorHandle,
    depth: bool,
) {
    let (groups, dim) = elementwise(client, shape.area);
    rgb_luma::launch(
        client,
        groups.clone(),
        dim,
        input.clone().into_arg(),
        luma.clone().into_arg(),
    );
    focus_energy::launch(
        client,
        groups,
        dim,
        luma.clone().into_arg(),
        energy.clone().into_arg(),
        shape.width,
        shape.height,
        u32::from(depth),
    );
}

/// Reduce spatial energy horizontally in deterministic sample order.
#[cube(launch, expand_only)]
fn sum_rows(input: &Tensor<f32>, output: &mut Tensor<f32>, width: usize) {
    if ABSOLUTE_POS >= output.len() {
        terminate!();
    }
    let row = ABSOLUTE_POS * width;
    let mut sum = 0.0_f32;
    for column in 0..width {
        sum += input[row + column];
    }
    output[ABSOLUTE_POS] = sum;
}

/// Build a normalized odd-width Gaussian stencil for the shared kernel.
fn gaussian_coefficients(window: u16) -> Vec<f32> {
    match window {
        1 => vec![1.0],
        3 => vec![0.25, 0.5, 0.25],
        5 => vec![0.0625, 0.25, 0.375, 0.25, 0.0625],
        _ => {
            let width = f32::from(window);
            let center = (width - 1.0) * 0.5;
            let sigma = width.mul_add(0.15, 0.35);
            let offsets = (0..window).map(|i| {
                let delta = (f32::from(i) - center) / sigma;
                (-0.5 * delta * delta).exp()
            });
            let mut coefficients = offsets.collect::<Vec<_>>();
            let total = coefficients.iter().sum::<f32>();
            for coefficient in &mut coefficients {
                *coefficient /= total;
            }
            coefficients
        }
    }
}

/// Compute RGB luminance without integer conversion or channel reordering.
#[cube(launch, expand_only)]
fn rgb_luma(image: &Tensor<f32>, output: &mut Tensor<f32>) {
    if ABSOLUTE_POS >= output.len() {
        terminate!();
    }
    let index = ABSOLUTE_POS * 3;
    output[ABSOLUTE_POS] =
        image[index] * 0.299 + image[index + 1] * 0.587 + image[index + 2] * 0.114;
}

/// Reflect coordinate +/- a filter radius across the two outermost sample centers.
#[cube(expand_only)]
fn reflect(coordinate: usize, offset: usize, radius: usize, count: usize) -> usize {
    if count == 1 {
        0_usize
    } else {
        let period = (count - 1) * 2;
        let wrapped = (coordinate + offset + period - radius % period) % period;
        if wrapped < count {
            wrapped
        } else {
            period - wrapped
        }
    }
}

/// Compute weighted-average or modified-Laplacian focus energy.
#[cube(launch, expand_only)]
fn focus_energy(
    input: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    depth: u32,
) {
    if ABSOLUTE_POS >= output.len() {
        terminate!();
    }
    let x = ABSOLUTE_POS % width;
    let y = ABSOLUTE_POS / width;
    let center = input[ABSOLUTE_POS];
    let lap_x = (input[y * width + reflect(x, 0, 1, width)] - center)
        + (input[y * width + reflect(x, 2, 1, width)] - center);
    let lap_y = (input[reflect(y, 0, 1, height) * width + x] - center)
        + (input[reflect(y, 2, 1, height) * width + x] - center);
    output[ABSOLUTE_POS] = if depth != 0 {
        lap_x.abs() + lap_y.abs()
    } else {
        let total = lap_x + lap_y;
        total * total
    };
}

/// Convolve one axis with a Gaussian stencil or a normalized box footprint.
#[cube(launch, expand_only)]
fn aggregate(
    input: &Tensor<f32>,
    coefficients: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    radius: usize,
    horizontal: u32,
    box_filter: u32,
    accumulate: u32,
) {
    if ABSOLUTE_POS >= output.len() {
        terminate!();
    }
    let x = ABSOLUTE_POS % width;
    let y = ABSOLUTE_POS / width;
    let kernel_width = radius * 2 + 1;
    let mut sum = 0.0_f32;
    for tap in 0..kernel_width {
        let index = if horizontal != 0 {
            y * width + reflect(x, tap, radius, width)
        } else {
            reflect(y, tap, radius, height) * width + x
        };
        sum += if box_filter != 0 {
            input[index]
        } else {
            input[index] * coefficients[tap]
        };
    }
    if box_filter != 0 {
        sum /= f32::cast_from(kernel_width);
    }
    if accumulate != 0 {
        sum += output[ABSOLUTE_POS];
    }
    output[ABSOLUTE_POS] = sum;
}

/// Update RGB and scores in frame order. A zero total retains the first frame.
#[cube(launch, expand_only)]
fn combine(
    frame: &Tensor<f32>,
    weights: &Tensor<f32>,
    scores: &mut Tensor<f32>,
    image: &mut Tensor<f32>,
    depth: u32,
    first: u32,
) {
    // A single active-lane bound protects both the score and the RGB tuple.
    if ABSOLUTE_POS >= weights.len() {
        terminate!();
    }
    let sample = ABSOLUTE_POS * 3;
    let weight = weights[ABSOLUTE_POS];
    if first != 0 {
        scores[ABSOLUTE_POS] = weight;
        image[sample] = frame[sample];
        image[sample + 1] = frame[sample + 1];
        image[sample + 2] = frame[sample + 2];
    } else if depth != 0 {
        if weight > scores[ABSOLUTE_POS] {
            scores[ABSOLUTE_POS] = weight;
            image[sample] = frame[sample];
            image[sample + 1] = frame[sample + 1];
            image[sample + 2] = frame[sample + 2];
        }
    } else if weight > 0.0 {
        let total = scores[ABSOLUTE_POS] + weight;
        let fraction = weight / total;
        scores[ABSOLUTE_POS] = total;
        for channel in 0..3 {
            let index = sample + channel;
            let prior = image[index];
            image[index] = prior + fraction * (frame[index] - prior);
        }
    }
}
