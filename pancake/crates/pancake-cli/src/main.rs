//! Headless Pancake argument parsing and job execution.

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use pancake::config::{
    AlignmentMode, AlignmentReference, BitDepth, DevicePreference, JobConfig, JobSettings, Method,
};
use pancake::job::{Options, run};

/// Stack a series of photos or fuse an exposure bracket.
#[derive(Debug, Parser)]
#[command(
    version,
    name = "pancake-cli",
    after_help = "To stack two images, run\n  pancake-cli --input near.tif far.tif --output stacked.tif"
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "Each boolean represents an independent command-line switch"
)]
struct Arguments {
    /// Input files or quoted glob patterns. Repeat to add more inputs.
    #[arg(long, required = true, num_args = 1.., value_name = "PATH", help_heading = "Sources")]
    input: Vec<PathBuf>,
    /// Export destination; the extension selects the image format.
    #[arg(long, value_name = "PATH", help_heading = "Output")]
    output: PathBuf,
    /// Combine with laplacian, hybrid, `weighted_average`, `depth_map`, or `exposure_fusion`.
    #[arg(long, value_name = "METHOD", help_heading = "Stack")]
    method: Option<Method>,
    /// Align inputs with a translation model unless a model is supplied.
    #[arg(long, help_heading = "Alignment")]
    align: bool,
    /// Select off, translation, euclidean, similarity, affine, or breathing.
    #[arg(long, value_name = "MODE", help_heading = "Alignment")]
    alignment_mode: Option<AlignmentMode>,
    /// Reference frame: first, middle, or previous.
    #[arg(
        long = "alignment-ref",
        value_name = "FRAME",
        help_heading = "Alignment"
    )]
    alignment_reference: Option<AlignmentReference>,
    /// Focus measurement window size.
    #[arg(long, value_name = "PIXELS", help_heading = "Stack")]
    kernel_size: Option<u32>,
    /// Number of levels in the focus pyramid.
    #[arg(long, value_name = "LEVELS", help_heading = "Stack")]
    pyramid_levels: Option<u32>,
    /// Registration downsampling factor.
    #[arg(long, value_name = "FACTOR", help_heading = "Alignment")]
    scale_factor: Option<u32>,
    /// Maximum positive fractional scale change between frames.
    #[arg(
        long,
        allow_negative_numbers = true,
        value_name = "FRACTION",
        help_heading = "Alignment"
    )]
    max_scale_step: Option<f32>,
    /// Request high-precision registration.
    #[arg(long, help_heading = "Alignment")]
    max_precision: bool,
    /// Process the requested number of inputs per slab; zero uses the whole stack.
    #[arg(
        long,
        allow_negative_numbers = true,
        value_name = "FRAMES",
        help_heading = "Stack"
    )]
    slab_size: Option<u32>,
    /// Neighboring slabs share this many inputs.
    #[arg(
        long,
        allow_negative_numbers = true,
        value_name = "FRAMES",
        help_heading = "Stack"
    )]
    slab_overlap: Option<u32>,
    /// Choose focus and pyramid parameters from the input dimensions.
    #[arg(long, help_heading = "Stack")]
    auto: bool,
    /// Crop to the usable area of the registered stack.
    #[arg(long, help_heading = "Output")]
    auto_crop: bool,
    /// Lossy output quality, from 1 to 100.
    #[arg(long, value_name = "PERCENT", help_heading = "Output")]
    quality: Option<u8>,
    /// Emit a sharpness report for a completed result.
    #[arg(long, help_heading = "Output")]
    quality_report: bool,
    /// Export 8 or 16 bits per integer color channel where supported.
    #[arg(long, value_name = "BITS", help_heading = "Output")]
    bit_depth: Option<BitDepth>,
    /// Run all numerical work on the CPU.
    #[arg(long, conflicts_with_all = ["gpu", "device"], help_heading = "Compute")]
    cpu: bool,
    /// Run numerical work on a GPU.
    #[arg(long, conflicts_with = "device", help_heading = "Compute")]
    gpu: bool,
    /// Device preference: cpu, gpu, or auto.
    #[arg(long, value_name = "DEVICE", help_heading = "Compute")]
    device: Option<DevicePreference>,
    /// Select a specific GPU.
    #[arg(long, value_name = "INDEX", help_heading = "Compute")]
    gpu_id: Option<usize>,
    /// Use the similarity model as the default when aligning.
    #[arg(long, help_heading = "Alignment")]
    rotation_scale: bool,
}

impl Arguments {
    /// Normalize command-line switches to the shared job settings.
    ///
    /// # Errors
    /// Returns invalid settings or inaccessible input paths.
    fn into_job(self) -> Result<JobConfig, String> {
        let defaults = JobSettings::default();
        let alignment = self.alignment_mode.unwrap_or({
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

/// Write a status message without unwinding on a disconnected stream.
fn report(stream: &mut impl Write, args: fmt::Arguments<'_>, status: ExitCode) -> ExitCode {
    if stream.write_fmt(args).is_err() {
        return ExitCode::FAILURE;
    }
    status
}

/// Parse public arguments and execute the shared complete stack.
fn main() -> ExitCode {
    let bad_arguments = ExitCode::from(2);
    let arguments = match Arguments::try_parse() {
        Ok(arguments) => arguments,
        Err(error) if error.use_stderr() => {
            return report(
                &mut io::stderr().lock(),
                format_args!("{error}"),
                bad_arguments,
            );
        }
        Err(help_or_version) => {
            return report(
                &mut io::stdout().lock(),
                format_args!("{help_or_version}"),
                ExitCode::SUCCESS,
            );
        }
    };
    let job = match arguments.into_job() {
        Ok(job) => job,
        Err(error) => {
            return report(
                &mut io::stderr().lock(),
                format_args!("error: {error}\n"),
                bad_arguments,
            );
        }
    };
    let outcome = execute(job, &mut io::stdout().lock(), &mut io::stderr().lock());
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => report(
            &mut io::stderr().lock(),
            format_args!("error: {error}\n"),
            ExitCode::FAILURE,
        ),
    }
}

/// Execute the requested job and report actual completed progress and quality.
///
/// # Errors
///
/// Returns configuration, processing, export or status output errors.
fn execute(
    config: JobConfig,
    output: &mut impl Write,
    diagnostics: &mut impl Write,
) -> Result<(), String> {
    let options = Options::from_env()?;
    let mut progress = Ok(());
    let result = run(config, &options, |snapshot| {
        if progress.is_ok() {
            progress = writeln!(output, "progress {}/{}", snapshot.completed, snapshot.total);
        }
    })
    .map_err(|error| error.to_string())?;
    progress.map_err(|error| error.to_string())?;
    if let Some(sharpness) = result.sharpness() {
        writeln!(output, "sharpness {sharpness:.9}").map_err(|error| error.to_string())?;
    }
    for message in result.warnings() {
        writeln!(diagnostics, "warning: {message}").map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
