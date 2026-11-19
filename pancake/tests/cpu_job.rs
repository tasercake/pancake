//! Real shared numerical stack, filesystem publication, and optional observer.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use pancake::config::{AlignmentMode, BitDepth, DevicePreference, JobConfig, JobSettings, Method};
use pancake::image_io::{RgbImage, decode, export};
use pancake::job::{
    Generation, JobControl, JobError, JobState, Options, StackResult, StackWorker, run,
    run_controlled,
};
use pancake::registration::Affine;

/// A completed weighted stack preserves identity, publishes and then logs.
///
/// # Errors
///
/// Returns invalid input, execution, I/O or invariant errors.
#[test]
fn stack_completion_follows_atomic_export() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("out.png");
    let events = directory.path().join("events.jsonl");
    let config = request(&inputs, &output)?;
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let mut progress = Vec::new();
    let result = run(config, &options, |state| progress.push(state.completed))
        .map_err(|error| error.to_string())?;
    require(
        progress == [1, 2],
        "progress did not count the accumulated sources",
    )?;
    let exported = decode(&output)?;
    let first = decode(inputs.first().ok_or("first input missing")?)?;
    require(
        exported.samples() == first.samples(),
        "sixteen-bit identity samples changed",
    )?;
    require(
        result.image().width() == first.width(),
        "result did not retain canvas dimensions",
    )?;
    let log = std::fs::read_to_string(events).map_err(|error| error.to_string())?;
    let event: serde_json::Value = serde_json::from_str(&log).map_err(|error| error.to_string())?;
    require(
        event.get("event").and_then(serde_json::Value::as_str) == Some("stack_complete")
            && event
                .get("actual_backend")
                .and_then(serde_json::Value::as_str)
                == Some("cpu")
            && event.get("completed").and_then(serde_json::Value::as_bool) == Some(true)
            && event
                .get("physical_gpu")
                .and_then(serde_json::Value::as_bool)
                == Some(false),
        "event does not describe the completed CPU job",
    )
}

/// Crop aligned identity stacks exactly once and preserve unregistered canvases.
///
/// # Errors
///
/// Returns encoding, job or output geometry errors.
#[test]
fn identity_crop_applies_only_to_aligned_jobs() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    for (alignment, width, height) in [(AlignmentMode::Off, 9, 8), (AlignmentMode::Euclidean, 7, 6)]
    {
        let output = directory.path().join(format!("{alignment:?}.png"));
        let config = JobConfig::new(
            &inputs,
            Some(output.clone()),
            JobSettings {
                alignment,
                auto_crop: true,
                device: DevicePreference::Cpu,
                method: Method::WeightedAverage,
                ..JobSettings::default()
            },
        )?;
        let result = run(config, &Options::default(), |_| {}).map_err(|error| error.to_string())?;
        let decoded = decode(&output)?;
        require(
            [result.crop().width(), result.crop().height()] == [width, height]
                && [decoded.width(), decoded.height()] == [width, height],
            "identity crop geometry disagrees with aligned export",
        )?;
    }
    Ok(())
}

/// Repeated structure must leave an ambiguous source unwarped in a completed CPU stack.
///
/// # Errors
///
/// Returns fixture, stack or output errors, or reports an invented displacement.
#[test]
fn repeated_registration_keeps_unwarped_cpu_export() -> Result<(), String> {
    const MOTIF: [u16; 12] = [16, 24, 40, 64, 106, 174, 218, 200, 144, 76, 36, 24];
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = directory.path();
    let inputs = [root.join("repeated.png"), root.join("shifted.png")];
    // A three-pixel phase shift leaves two equally good horizontal matches at
    // +3 and -9. Both are inside the scoring margin at this native proxy size.
    for (path, phase) in inputs.iter().zip([0_usize, 9]) {
        let samples = (0_u16..96)
            .flat_map(|row| {
                let shade = row.min(95_u16.saturating_sub(row)) / 2_u16;
                MOTIF
                    .iter()
                    .cycle()
                    .skip(phase)
                    .take(112)
                    .flat_map(move |tone| {
                        let sample = f32::from(tone.saturating_add(shade)) * (1.0_f32 / 255.0_f32);
                        [sample; 3]
                    })
            })
            .collect();
        let frame = RgbImage::new(112, 96, samples)?;
        export(path, &frame, BitDepth::Sixteen, 95, &[])?;
    }
    let originals = inputs
        .iter()
        .map(std::fs::read)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    let stack = |alignment, output: &Path, options: &Options| {
        let settings = JobSettings {
            alignment,
            auto_crop: false,
            bit_depth: BitDepth::Sixteen,
            device: DevicePreference::Cpu,
            method: Method::WeightedAverage,
            ..JobSettings::default()
        };
        let config = JobConfig::new(&inputs, Some(output.to_owned()), settings)?;
        run(config, options, |_| {}).map_err(|error| error.to_string())
    };
    let control_output = root.join("unaligned.png");
    let control = stack(AlignmentMode::Off, &control_output, &Options::default())?;
    let registered_output = root.join("aligned.png");
    let events = root.join("registration.jsonl");
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let result = stack(AlignmentMode::Translation, &registered_output, &options)?;
    require(
        control.source_transform(1) == Some(Affine::IDENTITY)
            && result.source_transform(1) == Some(Affine::IDENTITY),
        "repeated texture produced an unsupported transform",
    )?;
    let control_image = decode(&control_output)?;
    let registered_image = decode(&registered_output)?;
    require(
        (registered_image.width(), registered_image.height()) == (112, 96)
            && registered_image.samples() == control_image.samples(),
        "ambiguous registration changed the unwarped CPU export",
    )?;
    let event = std::fs::read_to_string(events).map_err(|error| error.to_string())?;
    let event: serde_json::Value =
        serde_json::from_str(&event).map_err(|error| error.to_string())?;
    let identity_transforms = [Affine::IDENTITY.matrix(); 2];
    require(
        event.get("event").and_then(serde_json::Value::as_str) == Some("stack_complete")
            && event.get("completed").and_then(serde_json::Value::as_bool) == Some(true)
            && event
                .get("actual_backend")
                .and_then(serde_json::Value::as_str)
                == Some("cpu")
            && event.get("sequential_transforms") == Some(&serde_json::json!(identity_transforms)),
        "completion did not retain the actual CPU identity transforms",
    )?;
    for (path, before) in inputs.iter().zip(originals) {
        require(
            std::fs::read(path).map_err(|error| error.to_string())? == before,
            "ambiguous registration modified its input",
        )?;
    }
    Ok(())
}

/// Corruption and mid-job cancellation must preserve existing output bytes.
///
/// # Errors
///
/// Returns output, event or lifecycle invariant errors.
#[test]
fn interrupted_execution_retains_an_existing_export() -> Result<(), String> {
    for (method, alignment) in [
        (Method::WeightedAverage, AlignmentMode::Off),
        (Method::WeightedAverage, AlignmentMode::Similarity),
        (Method::Hybrid, AlignmentMode::Similarity),
        (Method::ExposureFusion, AlignmentMode::Euclidean),
    ] {
        verify_interrupted_execution(method, alignment)?;
    }
    Ok(())
}

/// Exercise real publication boundaries for either CPU accumulation route.
/// # Errors
/// Returns output, event or lifecycle invariant errors.
fn verify_interrupted_execution(method: Method, alignment: AlignmentMode) -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("previous.png");
    let events = directory.path().join("events.jsonl");
    std::fs::write(&output, b"previous export").map_err(|error| error.to_string())?;
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let request = || {
        JobConfig::new(
            &inputs,
            Some(output.clone()),
            JobSettings {
                bit_depth: BitDepth::Sixteen,
                device: DevicePreference::Cpu,
                method,
                alignment,
                ..JobSettings::default()
            },
        )
    };
    let control = JobControl::new(Generation::from_counter(1), inputs.len())?;
    let mut accepted = false;
    let outcome = run_controlled(request()?, &options, &control, |progress| {
        if progress.completed == 1 {
            accepted = control.cancel() == Ok(true);
        }
    });
    require(
        accepted,
        "cancellation request was rejected during real accumulation",
    )?;
    require(
        matches!(outcome, Err(JobError::Cancelled)),
        "initial job ignored cancellation",
    )?;
    require(
        control.snapshot()?.state == JobState::Idle,
        "cancelled job did not settle",
    )?;
    let damaged = inputs.last().ok_or("last source missing")?;
    std::fs::write(damaged, b"truncated image").map_err(|error| error.to_string())?;
    let outcome = run(request()?, &options, |_| {});
    let failure = outcome
        .err()
        .ok_or("damaged source unexpectedly succeeded")?;
    require(
        failure.to_string().contains("frame-b.png"),
        "damaged source diagnostic did not identify the source",
    )?;
    let retained = std::fs::read(output).map_err(|error| error.to_string())?;
    require(
        retained == b"previous export" && !events.exists(),
        "interrupted jobs changed a previous export or emitted completion",
    )
}

/// Prepare two tiny genuine sixteen-bit PNG files with a textured identity target.
///
/// # Errors
///
/// Returns invalid raster, allocation or encoding errors.
fn sources(directory: &Path) -> Result<[PathBuf; 2], String> {
    let inputs = [directory.join("frame-a.png"), directory.join("frame-b.png")];
    let samples = (0_u16..216)
        .map(|sample| f32::from(sample) * (1.0 / 216.0))
        .collect();
    let original = RgbImage::new(9, 8, samples)?;
    for path in &inputs {
        export(path, &original, BitDepth::Sixteen, 95, &[])?;
    }
    Ok(inputs)
}

/// Select forced CPU weighted processing with lossless integer export.
///
/// # Errors
///
/// Returns source resolution and configuration errors.
fn request(inputs: &[PathBuf], output: &Path) -> Result<JobConfig, String> {
    JobConfig::new(
        inputs,
        Some(output.to_owned()),
        JobSettings {
            bit_depth: BitDepth::Sixteen,
            device: DevicePreference::Cpu,
            method: Method::WeightedAverage,
            ..JobSettings::default()
        },
    )
}

/// Turn an unmet expectation into a test failure.
///
/// # Errors
///
/// Returns a message when the expectation is false.
fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}

/// An unread result cannot be replaced and stale controls cannot stop its successor.
///
/// # Errors
///
/// Returns a failed stack, worker I/O or lifecycle invariant error.
#[test]
fn worker_reuses_one_thread_without_replacing_retained_results() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("first.png");
    let next_output = directory.path().join("second.png");
    let mut worker = StackWorker::new(Options::default())?;
    let first = worker.start(request(&inputs, &output)?)?;
    let duplicate = worker
        .start(request(&inputs, &next_output)?)
        .err()
        .ok_or("worker admitted a new job while one outcome was outstanding")?;
    require(
        duplicate.contains("active or unread"),
        "outstanding job lacked a useful diagnostic",
    )?;
    let retained = wait_for_stack(&mut worker, first)?;
    let second = worker.start(request(&inputs, &next_output)?)?;
    require(first != second, "worker reused a generation")?;
    require(
        !worker.cancel(first)?,
        "old cancellation reached a new generation",
    )?;
    require(
        worker.take_result(first)?.is_none(),
        "old generation stole the new result",
    )?;
    let result = wait_for_stack(&mut worker, second)?;
    require(
        result.image().samples() == retained.image().samples()
            && worker
                .snapshot()?
                .is_some_and(|snapshot| snapshot.state == JobState::Completed),
        "second stack changed retained pixels or failed to complete",
    )?;
    worker.shutdown()
}

/// Let real work proceed until the current result is available.
///
/// # Errors
///
/// Returns worker failures or reports a lost completion.
fn wait_for_stack(worker: &mut StackWorker, generation: Generation) -> Result<StackResult, String> {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(10) {
        if let Some(value) = worker.take_result(generation)? {
            return value;
        }
        std::thread::yield_now();
    }
    Err("worker did not finish a tiny stack within ten seconds".to_owned())
}

/// Invalid observer paths preserve both the real export and every source.
/// # Errors
/// Returns fixture, execution, I/O or observer-diagnostic errors.
#[test]
fn observer_failures_leave_completed_exports_and_sources_intact() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let first = inputs.first().ok_or("first input missing")?;
    let first_bytes = std::fs::read(first).map_err(|error| error.to_string())?;
    let alias = directory.path().join("source-alias.jsonl");
    std::fs::hard_link(first, &alias).map_err(|error| error.to_string())?;
    let output = directory.path().join("export.png");
    let missing = directory.path().join("missing").join("events.jsonl");
    let expected = decode(first)?;
    let observers: [(&Path, &str); 4] = [
        (&alias, "aliases an input or export"),
        (&output, "aliases an input or export"),
        (directory.path(), "must be a regular file"),
        (&missing, "cannot open completion observer"),
    ];
    for (observer, diagnostic) in observers {
        let options = Options {
            events_path: Some(observer.to_owned()),
            ..Options::default()
        };
        let result =
            run(request(&inputs, &output)?, &options, |_| {}).map_err(|error| error.to_string())?;
        require(
            result.warnings().len() == 1
                && result
                    .warnings()
                    .iter()
                    .all(|warning| warning.contains(diagnostic)),
            &format!(
                "unexpected observer diagnostic for {observer:?}: {:?}",
                result.warnings()
            ),
        )?;
        require(
            decode(&output)?.samples() == expected.samples(),
            "the exported result changed after an observer failure",
        )?;
        require(
            std::fs::read(first).map_err(|error| error.to_string())? == first_bytes,
            "observer failure modified source bytes",
        )?;
    }
    Ok(())
}

/// A memory result can report its real transforms and append to an existing observer.
/// # Errors
/// Returns fixture, compute, JSON or provenance errors.
#[test]
fn memory_result_appends_complete_geometry_to_an_existing_observer() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let observer = directory.path().join("events.jsonl");
    let prior = r#"{"event":"previous"}"#;
    std::fs::write(&observer, format!("{prior}\n")).map_err(|error| error.to_string())?;
    let config = JobConfig::new(
        &inputs,
        None,
        JobSettings {
            device: DevicePreference::Cpu,
            method: Method::WeightedAverage,
            quality_report: true,
            ..JobSettings::default()
        },
    )?;
    let options = Options {
        events_path: Some(observer.clone()),
        ..Options::default()
    };
    let result = run(config, &options, |_| {}).map_err(|error| error.to_string())?;
    require(
        result.warnings().is_empty()
            && result.settings().quality_report
            && result
                .sharpness()
                .is_some_and(|value| value.is_finite() && value >= 0.0),
        "the in-memory result lost its quality observation",
    )?;
    require(
        result.backend() == pancake::runtime::Backend::Cpu
            && result.canvas() == result.crop()
            && result.source_transform(2).is_none(),
        "the in-memory result lost its backend or first-frame canvas",
    )?;
    let log = std::fs::read_to_string(observer).map_err(|error| error.to_string())?;
    let mut lines = log.lines();
    require(
        lines.next() == Some(prior),
        "the previous observer record changed",
    )?;
    let current = lines.next().ok_or("the observer record is missing")?;
    require(
        lines.next().is_none(),
        "the job emitted multiple completion records",
    )?;
    let event: serde_json::Value =
        serde_json::from_str(current).map_err(|error| error.to_string())?;
    let geometry = event
        .get("alignment_transforms")
        .and_then(serde_json::Value::as_object)
        .ok_or("runtime event omitted original source transforms")?;
    let sequential = event
        .get("sequential_transforms")
        .and_then(serde_json::Value::as_array)
        .ok_or("runtime event omitted sequential transforms")?;
    let identity_rows: [[f32; 3]; 2] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let identity = serde_json::json!(identity_rows);
    require(
        geometry.len() == inputs.len()
            && sequential.len() == inputs.len()
            && sequential.iter().all(|transform| transform == &identity),
        "unaligned processing reported an invalid transform layout",
    )?;
    for path in result.sources() {
        let source = path.to_str().ok_or("fixture path must be valid UTF-8")?;
        require(
            geometry.get(source) == Some(&identity),
            "completion geometry does not reference the real source in its original canvas",
        )?;
    }
    Ok(())
}

/// Reject an undersized runtime budget before replacing any previously exported file.
/// # Errors
/// Returns fixture, execution or preservation errors.
#[test]
fn insufficient_runtime_budget_preserves_previous_output() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("previous.png");
    let observer = directory.path().join("events.jsonl");
    let previous = b"previous output sentinel";
    std::fs::write(&output, previous).map_err(|error| error.to_string())?;
    let options = Options {
        working_bytes: 1,
        events_path: Some(observer.clone()),
        ..Options::default()
    };
    let control = JobControl::new(Generation::from_counter(1), inputs.len())?;
    let failure = run_controlled(request(&inputs, &output)?, &options, &control, |_| {})
        .err()
        .ok_or("undersized runtime budget unexpectedly succeeded")?;
    require(
        matches!(failure, JobError::Failed(message) if message.contains("job budget")),
        "runtime budget failure lacked its real reason",
    )?;
    require(
        control.snapshot()?.state == JobState::Failed
            && !observer.exists()
            && std::fs::read(output).map_err(|error| error.to_string())? == previous,
        "failure changed the output or reported completion",
    )
}

/// A failed generation can be consumed once, followed by normal work on the same thread.
/// # Errors
/// Returns invalid fixtures, broken worker reuse or output preservation errors.
#[test]
fn worker_reports_a_failed_generation_and_accepts_recovery() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let repaired = inputs.last().ok_or("last source missing")?;
    let original = std::fs::read(repaired).map_err(|error| error.to_string())?;
    let output = directory.path().join("previous.png");
    let previous = b"previous output sentinel";
    std::fs::write(&output, previous).map_err(|error| error.to_string())?;
    std::fs::write(repaired, b"corrupt raster").map_err(|error| error.to_string())?;
    let mut worker = StackWorker::new(Options::default())?;
    require(
        worker.snapshot()?.is_none() && !worker.cancel(Generation::from_counter(0))?,
        "an empty worker accepted a stale cancellation",
    )?;
    let failed = worker.start(request(&inputs, &output)?)?;
    let diagnostic = wait_for_stack(&mut worker, failed)
        .err()
        .ok_or("worker accepted a corrupt source")?;
    require(
        diagnostic.contains("frame-b.png")
            && worker
                .snapshot()?
                .is_some_and(|snapshot| snapshot.state == JobState::Failed)
            && worker.take_result(failed)?.is_none()
            && std::fs::read(&output).map_err(|error| error.to_string())? == previous,
        "failed generation lost diagnostics or changed the previous export",
    )?;

    std::fs::write(repaired, original).map_err(|error| error.to_string())?;
    let recovered = worker.start(request(&inputs, &output)?)?;
    require(
        failed != recovered && !worker.resume(failed)?,
        "stale control reached recovery",
    )?;
    let complete = wait_for_stack(&mut worker, recovered)?;
    let expected = decode(inputs.first().ok_or("first input missing")?)?;
    require(
        decode(&output)?.samples() == expected.samples()
            && complete.backend() == pancake::runtime::Backend::Cpu,
        "recovery did not publish the actual stack",
    )?;
    worker.shutdown()?;
    let closed = worker
        .start(request(&inputs, &output)?)
        .err()
        .ok_or("shut-down worker accepted another request")?;
    require(
        closed.contains("shut down"),
        "closed worker reported the wrong error",
    )
}

/// A real stack waits at a drained boundary and publishes only after resume.
///
/// # Errors
/// Returns fixture, lifecycle, numerical or atomic publication errors.
#[test]
fn paused_stack_resumes_and_publishes_only_its_completed_generation() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let inputs = sources(directory.path())?;
    let output = directory.path().join("retained.png");
    let previous = b"prior result";
    std::fs::write(&output, previous).map_err(|error| error.to_string())?;
    let config = request(&inputs, &output)?;
    let control = JobControl::new(Generation::from_counter(8), inputs.len())?;
    let controller = control.clone();
    let (requested, seen) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        run_controlled(config, &Options::default(), &controller, |progress| {
            if progress.completed == 1 {
                drop(requested.send(controller.pause()));
            }
        })
    });
    let verification = (|| {
        require(
            seen.recv_timeout(Duration::from_secs(5))
                .map_err(|error| error.to_string())??,
            "running job refused an actual pause",
        )?;
        let started = Instant::now();
        loop {
            let snapshot = control.snapshot()?;
            if snapshot.state == JobState::Paused {
                require(
                    snapshot.completed == 1
                        && snapshot.pause_acknowledged.is_some()
                        && std::fs::read(&output).map_err(|error| error.to_string())? == previous,
                    "paused job submitted another frame or replaced the export",
                )?;
                break;
            }
            if started.elapsed() >= Duration::from_secs(5) {
                return Err("worker never acknowledged the pause".to_owned());
            }
            std::thread::yield_now();
        }
        require(
            control.resume()?
                && !matches!(
                    control.snapshot()?.state,
                    JobState::Pausing | JobState::Paused
                ),
            "resume returned without releasing the real stack",
        )
    })();
    if verification.is_err() {
        drop(control.cancel());
    }
    let outcome = worker.join().map_err(|_| "stack executor did not return")?;
    verification?;
    let result = outcome.map_err(|error| error.to_string())?;
    let finished = control.snapshot()?;
    require(
        finished.state == JobState::Completed
            && finished.completed == inputs.len()
            && decode(&output)?.samples() == result.image().samples(),
        "resumed job did not complete and preserve the actual samples",
    )
}

/// Report exact Unicode source geometry and preserve an export when its
/// observer cannot encode a native byte path.
///
/// # Errors
/// Returns fixture, execution, geometry, I/O or observer-diagnostic errors.
#[cfg(unix)]
#[test]
fn observer_handles_non_unicode_paths_without_fabricating_geometry() -> Result<(), String> {
    use std::os::unix::ffi::OsStrExt;

    // Darwin's EILSEQ is 92. APFS rejected the invalid UTF-8 rename in CI.
    const DARWIN_EILSEQ: i32 = 92;

    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = directory.path();
    let [first, second] = sources(root)?;
    let original = std::fs::read(&second).map_err(|error| error.to_string())?;
    let expected = decode(&first)?;
    let unicode = root.join("frame-雪-🥞.png");
    std::fs::rename(&second, &unicode).map_err(|error| error.to_string())?;
    let output = root.join("unicode.png");
    let events = root.join("unicode-events.jsonl");
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let result = run(
        request(&[first.clone(), unicode.clone()], &output)?,
        &options,
        |_| {},
    )
    .map_err(|error| error.to_string())?;
    let observed = std::fs::read(events).map_err(|error| error.to_string())?;
    let event: serde_json::Value =
        serde_json::from_slice(&observed).map_err(|error| error.to_string())?;
    let geometry = event
        .get("alignment_transforms")
        .and_then(serde_json::Value::as_object)
        .ok_or("Unicode stack did not report source geometry")?;
    require(
        event.get("event").and_then(serde_json::Value::as_str) == Some("stack_complete")
            && event
                .get("actual_backend")
                .and_then(serde_json::Value::as_str)
                == Some("cpu")
            && event.get("completed").and_then(serde_json::Value::as_bool) == Some(true)
            && geometry.len() == 2
            && result.warnings().is_empty()
            && decode(&output)?.samples() == expected.samples(),
        "Unicode paths changed the real CPU stack or its completion event",
    )?;
    let identity = serde_json::json!([[1.0_f32, 0.0_f32, 0.0_f32], [0.0_f32, 1.0_f32, 0.0_f32]]);
    for source in [&first, &unicode] {
        let canonical = source.canonicalize().map_err(|error| error.to_string())?;
        let key = canonical
            .to_str()
            .ok_or("Unicode fixture path was not UTF-8")?;
        require(
            geometry.get(key) == Some(&identity),
            "the observer did not report the exact Unicode source key and transform",
        )?;
    }

    let opaque = root.join(std::ffi::OsStr::from_bytes(b"frame-\xff.png"));
    match std::fs::rename(&unicode, &opaque) {
        Ok(()) => {}
        Err(error) if cfg!(target_os = "macos") && error.raw_os_error() == Some(DARWIN_EILSEQ) => {
            return require(
                std::fs::read(&unicode).map_err(|error| error.to_string())? == original,
                "Darwin's rejected rename changed the original source",
            );
        }
        Err(error) => return Err(format!("could not rename source to a byte path: {error}")),
    }
    let output = root.join("retained.png");
    let events = root.join("events.jsonl");
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let result = run(
        request(&[first, opaque.clone()], &output)?,
        &options,
        |_| {},
    )
    .map_err(|error| error.to_string())?;
    require(
        decode(&output)?.samples() == expected.samples()
            && !events.exists()
            && result
                .warnings()
                .iter()
                .any(|warning| warning.contains("UTF-8 source paths"))
            && std::fs::read(opaque).map_err(|error| error.to_string())? == original,
        "observer replaced undisplayable names or affected the real output or source",
    )
}

/// A controller rejects the wrong input count without consuming its valid
/// request. A completed controller cannot publish a second stack.
///
/// # Errors
/// Returns invalid fixture, admission, lifecycle or publication errors.
#[test]
fn controller_rejects_mismatched_and_repeated_admission() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = directory.path();
    let [first, second] = sources(root)?;
    let third = root.join("third.png");
    std::fs::hard_link(&first, &third).map_err(|error| error.to_string())?;
    let output = root.join("prior.png");
    let previous = b"prior export";
    std::fs::write(&output, previous).map_err(|error| error.to_string())?;
    let control = JobControl::new(Generation::from_counter(1), 2)?;
    let options = Options::default();
    let mismatched = run_controlled(
        request(&[first.clone(), second.clone(), third], &output)?,
        &options,
        &control,
        |_| {},
    )
    .err()
    .ok_or("controller accepted a mismatched request")?;
    require(
        mismatched.to_string().contains("configured input count")
            && control.snapshot()?.completed == 0
            && std::fs::read(&output).map_err(|error| error.to_string())? == previous,
        "mismatched admission consumed or published output",
    )?;
    let inputs = [first, second];
    let complete = run_controlled(request(&inputs, &output)?, &options, &control, |_| {})
        .map_err(|error| error.to_string())?;
    let published = std::fs::read(&output).map_err(|error| error.to_string())?;
    let duplicate = run_controlled(request(&inputs, &output)?, &options, &control, |_| {})
        .err()
        .ok_or("completed controller accepted a duplicate request")?;
    require(
        duplicate.to_string().contains("already owns an execution")
            && control.snapshot()?.state == JobState::Completed
            && decode(&output)?.samples() == complete.image().samples()
            && std::fs::read(output).map_err(|error| error.to_string())? == published,
        "repeated admission affected the completed CPU stack",
    )
}

/// Moving an already consumed source leaves the actual stack usable. An
/// existing observer cannot append a completion when identity inspection fails.
///
/// # Errors
/// Returns fixture, execution, observation or publication errors.
#[test]
fn moved_source_blocks_observer_append_without_undoing_export() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = directory.path();
    let inputs = sources(root)?;
    let source = inputs.first().ok_or("fixture source missing")?;
    let moved = root.join("moved.png");
    let expected = decode(source)?;
    let output = root.join("published.png");
    let marker = b"{\"event\":\"prior\"}\n";
    let observer = root.join("events.jsonl");
    std::fs::write(&observer, marker).map_err(|error| error.to_string())?;
    let options = Options {
        events_path: Some(observer.clone()),
        ..Options::default()
    };
    let mut movement = None;
    let completed = run(request(&inputs, &output)?, &options, |progress| {
        if progress.completed == inputs.len() {
            movement = Some(std::fs::rename(source, &moved));
        }
    })
    .map_err(|error| error.to_string())?;
    movement
        .ok_or("stack did not observe final accumulation")?
        .map_err(|error| error.to_string())?;
    let missing = std::fs::metadata(source)
        .err()
        .ok_or("source was not moved")?;
    require(
        completed.warnings() == [missing.to_string()]
            && std::fs::read(observer).map_err(|error| error.to_string())? == marker
            && decode(&moved)?.samples() == expected.samples()
            && decode(&output)?.samples() == expected.samples(),
        "observer identity failure affected the prior record or the published CPU stack",
    )
}

/// A directory at the export destination is rejected without replacing its
/// contents or claiming a completed stack.
///
/// # Errors
/// Returns fixture, execution, lifecycle or publication errors.
#[test]
fn directory_export_destination_preserves_existing_data() -> Result<(), String> {
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let root = directory.path();
    let inputs = sources(root)?;
    let destination = root.join("occupied.png");
    std::fs::create_dir(&destination).map_err(|error| error.to_string())?;
    let sentinel = destination.join("preserve");
    let previous = b"directory contents";
    std::fs::write(&sentinel, previous).map_err(|error| error.to_string())?;
    let events = root.join("events.jsonl");
    let options = Options {
        events_path: Some(events.clone()),
        ..Options::default()
    };
    let control = JobControl::new(Generation::from_counter(1), inputs.len())?;
    let failure = run_controlled(request(&inputs, &destination)?, &options, &control, |_| {})
        .err()
        .ok_or("directory was accepted as an export file")?;
    require(
        failure.to_string().contains("occupied.png")
            && failure.to_string().contains("not a regular file")
            && control.snapshot()?.state == JobState::Failed
            && !events.exists()
            && std::fs::read(sentinel).map_err(|error| error.to_string())? == previous,
        "failed export changed prior data or appeared completed",
    )
}
