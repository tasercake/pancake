//! Shared stack settings and validated immutable job requests.

use std::path::{Path, PathBuf};
use std::str::FromStr;

mod paths;

#[cfg(test)]
mod tests;

/// The algorithm that combines an ordered series of input frames.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Method {
    /// Laplacian pyramid fusion.
    Laplacian,
    /// Weighted fusion refined by Laplacian detail.
    #[default]
    Hybrid,
    /// Per-pixel focus-weighted average.
    WeightedAverage,
    /// Focus selection regularized by depth.
    DepthMap,
    /// Exposure fusion of a bracketed sequence.
    ExposureFusion,
}

impl FromStr for Method {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "laplacian" => Ok(Self::Laplacian),
            "hybrid" => Ok(Self::Hybrid),
            "weighted_average" => Ok(Self::WeightedAverage),
            "depth_map" => Ok(Self::DepthMap),
            "exposure_fusion" => Ok(Self::ExposureFusion),
            _ => Err("expected laplacian, hybrid, weighted_average, depth_map, or exposure_fusion"),
        }
    }
}

/// Registration model selected by the user.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignmentMode {
    /// Leave frames unregistered.
    #[default]
    Off,
    /// Register translation along both axes.
    Translation,
    /// Register translation and rotation.
    Euclidean,
    /// Register similarity transforms, including uniform scale.
    Similarity,
    /// Register general affine transforms.
    Affine,
    /// Register focus breathing between adjacent frames.
    Breathing,
}

impl FromStr for AlignmentMode {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "off" => Ok(Self::Off),
            "translation" => Ok(Self::Translation),
            "euclidean" => Ok(Self::Euclidean),
            "similarity" => Ok(Self::Similarity),
            "affine" => Ok(Self::Affine),
            "breathing" => Ok(Self::Breathing),
            _ => Err("expected off, translation, euclidean, similarity, affine, or breathing"),
        }
    }
}

/// Frame used when estimating registration transforms.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AlignmentReference {
    /// Register each frame to the first source.
    #[default]
    First,
    /// Register each frame to the middle source.
    Middle,
    /// Register against the previous frame and compose the results.
    Previous,
}

impl FromStr for AlignmentReference {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "first" => Ok(Self::First),
            "middle" => Ok(Self::Middle),
            "previous" => Ok(Self::Previous),
            _ => Err("expected first, middle, or previous"),
        }
    }
}

/// Requested backend class, before runtime discovery.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DevicePreference {
    /// Choose a qualified device within the resource budget.
    #[default]
    Auto,
    /// Run without GPU compute.
    Cpu,
    /// Request GPU compute.
    Gpu,
}

impl FromStr for DevicePreference {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            _ => Err("expected auto, cpu, or gpu"),
        }
    }
}

/// Requested integer export precision for formats that support it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BitDepth {
    /// Eight bits per channel.
    #[default]
    Eight,
    /// Sixteen bits per channel.
    Sixteen,
}

impl FromStr for BitDepth {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "8" => Ok(Self::Eight),
            "16" => Ok(Self::Sixteen),
            _ => Err("expected 8 or 16"),
        }
    }
}

/// Editable stack settings. Validate once before submitting a processing job.
///
/// Image and working-memory limits are applied separately when dimensions and
/// backend resources become known.
#[derive(Clone, Copy, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Each boolean is an independent option shared by the two frontends"
)]
pub struct JobSettings {
    /// Resolve the ordered inputs with this stacking algorithm.
    pub method: Method,
    /// Geometric model used for registration.
    pub alignment: AlignmentMode,
    /// Reference frame selection.
    pub alignment_reference: AlignmentReference,
    /// Focus measurement window diameter in pixels.
    pub kernel_size: u32,
    /// Number of pyramid levels requested.
    pub pyramid_levels: u32,
    /// Registration downsampling factor.
    pub scale_factor: u32,
    /// Maximum positive fractional scale change between frames.
    pub max_scale_step: f32,
    /// Request high-precision registration.
    pub max_precision: bool,
    /// Frames per slab, or zero for whole-stack processing.
    pub slab_size: u32,
    /// Shared frames in consecutive slabs.
    pub slab_overlap: u32,
    /// Select focus and pyramid settings using input dimensions.
    pub auto: bool,
    /// Crop to the registered stack's usable area.
    pub auto_crop: bool,
    /// Lossy output quality, as a percentage.
    pub quality: u8,
    /// Report output sharpness after completion.
    pub quality_report: bool,
    /// Integer export quantization.
    pub bit_depth: BitDepth,
    /// Backend preference for numerical processing.
    pub device: DevicePreference,
    /// Optional identifier of a particular GPU.
    pub gpu_id: Option<usize>,
}

impl Default for JobSettings {
    fn default() -> Self {
        Self {
            method: Method::default(),
            alignment: AlignmentMode::default(),
            alignment_reference: AlignmentReference::default(),
            kernel_size: 3,
            pyramid_levels: 4,
            scale_factor: 6,
            max_scale_step: 0.05,
            max_precision: false,
            slab_size: 0,
            slab_overlap: 0,
            auto: false,
            auto_crop: false,
            quality: 95,
            quality_report: false,
            bit_depth: BitDepth::default(),
            device: DevicePreference::default(),
            gpu_id: None,
        }
    }
}

impl JobSettings {
    /// Verify processing and export settings before opening any files.
    ///
    /// # Errors
    ///
    /// Returns a diagnostic naming the invalid option and its current value.
    pub fn validate(&self) -> Result<(), String> {
        for (flag, value, max) in [
            ("--kernel-size", self.kernel_size, 63),
            ("--pyramid-levels", self.pyramid_levels, u32::BITS),
            ("--scale-factor", self.scale_factor, 128),
            ("--quality", u32::from(self.quality), 100),
        ] {
            if !(1..=max).contains(&value) {
                return Err(format!("{flag} must be between 1 and {max}; got {value}"));
            }
        }
        // Both comparisons reject NaN as well as signed and unsigned infinities.
        if !(self.max_scale_step > 0.0 && self.max_scale_step < 1.0) {
            return Err(format!(
                "--max-scale-step must be greater than 0 and less than 1; got {}",
                self.max_scale_step
            ));
        }
        if self.slab_size == 1 || self.slab_size > paths::MAX_INPUT_COUNT {
            return Err(format!(
                "--slab-size must be 0 or between 2 and {}; got {}",
                paths::MAX_INPUT_COUNT,
                self.slab_size
            ));
        }
        if self.slab_overlap > 0 && self.slab_overlap >= self.slab_size {
            return Err(format!(
                "--slab-overlap {} must be smaller than --slab-size {}",
                self.slab_overlap, self.slab_size
            ));
        }
        if self.device == DevicePreference::Cpu && self.gpu_id.is_some() {
            return Err("--gpu-id conflicts with CPU device selection".into());
        }
        Ok(())
    }
}

/// Immutable request with validated settings and ordered unique input paths.
#[derive(Debug)]
pub struct JobConfig {
    /// Canonical paths in sorted order.
    inputs: Vec<PathBuf>,
    /// Optional destination for an immediate export.
    output: Option<PathBuf>,
    /// Validated processing and export settings.
    settings: JobSettings,
}

impl JobConfig {
    /// Validate options and resolve literal paths and glob inputs.
    ///
    /// A missing output destination requests an in-memory result.
    ///
    /// # Errors
    ///
    /// Returns invalid settings, unresolved paths, input count violations or
    /// an error when the output destination addresses an input file.
    pub fn new(
        input_patterns: &[PathBuf],
        output: Option<PathBuf>,
        settings: JobSettings,
    ) -> Result<Self, String> {
        settings.validate()?;
        let inputs = paths::resolve(input_patterns)?;
        if let Some(destination) = &output
            && let Ok(destination) = destination.canonicalize()
            && inputs.binary_search(&destination).is_ok()
        {
            return Err(format!("output {} is also an input", destination.display()));
        }
        Ok(Self {
            inputs,
            output,
            settings,
        })
    }

    /// Sorted canonical paths of the unique input files.
    #[must_use]
    pub fn inputs(&self) -> &[PathBuf] {
        &self.inputs
    }

    /// The export destination, if supplied.
    #[must_use]
    pub fn output(&self) -> Option<&Path> {
        self.output.as_deref()
    }

    /// The validated stack settings.
    #[must_use]
    pub const fn settings(&self) -> &JobSettings {
        &self.settings
    }
}
