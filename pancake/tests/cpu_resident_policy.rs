//! Native allocation contract for the private normalized RGB handoff.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use cubecl::MemoryScope;
use cubecl::logging::{LaunchObservation, LaunchObserver};
use pancake::config::{
    AlignmentMode, AlignmentReference, BitDepth, DevicePreference, JobConfig, JobSettings, Method,
};
use pancake::image_io::{RgbImage, export};
use pancake::job::{Generation, JobControl, JobState, Options, Stage, run_controlled};

/// One source's width. The packed allocation has no native pool padding.
const WIDTH: usize = 32;
/// One source's height.
const HEIGHT: usize = 24;
/// Bytes in one full packed f32 RGB source.
const RGB_BYTES: u64 = 9_216;

/// Request pause at the anchor's actual luma launch, before it has read back.
struct PauseAtLuma {
    /// State of the real single use job.
    control: JobControl,
    /// Whether the first registration launch already submitted a pause request.
    requested: AtomicBool,
}

impl LaunchObserver for PauseAtLuma {
    fn launched(&self, kernel: &'static str) {
        if kernel.contains("smooth_luma") && !self.requested.swap(true, Ordering::Relaxed) {
            let _request = self.control.pause();
        }
    }
}

/// Cancel on an early assertion so the scoped worker can always join.
struct ReleasePause<'a>(&'a JobControl);

impl Drop for ReleasePause<'_> {
    fn drop(&mut self) {
        let _request = self.0.cancel();
    }
}

/// Exact exported output and provenance from a completed job.
#[derive(PartialEq, Eq)]
struct Completed {
    /// Full-precision retained result sample bits.
    samples: Vec<u32>,
    /// Atomic RGB16 TIFF contents.
    file: Vec<u8>,
    /// Actual runtime and registration maps from the production event.
    event: serde_json::Value,
}

/// Active pool usage and the completed job after a real pause and resume.
struct Witness {
    /// Bytes in active allocations at the first fusion admission boundary.
    bytes: u64,
    /// Number of active allocations at the same boundary.
    allocations: u64,
    /// Published result and completion.
    completed: Completed,
}

/// Run one exact CPU stack using the requested slab option.
/// # Errors
/// Returns job, lifecycle, allocation observation or output errors.
fn observe(root: &Path, inputs: &[PathBuf], slab_size: u32) -> Result<Witness, String> {
    let output = root.join(format!("stack-{slab_size}.tif"));
    let event_path = root.join(format!("stack-{slab_size}.jsonl"));
    let config = JobConfig::new(
        inputs,
        Some(output.clone()),
        JobSettings {
            alignment: AlignmentMode::Translation,
            alignment_reference: AlignmentReference::First,
            method: Method::ExposureFusion,
            slab_size,
            bit_depth: BitDepth::Sixteen,
            device: DevicePreference::Cpu,
            ..JobSettings::default()
        },
    )?;
    let control = JobControl::new(Generation::from_counter(1), inputs.len())?;
    let options = Options {
        events_path: Some(event_path.clone()),
        ..Options::default()
    };
    let observer = Arc::new(PauseAtLuma {
        control: control.clone(),
        requested: AtomicBool::new(false),
    });
    let _observation = LaunchObservation::new(observer.clone());
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    let (usage, result) = thread::scope(|scope| {
        let worker = scope.spawn(|| run_controlled(config, &options, &control, |_| {}));
        let _release = ReleasePause(&control);
        let snapshot = loop {
            let snapshot = control.snapshot()?;
            if snapshot.state == JobState::Paused {
                break snapshot;
            }
            if worker.is_finished() {
                return Err("CPU job finished before the first luma pause".to_owned());
            }
            thread::yield_now();
        };
        if !observer.requested.load(Ordering::Relaxed)
            || snapshot.stage != Stage::Register
            || snapshot.completed != 0
            || snapshot.pause_acknowledged.is_none()
        {
            return Err("CPU job did not pause between first registration and fusion".to_owned());
        }
        let usage = device.client().memory_report(MemoryScope::Device).usage();
        if !control.resume()? {
            return Err("acknowledged CPU job could not resume".to_owned());
        }
        let result = worker
            .join()
            .map_err(|_| "CPU job worker unwound".to_owned())?
            .map_err(|error| error.to_string())?;
        Ok((usage, result))
    })?;
    let event = std::fs::read_to_string(event_path).map_err(|error| error.to_string())?;
    let lines = event.lines().collect::<Vec<_>>();
    let [line] = lines.as_slice() else {
        return Err("expected one complete CPU event".into());
    };
    let event: serde_json::Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
    if event.get("runtime").and_then(serde_json::Value::as_str) != Some("cpu")
        || event
            .get("actual_backend")
            .and_then(serde_json::Value::as_str)
            != Some("cpu")
        || event.get("completed").and_then(serde_json::Value::as_bool) != Some(true)
        || event
            .get("physical_gpu")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        || event.get("fallback_reason") != Some(&serde_json::Value::Null)
    {
        return Err("CPU residency observation did not complete on the selected runtime".into());
    }
    Ok(Witness {
        bytes: usage.bytes_in_use,
        allocations: usage.number_allocs,
        completed: Completed {
            samples: result
                .image()
                .samples()
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            file: std::fs::read(output).map_err(|error| error.to_string())?,
            event,
        },
    })
}

/// Only an unslabbed aligned original keeps one full RGB input across the pause.
/// # Errors
/// Returns malformed fixtures, failed jobs or a changed live allocation contract.
#[test]
fn configured_slabs_do_not_hold_the_registration_rgb() -> Result<(), String> {
    let root = tempfile::tempdir().map_err(|error| error.to_string())?;
    let image = RgbImage::new(
        WIDTH,
        HEIGHT,
        (0..253_u8)
            .cycle()
            .take(WIDTH * HEIGHT * 3)
            .map(|value| f32::from(value) / 256.0)
            .collect(),
    )?;
    let inputs = [
        root.path().join("first.tif"),
        root.path().join("second.tif"),
    ];
    for path in &inputs {
        export(path, &image, BitDepth::Sixteen, 95, &[])?;
    }
    let whole = observe(root.path(), &inputs, 0)?;
    let configured = observe(root.path(), &inputs, 2)?;
    if whole.completed != configured.completed {
        return Err("resident handoff changed the output, registration or completion event".into());
    }
    if configured.allocations.checked_add(1) != Some(whole.allocations)
        || configured.bytes.checked_add(RGB_BYTES) != Some(whole.bytes)
    {
        return Err(format!(
            "expected the unslabbed job to hold one additional {RGB_BYTES}-byte RGB tensor; unslabbed {}/{}, configured slab {}/{} active allocations/bytes",
            whole.allocations, whole.bytes, configured.allocations, configured.bytes
        ));
    }
    Ok(())
}
