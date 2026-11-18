//! Cooperative lifecycle for one stack job.

use std::fmt::{self, Display};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Instant;

#[cfg(test)]
#[path = "lifecycle/tests.rs"]
mod tests;

/// Identity assigned when a worker accepts a stack request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Generation(u64);

impl Generation {
    /// Construct an identity from the owning worker's monotonic counter.
    #[must_use]
    pub const fn from_counter(counter: u64) -> Self {
        Self(counter)
    }
}

/// Last confirmed condition of the stack worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobState {
    /// The worker has released the previous job's resources.
    Idle,
    /// The worker can submit stack work.
    Running,
    /// Pause was requested and the active batch is still draining.
    Pausing,
    /// The worker reached a safe pause boundary.
    Paused,
    /// The worker stopped submission and is relinquishing job resources.
    Cancelling,
    /// The job finished with an error.
    Failed,
    /// The job produced a complete result.
    Completed,
}

/// Last stage admitted by the worker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stage {
    /// Preparing the job and selecting an execution runtime.
    Prepare,
    /// Decoding an original input.
    Decode,
    /// Estimating alignment or warping one frame.
    Register,
    /// Accumulating or finishing focus or exposure fusion.
    Fuse,
    /// Preparing the encoded result in a temporary file.
    Encode,
    /// Publishing a result that has passed the final cancellation boundary.
    Publish,
}

/// Observable progress and acknowledged lifecycle timestamps for one job.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobSnapshot {
    /// Identity of the stack that produced this snapshot.
    pub generation: Generation,
    /// Last confirmed worker state.
    pub state: JobState,
    /// Stage last admitted for submission.
    pub stage: Stage,
    /// Number of original frames successfully accumulated.
    pub completed: usize,
    /// Total original inputs requested.
    pub total: usize,
    /// Most recent instant at which a pause reached a safe boundary.
    pub pause_acknowledged: Option<Instant>,
    /// Instant at which cancellation stopped further submissions.
    pub cancel_acknowledged: Option<Instant>,
}

/// Reason a worker cannot submit through a lifecycle boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stop {
    /// User cancellation was acknowledged at a safe boundary.
    Cancelled,
    /// The job reached an irreversible or terminal state.
    Closed,
    /// A progress or synchronization invariant failed.
    InvalidState,
}

impl Display for Stop {
    /// Format a human-readable worker diagnostic.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Cancelled => "stack cancelled",
            Self::Closed => "stack cannot admit further work",
            Self::InvalidState => "invalid or poisoned stack lifecycle state",
        })
    }
}

impl std::error::Error for Stop {}

/// Each synchronized transition updates the same observable job state.
#[derive(Debug)]
struct State {
    /// Externally observable state protected by the lifecycle mutex.
    snapshot: JobSnapshot,
    /// Whether one executor has acquired this one-use job controller.
    claimed: bool,
    /// Cancellation is requested separately from worker acknowledgement.
    cancel_requested: bool,
    /// The result crossed the final safe boundary and may now be published.
    committed: bool,
}

impl State {
    /// Whether there can be new user requests while this state is active.
    const fn controllable(&self) -> bool {
        !self.cancel_requested
            && !self.committed
            && matches!(
                self.snapshot.state,
                JobState::Running | JobState::Pausing | JobState::Paused
            )
    }
}

/// State and notification for the UI controller and the job worker.
#[derive(Debug)]
struct Shared {
    /// Protects both requests and worker acknowledgements.
    state: Mutex<State>,
    /// Wakes a paused worker after resume or cancellation.
    changed: Condvar,
}

/// A single-use, shareable controller for a stack job.
///
/// The worker calls a checkpoint only with its previous bounded batch drained.
/// The requester sends control updates without waiting for work to finish.
#[derive(Clone, Debug)]
pub struct JobControl {
    /// One shared state owned by this generation only.
    shared: Arc<Shared>,
}

impl JobControl {
    /// Start a controller for the accepted job's original input count.
    ///
    /// # Errors
    ///
    /// Rejects requests without inputs.
    pub fn new(generation: Generation, total: usize) -> Result<Self, String> {
        if total == 0 {
            return Err("stack job requires an input count".to_owned());
        }
        Ok(Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    snapshot: JobSnapshot {
                        generation,
                        state: JobState::Running,
                        stage: Stage::Prepare,
                        completed: 0,
                        total,
                        pause_acknowledged: None,
                        cancel_acknowledged: None,
                    },
                    claimed: false,
                    cancel_requested: false,
                    committed: false,
                }),
                changed: Condvar::new(),
            }),
        })
    }

    /// Admit exactly one executor for the original request count.
    ///
    /// # Errors
    ///
    /// Reports mismatched input counts, duplicate admission or lock failure.
    pub(super) fn claim(&self, total: usize) -> Result<(), String> {
        let mut state = self.lock()?;
        if state.claimed {
            return Err("stack controller already owns an execution".to_owned());
        }
        if state.snapshot.total != total {
            return Err("stack controller does not match the configured input count".to_owned());
        }
        state.claimed = true;
        drop(state);
        Ok(())
    }

    /// Read a consistent worker snapshot.
    ///
    /// # Errors
    ///
    /// Returns an error if the lifecycle lock is poisoned.
    pub fn snapshot(&self) -> Result<JobSnapshot, String> {
        Ok(self.lock()?.snapshot)
    }

    /// Request a pause. Returns true only when a new request was accepted.
    ///
    /// # Errors
    ///
    /// Returns an error if the lifecycle lock is poisoned.
    pub fn pause(&self) -> Result<bool, String> {
        let mut state = self.lock()?;
        if !state.controllable() || state.snapshot.state != JobState::Running {
            return Ok(false);
        }
        state.snapshot.state = JobState::Pausing;
        state.snapshot.pause_acknowledged = None;
        drop(state);
        Ok(true)
    }

    /// Resume a worker that has acknowledged a pause.
    ///
    /// # Errors
    ///
    /// Returns an error if the lifecycle lock is poisoned.
    pub fn resume(&self) -> Result<bool, String> {
        let mut state = self.lock()?;
        if !state.controllable() || state.snapshot.state != JobState::Paused {
            return Ok(false);
        }
        state.snapshot.state = JobState::Running;
        drop(state);
        self.shared.changed.notify_all();
        Ok(true)
    }

    /// Request cancellation and release a paused worker to acknowledge it.
    ///
    /// # Errors
    ///
    /// Returns an error if the lifecycle lock is poisoned.
    pub fn cancel(&self) -> Result<bool, String> {
        let mut state = self.lock()?;
        if !state.controllable() {
            return Ok(false);
        }
        state.cancel_requested = true;
        drop(state);
        self.shared.changed.notify_all();
        Ok(true)
    }

    /// Observe controls at a safe boundary before entering the requested stage.
    ///
    /// Acknowledges a pause only here and waits for a resume or cancel. Once
    /// cancellation is accepted, future checkpoints refuse further work.
    ///
    /// # Errors
    ///
    /// Reports cancellation, a closed job or a synchronization failure.
    pub fn checkpoint(&self, stage: Stage) -> Result<(), Stop> {
        let mut locked = self.admit()?;
        locked.snapshot.stage = stage;
        drop(locked);
        Ok(())
    }

    /// Count one original frame that finished accumulation.
    ///
    /// # Errors
    ///
    /// Returns a progress error if an inactive or fully counted job advances.
    pub fn complete_frame(&self) -> Result<(), String> {
        let mut state = self.lock()?;
        if !matches!(state.snapshot.state, JobState::Running | JobState::Pausing) || state.committed
        {
            return Err("stack cannot report frames in the current state".to_owned());
        }
        state.snapshot.completed = state
            .snapshot
            .completed
            .checked_add(1)
            .filter(|&completed| completed <= state.snapshot.total)
            .ok_or("completed frames exceed the stack total")?;
        drop(state);
        Ok(())
    }

    /// Reserve final publication after all active work and pause/cancel requests.
    ///
    /// Encode any file into temporary storage before this call. Only publication
    /// of the staged export and completed result may follow a successful claim.
    ///
    /// # Errors
    ///
    /// Reports cancellation, a closed job, an incomplete frame count or a
    /// synchronization failure.
    pub fn commit(&self) -> Result<(), Stop> {
        let mut state = self.admit()?;
        if state.snapshot.completed != state.snapshot.total {
            return Err(Stop::InvalidState);
        }
        state.snapshot.stage = Stage::Publish;
        state.committed = true;
        drop(state);
        Ok(())
    }

    /// Publish the final condition after successful publication or job failure.
    ///
    /// # Errors
    ///
    /// Rejects invalid transitions or a poisoned state lock.
    pub fn finish(&self, success: bool) -> Result<(), String> {
        let mut state = self.lock()?;
        if success && state.committed {
            state.snapshot.state = JobState::Completed;
            state.committed = false;
        } else if !success
            && matches!(
                state.snapshot.state,
                JobState::Running | JobState::Pausing | JobState::Cancelling
            )
        {
            state.snapshot.state = JobState::Failed;
            state.committed = false;
        } else {
            return Err("invalid stack completion transition".to_owned());
        }
        drop(state);
        Ok(())
    }

    /// Restore reusable idle status after the cancelled job's resources are freed.
    ///
    /// # Errors
    ///
    /// Rejects unacknowledged cancellation or a poisoned state lock.
    pub fn settle_cancelled(&self) -> Result<(), String> {
        let mut state = self.lock()?;
        if state.snapshot.state != JobState::Cancelling {
            return Err("stack cancellation has not been acknowledged".to_owned());
        }
        state.snapshot.state = JobState::Idle;
        drop(state);
        Ok(())
    }

    /// Lock shared state without panicking if the owner failed.
    ///
    /// # Errors
    ///
    /// Returns the lock poisoning error.
    fn lock(&self) -> Result<MutexGuard<'_, State>, String> {
        self.shared
            .state
            .lock()
            .map_err(|_| "stack lifecycle lock poisoned".to_owned())
    }

    /// Admit work only after processing outstanding controls at a safe boundary.
    ///
    /// # Errors
    ///
    /// Reports a closed or cancelled job, or lock failure.
    fn admit(&self) -> Result<MutexGuard<'_, State>, Stop> {
        let mut state = self.shared.state.lock().map_err(|_| Stop::InvalidState)?;
        loop {
            match state.snapshot.state {
                JobState::Idle | JobState::Failed | JobState::Completed => {
                    return Err(Stop::Closed);
                }
                JobState::Cancelling => return Err(Stop::Cancelled),
                JobState::Running | JobState::Pausing | JobState::Paused => {}
            }
            if state.committed {
                return Err(Stop::Closed);
            }
            if state.cancel_requested {
                state.snapshot.state = JobState::Cancelling;
                state.snapshot.cancel_acknowledged = Some(Instant::now());
                return Err(Stop::Cancelled);
            }
            if state.snapshot.state == JobState::Running {
                return Ok(state);
            }
            if state.snapshot.state == JobState::Pausing {
                state.snapshot.state = JobState::Paused;
                state.snapshot.pause_acknowledged = Some(Instant::now());
                self.shared.changed.notify_all();
            }
            state = self
                .shared
                .changed
                .wait(state)
                .map_err(|_| Stop::InvalidState)?;
        }
    }
}
