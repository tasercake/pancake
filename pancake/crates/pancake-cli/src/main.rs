//! Headless Pancake argument parsing and job execution.

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use pancake::config::{
    AlignmentMode, AlignmentReference, BitDepth, DevicePreference, JobConfig, JobSettings, Method,
};

/// Process a focal stack or exposure bracket.
#[derive(Debug, Parser)]
#[command(version, name = "pancake-cli")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Each boolean represents an independent command-line switch"
)]
struct Arguments {
    /// Input files or quoted glob patterns. Repeat to add more inputs.
    #[arg(long, required = true, num_args = 1.., value_name = "PATH")]
    input: Vec<PathBuf>,
    /// Export destination; the extension selects the image format.
    #[arg(long, value_name = "PATH")]
    output: PathBuf,
    /// Combine with laplacian, hybrid, weighted_average, depth_map, or exposure_fusion.
    #[arg(long, value_name = "METHOD")]
    method: Option<Method>,
    /// Align inputs with a translation model unless a model is supplied.
    #[arg(long)]
    align: bool,
    /// Select off, translation, euclidean, similarity, affine, or breathing.
    #[arg(long, value_name = "MODE")]
    alignment_mode: Option<AlignmentMode>,
    /// Reference frame: first, middle, or previous.
    #[arg(long = "alignment-ref", value_name = "FRAME")]
    alignment_reference: Option<AlignmentReference>,
    /// Focus measurement window size.
    #[arg(long, value_name = "PIXELS")]
    kernel_size: Option<u32>,
    /// Number of levels in the focus pyramid.
    #[arg(long, value_name = "LEVELS")]
    pyramid_levels: Option<u32>,
    /// Registration downsampling factor.
    #[arg(long, value_name = "FACTOR")]
    scale_factor: Option<u32>,
    /// Maximum positive fractional scale change between frames.
    #[arg(long, allow_negative_numbers = true, value_name = "FRACTION")]
    max_scale_step: Option<f32>,
    /// Request high-precision registration.
    #[arg(long)]
    max_precision: bool,
    /// Process the requested number of inputs per slab; zero uses the whole stack.
    #[arg(long, allow_negative_numbers = true, value_name = "FRAMES")]
    slab_size: Option<u32>,
    /// Neighboring slabs share this many inputs.
    #[arg(long, allow_negative_numbers = true, value_name = "FRAMES")]
    slab_overlap: Option<u32>,
    /// Choose focus and pyramid parameters from the input dimensions.
    #[arg(long)]
    auto: bool,
    /// Crop to the usable area of the registered stack.
    #[arg(long)]
    auto_crop: bool,
    /// Lossy output quality, from 1 to 100.
    #[arg(long, value_name = "PERCENT")]
    quality: Option<u8>,
    /// Emit a sharpness report for a completed result.
    #[arg(long)]
    quality_report: bool,
    /// Export 8 or 16 bits per integer color channel where supported.
    #[arg(long, value_name = "BITS")]
    bit_depth: Option<BitDepth>,
    /// Run all numerical work on the CPU.
    #[arg(long, conflicts_with_all = ["gpu", "device"])]
    cpu: bool,
    /// Run numerical work on a GPU.
    #[arg(long, conflicts_with = "device")]
    gpu: bool,
    /// Device preference: cpu, gpu, or auto.
    #[arg(long, value_name = "DEVICE")]
    device: Option<DevicePreference>,
    /// Select a specific GPU.
    #[arg(long, value_name = "INDEX")]
    gpu_id: Option<usize>,
    /// Use the similarity model as the default when aligning.
    #[arg(long)]
    rotation_scale: bool,
}

impl Arguments {
    /// Normalize command-line switches to the shared job settings.
    fn into_job(self) -> Result<JobConfig, String> {
        let defaults = JobSettings::default();
        let alignment = self.alignment_mode.unwrap_or_else(|| {
            if self.rotation_scale {
                AlignmentMode::Similarity
            } else if self.align {
                AlignmentMode::Translation
            } else {
                defaults.alignment
            }
        });
        let device = if self.cpu {
            DevicePreference::Cpu
        } else if self.gpu {
            DevicePreference::Gpu
        } else {
            self.device.unwrap_or(defaults.device)
        };
        let settings = JobSettings {
            method: self.method.unwrap_or(defaults.method),
            alignment,
            alignment_reference: self
                .alignment_reference
                .unwrap_or(defaults.alignment_reference),
            kernel_size: self.kernel_size.unwrap_or(defaults.kernel_size),
            pyramid_levels: self.pyramid_levels.unwrap_or(defaults.pyramid_levels),
            scale_factor: self.scale_factor.unwrap_or(defaults.scale_factor),
            max_scale_step: self.max_scale_step.unwrap_or(defaults.max_scale_step),
            max_precision: self.max_precision,
            slab_size: self.slab_size.unwrap_or(defaults.slab_size),
            slab_overlap: self.slab_overlap.unwrap_or(defaults.slab_overlap),
            auto: self.auto,
            auto_crop: self.auto_crop,
            quality: self.quality.unwrap_or(defaults.quality),
            quality_report: self.quality_report,
            bit_depth: self.bit_depth.unwrap_or(defaults.bit_depth),
            device,
            gpu_id: self.gpu_id,
        };
        JobConfig::new(&self.input, Some(self.output), settings)
    }
}

/// Write an error without unwinding on a disconnected output stream.
fn fail(error: &str, status: ExitCode) -> ExitCode {
    if writeln!(io::stderr().lock(), "error: {error}").is_err() {
        return ExitCode::FAILURE;
    }
    status
}

/// Parse arguments and reject incomplete jobs without publishing output.
fn main() -> ExitCode {
    const BAD_ARGUMENTS: ExitCode = ExitCode::from(2);
    let arguments = match Arguments::try_parse() {
        Ok(arguments) => arguments,
        Err(error) => {
            if error.print().is_err() {
                return ExitCode::FAILURE;
            }
            return if error.use_stderr() {
                BAD_ARGUMENTS
            } else {
                ExitCode::SUCCESS
            };
        }
    };
    let _job = match arguments.into_job() {
        Ok(job) => job,
        Err(error) => return fail(&error, BAD_ARGUMENTS),
    };
    fail(
        "stack processing is not available in this build",
        ExitCode::FAILURE,
    )
}
