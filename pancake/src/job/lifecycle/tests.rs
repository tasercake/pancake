//! Check control requests at the same boundary observed by a real worker.

use super::{Generation, JobControl, JobState, Stage, Stop};
use std::thread;
use std::time::Duration;

/// A request is acknowledged only at a drained boundary.
///
/// # Errors
///
/// Returns missing acknowledgement, synchronization or transition errors.
#[test]
fn pause_acknowledgement_and_resume() -> Result<(), String> {
    let control = JobControl::new(Generation(3), 2)?;
    control
        .checkpoint(Stage::Decode)
        .map_err(|error| error.to_string())?;
    require(control.pause()?, "running work must accept a pause request")?;
    require(
        control.snapshot()?.state == JobState::Pausing,
        "request receipt does not prove work has reached a safe boundary",
    )?;
    let worker_control = control.clone();
    let worker = thread::spawn(move || worker_control.checkpoint(Stage::Fuse));
    let paused = wait_for_pause(&control)?;
    require(
        paused.state == JobState::Paused && paused.stage == Stage::Decode,
        "worker must pause before starting the next stage",
    )?;
    let acknowledged = control.resume()? && control.snapshot()?.state == JobState::Running;
    if !acknowledged {
        drop(control.cancel());
    }
    let resumed = worker.join().map_err(|_| "boundary worker panicked")?;
    require(acknowledged, "resume returned without releasing the pause")?;
    resumed.map_err(|error| error.to_string())?;
    require(
        control.snapshot()?.stage == Stage::Fuse,
        "resumed stage is invisible",
    )
}

/// A cancelled job keeps acknowledgements through full cleanup and refuses new work.
///
/// # Errors
///
/// Returns missing acknowledgement or inconsistent terminal state.
#[test]
fn cancellation_followed_by_cleanup() -> Result<(), String> {
    let control = JobControl::new(Generation(2), 2)?;
    require(control.cancel()?, "first cancellation was refused")?;
    require(!control.cancel()?, "same cancellation was accepted twice")?;
    let pending = control.snapshot()?;
    require(
        pending.state == JobState::Running && pending.cancel_acknowledged.is_none(),
        "request receipt occurred before the worker drained the active batch",
    )?;
    require(
        control.checkpoint(Stage::Decode) == Err(Stop::Cancelled),
        "new batch started despite cancellation",
    )?;
    let cancelling = control.snapshot()?;
    require(
        cancelling.state == JobState::Cancelling && cancelling.cancel_acknowledged.is_some(),
        "worker did not acknowledge cancellation before cleanup",
    )?;
    control.settle_cancelled()?;
    let idle = control.snapshot()?;
    require(
        idle.state == JobState::Idle && idle.cancel_acknowledged == cancelling.cancel_acknowledged,
        "settled state lost cancellation acknowledgement",
    )?;
    require(
        control.checkpoint(Stage::Fuse) == Err(Stop::Closed),
        "settled work submitted again",
    )
}

/// Cancelling a paused controller must wake the worker without resuming submissions.
///
/// # Errors
///
/// Returns missing acknowledgement, synchronization or incorrect resume status.
#[test]
fn cancellation_wakes_paused_worker() -> Result<(), String> {
    let control = JobControl::new(Generation(7), 2)?;
    require(control.pause()?, "initial work refused a pause")?;
    require(
        !control.pause()? && !control.resume()?,
        "pause was acknowledged before the worker boundary",
    )?;
    control.complete_frame()?;
    require(
        control.snapshot()?.completed == 1,
        "an already submitted frame was discarded during its drain",
    )?;
    let worker_control = control.clone();
    let worker = thread::spawn(move || worker_control.checkpoint(Stage::Fuse));
    let paused = wait_for_pause(&control)?;
    require(
        paused.state == JobState::Paused,
        "worker missed the pause boundary",
    )?;
    require(control.cancel()?, "paused job refused cancellation")?;
    require(!control.resume()?, "resume overtook cancellation")?;
    let result = worker.join().map_err(|_| "paused worker panicked")?;
    require(
        result == Err(Stop::Cancelled) && control.snapshot()?.stage == Stage::Prepare,
        "paused worker woke and entered a stage after cancellation",
    )
}

/// Progress comes from completed frames; terminal state freezes the generation.
///
/// # Errors
///
/// Returns incorrect progress, terminal status or lifecycle transition errors.
#[test]
fn completed_stage_freezes_progress() -> Result<(), String> {
    let control = JobControl::new(Generation(4), 2)?;
    control.complete_frame()?;
    control.complete_frame()?;
    let overflow = control
        .complete_frame()
        .err()
        .ok_or("excess frame accepted")?;
    require(
        overflow.contains("total"),
        "frame overflow failed without a useful diagnostic",
    )?;
    control.commit().map_err(|error| error.to_string())?;
    require(
        !control.cancel()? && !control.pause()? && !control.resume()?,
        "publication accepted a competing control",
    )?;
    let closed = control
        .checkpoint(Stage::Fuse)
        .err()
        .ok_or("publication admitted another calculation")?;
    let progress_error = control
        .complete_frame()
        .err()
        .ok_or("publication admitted more progress")?;
    require(
        closed == Stop::Closed
            && closed.to_string().contains("cannot admit")
            && progress_error.contains("current state"),
        "closed checkpoint lost its public diagnostic or changed progress",
    )?;
    control.finish(true)?;
    let completed = control.snapshot()?;
    require(
        completed.generation == Generation(4)
            && completed.completed == 2
            && completed.total == 2
            && completed.state == JobState::Completed,
        "completed job lost progress or generation",
    )?;
    require(
        !control.pause()? && !control.cancel()?,
        "completed work accepted requests",
    )
}

/// Wait on the real status condition; a deadline reports a lost acknowledgement.
///
/// # Errors
///
/// Returns a lock error if the state cannot be read.
fn wait_for_pause(control: &JobControl) -> Result<super::JobSnapshot, String> {
    let (locked, _) = control
        .shared
        .changed
        .wait_timeout_while(control.lock()?, Duration::from_secs(5), |state| {
            state.snapshot.state != JobState::Paused
        })
        .map_err(|_| "job state lock poisoned during acknowledgement")?;
    let snapshot = locked.snapshot;
    drop(locked);
    Ok(snapshot)
}

/// Convert an unmet test condition into a descriptive failure.
///
/// # Errors
///
/// Returns the supplied message if the condition is false.
fn require(condition: bool, message: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| message.to_owned())
}

/// Publication rejects incomplete processing and failed jobs do not report success.
///
/// # Errors
///
/// Returns absent diagnostics or inconsistent failure state.
#[test]
fn incomplete_processing_cannot_publish() -> Result<(), String> {
    let zero = JobControl::new(Generation(0), 0)
        .err()
        .ok_or("zero-input job started")?;
    require(
        zero.contains("input count"),
        "empty job failed without an input count diagnostic",
    )?;
    let control = JobControl::new(Generation(1), 2)?;
    require(
        control.commit() == Err(Stop::InvalidState),
        "an incomplete job crossed the publication boundary",
    )?;
    let early_completion = control
        .finish(true)
        .err()
        .ok_or("uncommitted job completed")?;
    require(
        early_completion.contains("completion"),
        "uncommitted job had no completion diagnostic",
    )?;
    let early_settlement = control
        .settle_cancelled()
        .err()
        .ok_or("running job settled")?;
    require(
        early_settlement.contains("cancellation"),
        "running job had no cancellation diagnostic",
    )?;
    control.finish(false)?;
    require(
        control.snapshot()?.state == JobState::Failed,
        "worker failure was not published",
    )?;
    require(
        control.checkpoint(Stage::Decode) == Err(Stop::Closed),
        "failed job admitted new work",
    )?;
    let late_progress = control
        .complete_frame()
        .err()
        .ok_or("failed job advanced")?;
    require(
        late_progress.contains("current state"),
        "late progress had no lifecycle diagnostic",
    )
}

/// A late cancellation takes precedence over atomically publishing a staged export.
///
/// # Errors
///
/// Returns erroneous final-stage lifecycle transitions.
#[test]
fn last_boundary_observes_cancellation() -> Result<(), String> {
    let control = JobControl::new(Generation(6), 1)?;
    control.complete_frame()?;
    control
        .checkpoint(Stage::Encode)
        .map_err(|error| error.to_string())?;
    require(
        control.cancel()?,
        "job preparing an export refused cancellation",
    )?;
    require(
        control.commit() == Err(Stop::Cancelled),
        "a cancelled export could publish",
    )?;
    let snapshot = control.snapshot()?;
    require(
        snapshot.stage == Stage::Encode
            && snapshot.state == JobState::Cancelling
            && snapshot.cancel_acknowledged.is_some(),
        "final boundary did not observe cancellation before publication",
    )
}
