//! Streaming multiscale Laplacian, Hybrid, and exposure fusion.

use std::fmt;

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

mod levels;
mod plan;
mod pyramid;
mod weights;

#[cfg(test)]
mod tests;

pub use plan::{AutomaticParameters, automatic_parameters, slab_ranges};

use super::buffers::{ImageShape, empty, read_image, upload};
use levels::Level;
use pyramid::{grids, reconstruct, reduce};

/// Maximum frames retained by the exposure batch tree before reconstruction.
const EXPOSURE_BATCH_SIZE: usize = 4;

/// Maximum local focus radius is deliberately bounded before kernel launch.
const MAX_KERNEL_SIZE: usize = 64;

/// Multiscale fusion method with a shared device-independent runtime path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdvancedMethod {
    /// Select the sharpest Laplacian band at each pixel and pyramid scale.
    Laplacian,
    /// Combine selected detail with progressively softer coarse-band blending.
    Hybrid,
    /// Fuse exposures using contrast and saturation, in four-frame batches.
    Exposure,
}

/// Bounded stream of frames in a common coordinate system.
///
/// Each added frame completes its device work before returning. Persistent
/// statistics scale with image size and pyramid depth independently of the
/// frame count. The caller owns registration, output, and scheduling.
pub struct AdvancedAccumulator {
    /// Selected compute stream; CPU and physical GPUs dispatch the same kernels.
    client: Client,
    /// Untiled input and output grid.
    grid: ImageShape,
    /// Statistics ordered from the original grid to the smallest grid.
    levels: Vec<Level>,
    /// Selected accumulation and reconstruction rules.
    method: AdvancedMethod,
    /// Radius of the local focus-support window.
    radius: usize,
    /// Accepted frames since the current batch began.
    frames: usize,
    /// Running sum of fully reconstructed exposure batches.
    batch_sum: Option<TensorHandle>,
    /// Number of exposure batches represented in the sum.
    batches: u32,
    /// A failed submission leaves the stream's retained output unusable.
    failure: Option<String>,
}

impl fmt::Debug for AdvancedAccumulator {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdvancedAccumulator")
            .field("grid", &self.grid)
            .field("method", &self.method)
            .field("frames", &self.frames)
            .field("batches", &self.batches)
            .finish_non_exhaustive()
    }
}

impl AdvancedAccumulator {
    /// Allocate a streaming accumulator for already registered RGB frames.
    ///
    /// The requested method controls the persistent per-level statistics.
    /// Logical arrays and staging are checked before starting the known device.
    ///
    /// # Errors
    /// Returns an error for invalid dimensions, an unsupported window or
    /// pyramid depth, or a request above the caller's working-data budget.
    pub fn new(
        device: &cubecl::Device,
        width: usize,
        height: usize,
        method: AdvancedMethod,
        kernel_size: usize,
        pyramid_levels: usize,
        max_working_bytes: usize,
    ) -> Result<Self, String> {
        let grid = ImageShape::new([width, height])?;
        if !(1..=MAX_KERNEL_SIZE).contains(&kernel_size) {
            return Err(format!(
                "fusion kernel size {kernel_size} is outside 1..={MAX_KERNEL_SIZE}"
            ));
        }
        if pyramid_levels > 32 {
            return Err(format!(
                "fusion pyramid depth {pyramid_levels} is outside 0..=32"
            ));
        }
        let grids = grids(grid, pyramid_levels)?;
        validate_budget(&grids, grid, method, max_working_bytes)?;
        let client = device.client();
        validate_buffer_limit(grid, client.properties().memory.max_page_size)?;
        let levels = grids
            .into_iter()
            .map(|level| Level::new(&client, level, method))
            .collect();
        Ok(Self {
            client,
            grid,
            levels,
            method,
            radius: kernel_size / 2,
            frames: 0,
            batch_sum: None,
            batches: 0,
            failure: None,
        })
    }

    /// Add the next finite interleaved RGB frame at its original precision.
    ///
    /// # Errors
    /// Returns a shape, nonfinite-sample, arithmetic or runtime completion error.
    pub fn add_frame(&mut self, rgb: &[f32]) -> Result<(), String> {
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        self.grid.validate(rgb)?;
        let result = self.submit_frame(rgb);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }

    /// Queue the shared pipeline, then check all mutated output buffers.
    ///
    /// # Errors
    /// Returns resource, submission or completed-kernel errors.
    fn submit_frame(&mut self, rgb: &[f32]) -> Result<(), String> {
        let first = self.frames == 0;
        let mut image = upload(&self.client, rgb);
        let mut weights = weights::weights(
            &self.client,
            &image,
            self.grid,
            self.radius,
            self.method == AdvancedMethod::Exposure,
        );
        let mut levels = self.levels.iter().peekable();
        while let Some(level) = levels.next() {
            let Some(next) = levels.peek() else {
                level.accumulate(&self.client, &image, None, &weights, self.method, first);
                break;
            };
            let coarse = reduce(&self.client, &image, level.grid, next.grid, 3)?;
            level.accumulate(
                &self.client,
                &image,
                Some((&coarse, next.grid)),
                &weights,
                self.method,
                first,
            );
            weights = reduce(&self.client, &weights, level.grid, next.grid, 1)?;
            image = coarse;
        }

        self.frames = self
            .frames
            .checked_add(1)
            .ok_or("fusion frame count overflow")?;
        if self.method == AdvancedMethod::Exposure && self.frames == EXPOSURE_BATCH_SIZE {
            self.finish_exposure_batch()?;
        }
        let outputs = self
            .levels
            .iter()
            .flat_map(Level::outputs)
            .chain(self.batch_sum.iter().map(|sum| &sum.handle));
        cubecl::future::block_on(self.client.sync_buffers(outputs))
            .map_err(|error| error.to_string())
    }

    /// Reconstruct and read back an output on the original (possibly odd) grid.
    ///
    /// # Errors
    /// Returns a runtime or numerical error, or an error for an empty stream.
    pub fn finish(mut self) -> Result<Vec<f32>, String> {
        if let Some(error) = self.failure.take() {
            return Err(error);
        }
        let output = if self.method == AdvancedMethod::Exposure {
            if self.frames > 0 {
                self.finish_exposure_batch()?;
            }
            let total = self
                .batch_sum
                .ok_or("cannot finish fusion without frames")?;
            let (blocks, threads) = elementwise(&self.client, self.grid.samples);
            levels::average_batches::launch(
                &self.client,
                blocks,
                threads,
                total.clone().into_arg(),
                self.batches,
            );
            total
        } else {
            if self.frames == 0 {
                return Err("cannot finish fusion without frames".to_owned());
            }
            self.reconstruct()?
        };
        read_image(&self.client, output, self.grid.samples)
    }

    /// Keep the partial batch exactly as influential as each set of four.
    ///
    /// # Errors
    /// Returns an error if reconstruction fails or the batch count overflows.
    fn finish_exposure_batch(&mut self) -> Result<(), String> {
        let output = self.reconstruct()?;
        let first = self.batch_sum.is_none();
        let sum = self
            .batch_sum
            .get_or_insert_with(|| empty(&self.client, self.grid.samples));
        let (blocks, threads) = elementwise(&self.client, self.grid.samples);
        levels::accumulate_batch::launch(
            &self.client,
            blocks,
            threads,
            output.into_arg(),
            sum.clone().into_arg(),
            u32::from(first),
        );
        self.frames = 0;
        self.batches = self
            .batches
            .checked_add(1)
            .ok_or("exposure batch count overflow")?;
        Ok(())
    }

    /// Traverse adjacent scales from the coarsest plane to the input grid.
    ///
    /// # Errors
    /// Returns an error if the pyramid has no input grid.
    fn reconstruct(&self) -> Result<TensorHandle, String> {
        let mut levels = self.levels.iter().enumerate().rev();
        let (_, coarsest) = levels.next().ok_or("missing source grid")?;
        let mut output = coarsest.finish(&self.client, self.method, false);
        let mut coarse_grid = coarsest.grid;
        for (index, level) in levels {
            let band = level.finish(&self.client, self.method, index < 2);
            output = reconstruct(&self.client, &band, output, level.grid, coarse_grid);
            coarse_grid = level.grid;
        }
        Ok(output)
    }
}

/// Check f32 planes plus conservative host staging and reusable kernel scratch.
///
/// # Errors
/// Returns an error if the working set exceeds the budget or host addressing.
fn validate_budget(
    grids: &[ImageShape],
    grid: ImageShape,
    method: AdvancedMethod,
    available: usize,
) -> Result<(), String> {
    let persistent_bytes_per_pixel = if method == AdvancedMethod::Hybrid {
        32
    } else {
        16
    };
    let total_persistent = grids
        .iter()
        .try_fold(0_usize, |total, level| {
            total.checked_add(level.area.checked_mul(persistent_bytes_per_pixel)?)
        })
        .ok_or("fusion persistent byte count overflow")?;
    let scratch_and_staging = grid
        .area
        .checked_mul(80)
        .ok_or("fusion scratch byte count overflow")?;
    let required = total_persistent
        .checked_add(scratch_and_staging)
        .ok_or("fusion byte count overflow")?;
    if required > available {
        return Err(format!(
            "fusion working-data budget exceeded: needs {required} bytes, have {available}"
        ));
    }
    Ok(())
}

/// Require the largest packed source or coefficient allocation to fit a page.
///
/// # Errors
/// Returns an error for an unaddressable or oversized allocation.
fn validate_buffer_limit(grid: ImageShape, limit: u64) -> Result<(), String> {
    let required = grid
        .samples
        .checked_mul(size_of::<f32>())
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or("fusion buffer bytes overflow")?;
    if required > limit {
        return Err(format!(
            "fusion device buffer needs {required} bytes, limit is {limit}"
        ));
    }
    Ok(())
}

/// Reuse one kernel specialization through all pyramid levels.
///
/// Choose the device's normal full workgroup once by policy. Every multiscale
/// kernel bounds its own tail lanes, so shrinking levels only change the number
/// of groups. Otherwise small levels repeatedly compile the same operation.
fn elementwise(client: &Client, elements: usize) -> (CubeCount, CubeDim) {
    let workgroup_limit = client.properties().hardware.max_units_per_cube;
    let full_workgroup = usize::try_from(workgroup_limit).unwrap_or(usize::MAX);
    let dimensions = CubeDim::new(client, full_workgroup);
    (
        cubecl::calculate_cube_count_elemwise(client, elements, dimensions),
        dimensions,
    )
}
