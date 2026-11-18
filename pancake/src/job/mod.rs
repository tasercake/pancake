//! Shared stack execution, lifecycle, and publication for both frontends.

mod events;
mod lifecycle;
mod registration;
mod stream;
mod worker;

use std::fmt::{self, Display};
use std::path::PathBuf;

use crate::config::{DevicePreference, JobConfig, JobSettings};
use crate::image_io::{RgbImage, decode, prepare_export};
use crate::registration::{Affine, Bounds};
use crate::runtime::{Backend, RuntimeOverride, SelectedRuntime, select_runtime};

pub use lifecycle::{Generation, JobControl, JobSnapshot, JobState, Stage, Stop};
pub use worker::StackWorker;

/// Default logical working-data cap for one full stack.
///
/// A full stack can hold a registration batch and fusion statistics at once;
/// the smaller primitive budget does not describe those combined resources.
const JOB_WORKING_BYTES: usize = 4 * 1024 * 1024 * 1024;

/// Invocation settings shared by synchronous and background execution.
#[derive(Clone, Debug)]
pub struct Options {
    /// Total checked logical working-data budget for one request.
    pub working_bytes: usize,
    /// Explicit backend runtime override when qualifying a GPU path.
    pub runtime: Option<RuntimeOverride>,
    /// Physical, qualified wgpu runtime shared with an existing renderer.
    pub prepared_wgpu: Option<SelectedRuntime>,
    /// Optional case-owned JSON Lines diagnostics destination.
    pub events_path: Option<PathBuf>,
}

impl Default for Options {
    /// Leave normal device policy enabled and observation disabled.
    fn default() -> Self {
        Self {
            working_bytes: JOB_WORKING_BYTES,
            runtime: None,
            prepared_wgpu: None,
            events_path: None,
        }
    }
}

impl Options {
    /// Read application-level runtime and diagnostics environment variables.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic for an unknown or malformed runtime override.
    pub fn from_env() -> Result<Self, String> {
        let runtime = std::env::var_os("PANCAKE_RUNTIME")
            .map(|value| {
                let name = value
                    .to_str()
                    .ok_or("PANCAKE_RUNTIME must be valid UTF-8")?;
                name.parse()
            })
            .transpose()?;
        Ok(Self {
            runtime,
            events_path: std::env::var_os("CONFORMANCE_EVENTS_PATH").map(PathBuf::from),
            ..Self::default()
        })
    }
}

/// Completed computation and immutable result data, independent of the preview.
#[derive(Debug)]
pub struct StackResult {
    /// Full-precision output in its actual cropped or uncropped canvas.
    image: RgbImage,
    /// Ordered original sources and validated settings for this result.
    config: JobConfig,
    /// Full first-frame coordinate space.
    canvas: Bounds,
    /// Exact rectangle used for the returned image.
    crop: Bounds,
    /// Maximum valid symmetric crop in the original canvas, when nonempty.
    safe_crop: Option<Bounds>,
    /// Registration transforms keyed by the ordered sources.
    transforms: Vec<registration::SourceMap>,
    /// Runtime that actually completed the numerical stages.
    backend: Backend,
    /// Optional rendered result sharpness.
    sharpness: Option<f32>,
    /// Nonfatal format/observer diagnostics for the caller.
    warnings: Vec<String>,
}

impl StackResult {
    /// Read the immutable full-precision result.
    #[must_use]
    pub const fn image(&self) -> &RgbImage {
        &self.image
    }

    /// Read the ordered originals used in the job.
    #[must_use]
    pub fn sources(&self) -> &[PathBuf] {
        self.config.inputs()
    }

    /// Read the validated settings used by this job.
    #[must_use]
    pub const fn settings(&self) -> &JobSettings {
        self.config.settings()
    }

    /// Access the uncropped first-frame coordinate system.
    #[must_use]
    pub const fn canvas(&self) -> Bounds {
        self.canvas
    }

    /// Exact bounds of the retained image in the first-frame coordinate space.
    #[must_use]
    pub const fn crop(&self) -> Bounds {
        self.crop
    }

    /// Symmetric valid crop for native manual crop controls.
    #[must_use]
    pub const fn safe_crop(&self) -> Option<Bounds> {
        self.safe_crop
    }

    /// Produce the exact valid crop from an immutable completed result.
    ///
    /// Returns no image if a safe crop is unavailable or already represented.
    ///
    /// # Errors
    ///
    /// Returns invalid result geometry, allocation or sample errors.
    pub fn manually_cropped(&self) -> Result<Option<RgbImage>, String> {
        self.safe_crop
            .filter(|&crop| crop != self.crop)
            .map(|crop| stream::copy_crop(&self.image, self.crop, crop))
            .transpose()
    }

    /// Map a result-canvas pixel to the indexed original source.
    #[must_use]
    pub fn source_transform(&self, index: usize) -> Option<Affine> {
        self.transforms.get(index).map(|map| map.original)
    }

    /// Actual completed execution class.
    #[must_use]
    pub const fn backend(&self) -> Backend {
        self.backend
    }

    /// Numerical sharpness measured on the completed retained image if enabled.
    #[must_use]
    pub const fn sharpness(&self) -> Option<f32> {
        self.sharpness
    }

    /// Nonfatal diagnostics accumulated after numerical processing.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }
}

/// Recoverable job outcome at either frontend.
#[derive(Debug)]
pub enum JobError {
    /// Cancellation won a safe boundary before result publication.
    Cancelled,
    /// A complete result could not be produced or published.
    Failed(String),
}

impl Display for JobError {
    /// Return the cancellation acknowledgement or underlying job diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("stack cancelled"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for JobError {}

impl From<String> for JobError {
    /// Retain a recoverable I/O or compute diagnostic.
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

impl From<&str> for JobError {
    /// Retain a static validation diagnostic.
    fn from(message: &str) -> Self {
        Self::Failed(message.to_owned())
    }
}

impl From<Stop> for JobError {
    /// Distinguish a user stop from an illegal lifecycle transition.
    fn from(reason: Stop) -> Self {
        match reason {
            Stop::Cancelled => Self::Cancelled,
            Stop::Closed | Stop::InvalidState => Self::Failed(reason.to_string()),
        }
    }
}

/// Run one complete stack in the calling thread.
///
/// The callback runs after each new frame is successfully accumulated.
///
/// # Errors
///
/// Returns a source, runtime, numerical or export error.
pub fn run(
    config: JobConfig,
    options: &Options,
    progress: impl FnMut(JobSnapshot),
) -> Result<StackResult, JobError> {
    let control = JobControl::new(Generation::from_counter(1), config.inputs().len())?;
    run_controlled(config, options, &control, progress)
}

/// Run one complete stack using an externally owned control ticket.
///
/// Each checkpoint observes controls before admitting new bounded work. No
/// failure or cancelled job emits successful completion or publishes a staged
/// export. Retained results are transferred to the caller only after completion.
///
/// # Errors
///
/// Returns cancellation or a validation, source, runtime or output error.
pub fn run_controlled(
    config: JobConfig,
    options: &Options,
    control: &JobControl,
    progress: impl FnMut(JobSnapshot),
) -> Result<StackResult, JobError> {
    let outcome = execute(config, options, control, progress);
    match &outcome {
        Ok(_) => control.finish(true)?,
        Err(JobError::Cancelled) => control.settle_cancelled()?,
        Err(JobError::Failed(_)) => control.finish(false)?,
    }
    outcome
}

/// Process originals in order before staging and committing any result output.
///
/// # Errors
///
/// Returns cancellation or a decode, compute, configuration, or output error.
fn execute(
    config: JobConfig,
    options: &Options,
    control: &JobControl,
    progress: impl FnMut(JobSnapshot),
) -> Result<StackResult, JobError> {
    control.checkpoint(Stage::Decode)?;
    let initial = config
        .inputs()
        .first()
        .ok_or("stack requires a first input")?;
    let first = decode(initial)?;
    control.checkpoint(Stage::Prepare)?;
    let runtime = choose_runtime(config.settings(), options)?;
    let product = stream::process(
        &config,
        first,
        &runtime,
        options.working_bytes,
        control,
        progress,
    )?;
    runtime.synchronize().map_err(|error| error.to_string())?;

    let settings = *config.settings();
    let staged = if let Some(path) = config.output() {
        control.checkpoint(Stage::Encode)?;
        Some(prepare_export(
            path,
            &product.image,
            settings.bit_depth,
            settings.quality,
            config.inputs(),
        )?)
    } else {
        None
    };
    control.commit()?;
    let mut warnings = Vec::new();
    if let Some(file) = staged {
        if let Some(warning) = file.warning() {
            warnings.push(warning.to_owned());
        }
        file.publish()?;
    }
    if let Some(path) = &options.events_path
        && let Err(error) = events::completion(path, &config, &runtime, &product.transforms)
    {
        warnings.push(error);
    }

    Ok(StackResult {
        image: product.image,
        config,
        canvas: product.canvas,
        crop: product.crop,
        safe_crop: product.safe_crop,
        transforms: product.transforms,
        backend: runtime.backend(),
        sharpness: product.sharpness,
        warnings,
    })
}

/// Preserve explicit CPU selection and reuse a compatible renderer's runtime.
///
/// # Errors
///
/// Returns a runtime setup or selection error.
fn choose_runtime(settings: &JobSettings, options: &Options) -> Result<SelectedRuntime, String> {
    if settings.device != DevicePreference::Cpu
        && settings.gpu_id.is_none()
        && options
            .runtime
            .is_none_or(|value| value == RuntimeOverride::Wgpu)
        && let Some(prepared) = &options.prepared_wgpu
        && prepared.physical_gpu()
        && prepared.wgpu_setup().is_some()
    {
        return Ok(prepared.clone());
    }
    select_runtime(
        settings.device,
        settings.gpu_id,
        options.working_bytes,
        options.runtime,
    )
}
