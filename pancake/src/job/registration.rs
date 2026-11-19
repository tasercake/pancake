//! Bounded reference tracking while originals stream through one at a time.

use crate::config::{AlignmentMode, AlignmentReference, JobConfig};
use crate::image_io::{RgbImage, decode};
use crate::registration::{
    Affine, Bounds, Frame, Model, ResidentRgb, Thumbnail, estimate_pair, frame_thumbnail, warp,
};

use super::{JobControl, JobError, Stage};

/// Expected geometry of one registered source.
#[derive(Clone, Copy, Debug)]
pub(super) struct SourceMap {
    /// Common-canvas map used to calculate symmetric crop.
    pub(super) common: Affine,
    /// Reference-to-source map in the original raster's pixel coordinates.
    pub(super) original: Affine,
    /// Local map before compounding previous-reference steps.
    pub(super) sequential: Affine,
}

/// Input-by-input alignment and compact reference image ownership.
#[derive(Debug)]
pub(super) struct Alignment {
    /// Method chosen by the user or disabled entirely.
    model: Option<Model>,
    /// Fixed first-frame canvas.
    canvas: Bounds,
    /// Whether each estimate updates the reference to the most recent frame.
    reference: AlignmentReference,
    /// Index of the First/Middle fixed anchor.
    anchor: usize,
    /// Thumbnail of the current fixed or previous-frame reference.
    proxy: Option<Thumbnail>,
    /// Scale bound from shared user settings.
    max_scale_step: f32,
    /// User preference for denser subpixel scoring.
    max_precision: bool,
    /// Estimation scale selected manually or derived from image dimensions.
    scale_factor: u32,
    /// Latest accumulated common-canvas transform for sequential mode.
    accumulated: Affine,
    /// Previously estimated transforms, indexed by monotonic source order.
    transforms: Vec<SourceMap>,
}

impl Alignment {
    /// Prepare the fixed middle-frame reference, when selected.
    ///
    /// # Errors
    ///
    /// Returns cancellation, dimension, decode or compute errors.
    pub(super) fn new(
        device: &cubecl::Device,
        config: &JobConfig,
        canvas: Bounds,
        scale_factor: u32,
        budget: usize,
        control: &JobControl,
    ) -> Result<Self, JobError> {
        let settings = config.settings();
        let model = match settings.alignment {
            AlignmentMode::Off => None,
            AlignmentMode::Translation => Some(Model::Translation),
            AlignmentMode::Euclidean => Some(Model::Euclidean),
            AlignmentMode::Similarity => Some(Model::Similarity),
            AlignmentMode::Affine => Some(Model::Affine),
            AlignmentMode::Breathing => Some(Model::Breathing),
        };
        let reference = settings.alignment_reference;
        let anchor = if reference == AlignmentReference::Middle && model.is_some() {
            config
                .inputs()
                .len()
                .checked_div(2)
                .ok_or("invalid middle reference")?
        } else {
            0
        };
        let proxy = if anchor == 0 {
            None
        } else {
            let path = config
                .inputs()
                .get(anchor)
                .ok_or("invalid middle reference")?;
            control.checkpoint(Stage::Decode)?;
            let image = decode(path)?;
            control.checkpoint(Stage::Register)?;
            let (proxy, _) = make_proxy(device, &image, canvas, scale_factor, budget, false)?;
            Some(proxy)
        };
        Ok(Self {
            model,
            canvas,
            reference,
            anchor,
            proxy,
            max_scale_step: settings.max_scale_step,
            max_precision: settings.max_precision,
            scale_factor,
            accumulated: Affine::IDENTITY,
            transforms: Vec::with_capacity(config.inputs().len()),
        })
    }

    /// Match a new source, or reuse the cached geometry for an overlap frame.
    ///
    /// # Errors
    ///
    /// Returns cancellation, invalid source geometry, or compute failure.
    pub(super) fn normalize(
        &mut self,
        device: &cubecl::Device,
        index: usize,
        image: RgbImage,
        budget: usize,
        control: &JobControl,
        retain_input: bool,
    ) -> Result<(Vec<f32>, Option<ResidentRgb>), JobError> {
        control.checkpoint(Stage::Register)?;
        let matching_canvas =
            image.width() == self.canvas.width() && image.height() == self.canvas.height();
        let (map, resident) = if let Some(previous) = self.transforms.get(index) {
            (*previous, None)
        } else {
            if index != self.transforms.len() {
                return Err("registration inputs were not presented in order".into());
            }
            let (new, resident) = self.estimate(
                device,
                index,
                &image,
                budget,
                control,
                retain_input && matching_canvas,
            )?;
            self.transforms.push(new);
            (new, resident)
        };
        if map.original == Affine::IDENTITY && matching_canvas {
            return Ok((image.into_samples(), resident));
        }
        drop(resident);
        control.checkpoint(Stage::Register)?;
        let samples = if self.model.is_some() {
            warp(device, frame(&image)?, map.original, self.canvas, budget)?
        } else {
            crate::fusion::resize(
                device,
                [image.width(), image.height()],
                [self.canvas.width(), self.canvas.height()],
                image.samples(),
                budget,
            )?
        };
        Ok((samples, None))
    }

    /// Include no more than one prior reference image in the working set.
    ///
    /// # Errors
    ///
    /// Returns malformed geometry or device errors.
    fn estimate(
        &mut self,
        device: &cubecl::Device,
        index: usize,
        image: &RgbImage,
        budget: usize,
        control: &JobControl,
        retain_input: bool,
    ) -> Result<(SourceMap, Option<ResidentRgb>), JobError> {
        let mut sequential = Affine::IDENTITY;
        let mut common = Affine::IDENTITY;
        let mut resident = None;
        if let Some(model) = self.model {
            if index == self.anchor && self.proxy.is_none() {
                let (proxy, input) = make_proxy(
                    device,
                    image,
                    self.canvas,
                    self.scale_factor,
                    budget,
                    retain_input,
                )?;
                self.proxy = Some(proxy);
                resident = input;
            } else if index != self.anchor {
                let (source, input) = make_proxy(
                    device,
                    image,
                    self.canvas,
                    self.scale_factor,
                    budget,
                    retain_input,
                )?;
                resident = input;
                let reference = self
                    .proxy
                    .as_ref()
                    .ok_or("registration reference missing")?;
                control.checkpoint(Stage::Register)?;
                let relative = estimate_pair(
                    device,
                    reference,
                    &source,
                    model,
                    self.max_scale_step,
                    self.max_precision,
                )?
                .transform;
                if self.reference == AlignmentReference::Previous {
                    sequential = relative;
                    common = relative.compose(self.accumulated)?;
                    self.proxy = Some(source);
                } else {
                    common = relative;
                }
            }
        }
        if self.reference != AlignmentReference::Previous && index != 0 {
            sequential = common.compose(self.accumulated.inverse()?)?;
        }
        self.accumulated = common;
        let original = common.rescale_source(
            [self.canvas.width(), self.canvas.height()],
            [image.width(), image.height()],
        )?;
        Ok((
            SourceMap {
                common,
                original,
                sequential,
            },
            resident,
        ))
    }

    /// Release compact reference data and take the geometry for result export.
    pub(super) fn into_transforms(self) -> Vec<SourceMap> {
        self.transforms
    }
}

/// Create a thumbnail at the common reference scale, without retaining a full frame.
///
/// # Errors
///
/// Returns checked geometry or registration resource errors.
fn make_proxy(
    device: &cubecl::Device,
    image: &RgbImage,
    canvas: Bounds,
    factor: u32,
    budget: usize,
    retain_input: bool,
) -> Result<(Thumbnail, Option<ResidentRgb>), String> {
    frame_thumbnail(
        device,
        frame(image)?,
        [canvas.width(), canvas.height()],
        factor,
        budget,
        retain_input,
    )
}

/// Borrow RGB for the registration package's checked entry point.
///
/// # Errors
///
/// Returns a dimension/count validation error.
fn frame(image: &RgbImage) -> Result<Frame<'_>, String> {
    Frame::new(image.width(), image.height(), image.samples())
}
