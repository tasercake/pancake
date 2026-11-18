//! Bounded streaming from original files to one full-precision numerical result.

use std::ops::Range;

use crate::config::{JobConfig, JobSettings, Method};
use crate::fusion::advanced::{
    AdvancedAccumulator, AdvancedMethod, automatic_parameters, slab_ranges,
};
use crate::fusion::{FocusAccumulator, FocusMethod, sharpness};
use crate::image_io::{RgbImage, decode};
use crate::registration::{Bounds, symmetric_crop};
use crate::runtime::SelectedRuntime;

use super::registration::{Alignment, SourceMap};
use super::{JobControl, JobError, JobSnapshot, Stage};

#[cfg(test)]
mod tests;

/// Geometry, budget partitions, and derived parameters for a stack.
#[derive(Debug)]
struct Plan {
    /// Common first-frame dimensions and zero crop origin.
    canvas: Bounds,
    /// Slab input ranges, or one original range without slabbing.
    slabs: Vec<Range<usize>>,
    /// Effective focus measurement window width.
    kernel_size: usize,
    /// Effective pyramid depth for multiscale methods.
    pyramid_levels: usize,
    /// Effective registration proxy scale factor.
    scale_factor: u32,
    /// Budget for one active accumulator, including its submitted frame.
    accumulator_bytes: usize,
    /// Budget for source normalization and registration staging.
    registration_bytes: usize,
    /// Full logical budget after active accumulators have been released.
    working_bytes: usize,
}

impl Plan {
    /// Account for simultaneous registration, current slab and final statistics.
    ///
    /// # Errors
    ///
    /// Returns dimension, parameter, or addressability errors.
    fn new(config: &JobConfig, first: &RgbImage, bytes: usize) -> Result<Self, String> {
        let [width, height] = [first.width(), first.height()];
        let count = config.inputs().len();
        let settings = config.settings();
        let (kernel_size, pyramid_levels, scale_factor) = if settings.auto {
            let auto = automatic_parameters(width, height, count)?;
            (
                auto.kernel_size,
                auto.pyramid_levels,
                u32::try_from(auto.alignment_scale_factor).map_err(|error| error.to_string())?,
            )
        } else {
            (
                to_usize(settings.kernel_size)?,
                to_usize(settings.pyramid_levels)?,
                settings.scale_factor,
            )
        };
        let slabs = slab_ranges(
            count,
            if settings.method == Method::ExposureFusion {
                0
            } else {
                to_usize(settings.slab_size)?
            },
            to_usize(settings.slab_overlap)?,
        )?;
        let registration_bytes = bytes.div_ceil(4);
        let fusion_bytes = bytes.saturating_sub(registration_bytes);
        let accumulator_bytes = if slabs.len() > 1 {
            fusion_bytes.checked_div(2).ok_or("invalid slab budget")?
        } else {
            fusion_bytes
        };
        Ok(Self {
            canvas: Bounds::new(0, 0, width, height)?,
            slabs,
            kernel_size,
            pyramid_levels,
            scale_factor,
            accumulator_bytes,
            registration_bytes,
            working_bytes: bytes,
        })
    }
}

/// A method-specific accumulation state using the same selected runtime.
#[derive(Debug)]
enum Accumulator {
    /// Per-pixel focus weights or the strongest focus source.
    Focus(Box<FocusAccumulator>),
    /// Multiscale Laplacian, Hybrid, or four-frame exposure batches.
    Advanced(Box<AdvancedAccumulator>),
}

impl Accumulator {
    /// Select one of the shared numerical implementations.
    ///
    /// # Errors
    ///
    /// Returns shape, settings or budget validation errors.
    fn new(device: &cubecl::Device, plan: &Plan, method: Method) -> Result<Self, String> {
        let constructor = |algorithm| {
            AdvancedAccumulator::new(
                device,
                plan.canvas.width(),
                plan.canvas.height(),
                algorithm,
                plan.kernel_size,
                plan.pyramid_levels,
                plan.accumulator_bytes,
            )
            .map(|advanced| Self::Advanced(Box::new(advanced)))
        };
        let focus = match method {
            Method::Laplacian => return constructor(AdvancedMethod::Laplacian),
            Method::Hybrid => return constructor(AdvancedMethod::Hybrid),
            Method::ExposureFusion => return constructor(AdvancedMethod::Exposure),
            Method::WeightedAverage => FocusMethod::WeightedAverage,
            Method::DepthMap => FocusMethod::DepthMap,
        };
        FocusAccumulator::new(
            device,
            plan.canvas.width(),
            plan.canvas.height(),
            focus,
            u32::try_from(plan.kernel_size).map_err(|error| error.to_string())?,
            plan.accumulator_bytes,
        )
        .map(|focus| Self::Focus(Box::new(focus)))
    }

    /// Complete one real, synchronized device batch.
    ///
    /// # Errors
    ///
    /// Returns invalid sample or device completion errors.
    fn add_frame(&mut self, samples: &[f32]) -> Result<(), String> {
        match self {
            Self::Focus(accumulator) => accumulator.add_frame(samples),
            Self::Advanced(accumulator) => accumulator.add_frame(samples),
        }
    }

    /// Read back and relinquish the active device result.
    ///
    /// # Errors
    ///
    /// Returns numerical or device completion errors.
    fn finish(self) -> Result<Vec<f32>, String> {
        match self {
            Self::Focus(accumulator) => (*accumulator).finish(),
            Self::Advanced(accumulator) => (*accumulator).finish(),
        }
    }
}

/// Successful pure processing before external publication.
#[derive(Debug)]
pub(super) struct Product {
    /// Full-precision result after optional crop.
    pub(super) image: RgbImage,
    /// Canvas shared by all normalized images.
    pub(super) canvas: Bounds,
    /// Bounds of the retained result within the original canvas.
    pub(super) crop: Bounds,
    /// Symmetric crop usable by native manual crop tools.
    pub(super) safe_crop: Option<Bounds>,
    /// Input-ordered reference-to-source matrix evidence.
    pub(super) transforms: Vec<SourceMap>,
    /// Real focus measurement computed on the completed output if requested.
    pub(super) sharpness: Option<f32>,
}

/// Stream original sources, keeping only reference proxies and one active frame.
///
/// # Errors
///
/// Returns cancellation, file, shape, numerical or resource failures.
pub(super) fn process(
    config: &JobConfig,
    first: RgbImage,
    runtime: &SelectedRuntime,
    bytes: usize,
    control: &JobControl,
    mut progress: impl FnMut(JobSnapshot),
) -> Result<Product, JobError> {
    let device = runtime.device();
    let plan = Plan::new(config, &first, bytes)?;
    let settings = config.settings();
    let mut alignment = Alignment::new(
        device,
        config,
        plan.canvas,
        plan.scale_factor,
        plan.registration_bytes,
        control,
    )?;
    let mut final_accumulator = if plan.slabs.len() > 1 {
        control.checkpoint(Stage::Fuse)?;
        Some(Accumulator::new(device, &plan, settings.method)?)
    } else {
        None
    };
    let mut unused_first = Some(first);
    let mut grayscale_array = true;
    let mut single_slab_result = None;
    for slab in &plan.slabs {
        control.checkpoint(Stage::Fuse)?;
        let mut accumulator = Accumulator::new(device, &plan, settings.method)?;
        for index in slab.clone() {
            control.checkpoint(Stage::Decode)?;
            let image = if let Some(first) = unused_first.take() {
                first
            } else {
                decode(
                    config
                        .inputs()
                        .get(index)
                        .ok_or("stack input index out of range")?,
                )?
            };
            grayscale_array &= image.is_grayscale_array();
            let pixels =
                alignment.normalize(device, index, image, plan.registration_bytes, control)?;
            control.checkpoint(Stage::Fuse)?;
            accumulator.add_frame(&pixels)?;
            drop(pixels);
            if index == control.snapshot()?.completed {
                control.complete_frame()?;
                progress(control.snapshot()?);
            }
        }
        control.checkpoint(Stage::Fuse)?;
        let result = accumulator.finish()?;
        if let Some(accumulator) = &mut final_accumulator {
            control.checkpoint(Stage::Fuse)?;
            accumulator.add_frame(&result)?;
        } else {
            single_slab_result = Some(result);
        }
    }
    control.checkpoint(Stage::Fuse)?;
    let samples = if let Some(accumulator) = final_accumulator {
        accumulator.finish()?
    } else {
        single_slab_result.ok_or("stack produced no fusion result")?
    };
    let mut product = finish(
        device,
        &plan,
        alignment.into_transforms(),
        samples,
        settings,
        control,
    )?;
    if grayscale_array {
        product.image.mark_grayscale_array();
    }
    Ok(product)
}

/// Retain geometry and crop the actual full-precision readback if requested.
///
/// # Errors
///
/// Returns invalid output or crop geometry and failed quality measurements.
fn finish(
    device: &cubecl::Device,
    plan: &Plan,
    transforms: Vec<SourceMap>,
    mut samples: Vec<f32>,
    settings: &JobSettings,
    control: &JobControl,
) -> Result<Product, JobError> {
    let canvas = plan.canvas;
    let common = transforms.iter().map(|map| map.common).collect::<Vec<_>>();
    let valid = symmetric_crop(canvas.width(), canvas.height(), &common);
    let crop = if settings.auto_crop {
        let bounds = *valid.as_ref().map_err(String::clone)?;
        if bounds != canvas {
            crop_samples(&mut samples, canvas, bounds)?;
        }
        bounds
    } else {
        canvas
    };
    let image = RgbImage::new(crop.width(), crop.height(), samples)?;
    let quality = if settings.quality_report {
        control.checkpoint(Stage::Fuse)?;
        Some(sharpness(
            device,
            image.width(),
            image.height(),
            image.samples(),
            plan.working_bytes,
        )?)
    } else {
        None
    };
    Ok(Product {
        image,
        canvas,
        crop,
        safe_crop: valid.ok(),
        transforms,
        sharpness: quality,
    })
}

/// Compact an RGB result to an exact rectangular subset in place.
///
/// # Errors
///
/// Returns invalid geometry or address overflow.
fn crop_samples(samples: &mut Vec<f32>, old: Bounds, crop: Bounds) -> Result<(), String> {
    let row_width = rgb_stride(crop.width())?;
    for (target_row, range) in crop_rows(old, crop, samples.len())?.enumerate() {
        let target = target_row
            .checked_mul(row_width)
            .ok_or("crop destination overflow")?;
        samples.copy_within(range?, target);
    }
    let len = crop
        .height()
        .checked_mul(row_width)
        .ok_or("crop destination size overflow")?;
    samples.truncate(len);
    Ok(())
}

/// Copy only the selected reference rectangle, preserving the original raster.
///
/// # Errors
///
/// Returns invalid geometry, allocation failure or invalid samples.
pub(super) fn copy_crop(image: &RgbImage, old: Bounds, crop: Bounds) -> Result<RgbImage, String> {
    let original = image.samples();
    let mut samples = Vec::new();
    let count = rgb_stride(crop.width())?
        .checked_mul(crop.height())
        .ok_or("crop destination size overflow")?;
    samples
        .try_reserve_exact(count)
        .map_err(|error| error.to_string())?;
    for range in crop_rows(old, crop, original.len())? {
        samples.extend_from_slice(original.get(range?).ok_or("crop row exceeded its input")?);
    }
    let mut cropped = RgbImage::new(crop.width(), crop.height(), samples)?;
    if image.is_grayscale_array() {
        cropped.mark_grayscale_array();
    }
    Ok(cropped)
}

/// Resolve exact RGB sample rows for absolute reference-space bounds.
///
/// # Errors
///
/// Returns invalid geometry or address overflow rather than clipping a crop.
fn crop_rows(
    old: Bounds,
    crop: Bounds,
    sample_count: usize,
) -> Result<impl Iterator<Item = Result<Range<usize>, String>>, String> {
    let stride = rgb_stride(old.width())?;
    let row_width = rgb_stride(crop.width())?;
    let column = crop
        .x()
        .checked_sub(old.x())
        .ok_or("crop precedes its source column")?;
    let offset = rgb_stride(column)?;
    let source_y = crop
        .y()
        .checked_sub(old.y())
        .ok_or("crop precedes its source row")?;
    let source_y_end = source_y
        .checked_add(crop.height())
        .ok_or("crop y overflow")?;
    let row_end = offset.checked_add(row_width).ok_or("crop x overflow")?;
    if source_y_end > old.height() || row_end > stride {
        return Err("crop extends outside the completed result".to_owned());
    }

    Ok((source_y..source_y_end).map(move |source_row| {
        let start = source_row
            .checked_mul(stride)
            .and_then(|row| row.checked_add(offset))
            .ok_or("crop source overflow")?;
        let end = start
            .checked_add(row_width)
            .filter(|&end| end <= sample_count)
            .ok_or("crop source exceeds the complete raster")?;
        Ok(start..end)
    }))
}

/// Address checked interleaved RGB samples.
///
/// # Errors
///
/// Returns integer overflow.
fn rgb_stride(width: usize) -> Result<usize, String> {
    width
        .checked_mul(3)
        .ok_or_else(|| "RGB crop stride overflow".to_owned())
}

/// Convert a validated u32 setting to a host index.
///
/// # Errors
///
/// Returns host address-space overflow.
fn to_usize(value: u32) -> Result<usize, String> {
    usize::try_from(value).map_err(|error| error.to_string())
}
