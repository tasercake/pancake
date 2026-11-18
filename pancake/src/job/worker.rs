//! One persistent stack worker with bounded request and result queues.

use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};

use crate::config::JobConfig;

use super::{
    Generation, JobControl, JobError, JobSnapshot, JobState, Options, StackResult, run_controlled,
};

/// One accepted request and its unique job controller.
#[derive(Debug)]
struct Work {
    /// Already validated ordered input paths and settings.
    config: JobConfig,
    /// Observable worker state for this request only.
    control: JobControl,
}

/// One finished stack or failure, keyed by the job that produced it.
#[derive(Debug)]
struct Outcome {
    /// Identity accepted before the worker was contacted.
    generation: Generation,
    /// Owned successful result or recoverable error.
    result: Result<StackResult, String>,
}

/// The UI thread's retained handle for the single worker.
#[derive(Debug)]
pub struct StackWorker {
    /// Current generation, strictly growing across accepted requests.
    counter: u64,
    /// Controller of the last accepted request, kept for final observations.
    current: Option<JobControl>,
    /// Whether the caller consumed the latest terminal outcome.
    delivered: bool,
    /// Bounded work queue, closed explicitly before joining.
    requests: Option<SyncSender<Work>>,
    /// Bounded transfer of the most recent completed result.
    outcomes: Receiver<Outcome>,
    /// Join handle owned by the initiating frontend.
    thread: Option<JoinHandle<()>>,
}

impl StackWorker {
    /// Start one dedicated compute worker for the lifetime of the session.
    ///
    /// # Errors
    ///
    /// Returns a thread startup error.
    pub fn new(options: Options) -> Result<Self, String> {
        let (requests, work) = mpsc::sync_channel(1);
        let (done, outcomes) = mpsc::sync_channel(1);
        let thread = thread::Builder::new()
            .name("pancake-stack".to_owned())
            .spawn(move || serve(&options, &work, &done))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            counter: 0,
            current: None,
            delivered: false,
            requests: Some(requests),
            outcomes,
            thread: Some(thread),
        })
    }

    /// Accept a validated immutable request while the worker is reusable.
    ///
    /// Consume any completed or failed outcome before starting the next stack.
    /// A cancelled request needs no result acknowledgement after it reaches Idle.
    ///
    /// # Errors
    ///
    /// Returns unavailable worker, active stack, outstanding result or overflow.
    pub fn start(&mut self, config: JobConfig) -> Result<Generation, String> {
        if let Some(last) = self.snapshot()?
            && !(last.state == JobState::Idle || self.delivered)
        {
            return Err("stack worker still owns an active or unread result".to_owned());
        }
        let counter = self
            .counter
            .checked_add(1)
            .ok_or("stack generation overflow")?;
        let generation = Generation::from_counter(counter);
        let control = JobControl::new(generation, config.inputs().len())?;
        self.requests
            .as_ref()
            .ok_or("stack worker is shut down")?
            .try_send(Work {
                config,
                control: control.clone(),
            })
            .map_err(|error| error.to_string())?;
        self.counter = counter;
        self.current = Some(control);
        self.delivered = false;
        Ok(generation)
    }

    /// Read real progress of the most recent accepted job.
    ///
    /// # Errors
    ///
    /// Returns a state synchronization error.
    pub fn snapshot(&self) -> Result<Option<JobSnapshot>, String> {
        self.current.as_ref().map(JobControl::snapshot).transpose()
    }

    /// Pause the active generation. Stale requests are ignored.
    ///
    /// # Errors
    ///
    /// Returns a state synchronization error.
    pub fn pause(&self, generation: Generation) -> Result<bool, String> {
        self.act(generation, JobControl::pause)
    }

    /// Resume the active generation. Stale requests are ignored.
    ///
    /// # Errors
    ///
    /// Returns a state synchronization error.
    pub fn resume(&self, generation: Generation) -> Result<bool, String> {
        self.act(generation, JobControl::resume)
    }

    /// Cancel the active generation. Stale requests are ignored.
    ///
    /// # Errors
    ///
    /// Returns a state synchronization error.
    pub fn cancel(&self, generation: Generation) -> Result<bool, String> {
        self.act(generation, JobControl::cancel)
    }

    /// Transfer the current final result or failure exactly once.
    ///
    /// # Errors
    ///
    /// Returns worker disconnection, synchronization or stale message errors.
    pub fn take_result(
        &mut self,
        generation: Generation,
    ) -> Result<Option<Result<StackResult, String>>, String> {
        if self.delivered || !self.is_current(generation)? {
            return Ok(None);
        }
        match self.outcomes.try_recv() {
            Ok(outcome) if outcome.generation == generation => {
                self.delivered = true;
                Ok(Some(outcome.result))
            }
            Ok(_) => Err("stack worker returned a stale result".to_owned()),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err("stack worker disconnected".to_owned()),
        }
    }

    /// Cancel outstanding work, release queued messages, and join the worker.
    ///
    /// # Errors
    ///
    /// Returns closed/poisoned state or abnormal thread termination diagnostics.
    pub fn shutdown(&mut self) -> Result<(), String> {
        let cancel = self.current.as_ref().map(JobControl::cancel).transpose();
        drop(self.requests.take());
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| "stack worker stopped unexpectedly")?;
        }
        cancel.map(|_| ())
    }

    /// Apply a user action only to the currently displayed generation.
    ///
    /// # Errors
    ///
    /// Returns a state synchronization error.
    fn act(
        &self,
        generation: Generation,
        operation: impl FnOnce(&JobControl) -> Result<bool, String>,
    ) -> Result<bool, String> {
        if let Some(control) = &self.current
            && control.snapshot()?.generation == generation
        {
            return operation(control);
        }
        Ok(false)
    }

    /// Check the generation before polling the bounded result channel.
    ///
    /// # Errors
    ///
    /// Returns a state synchronization error.
    fn is_current(&self, generation: Generation) -> Result<bool, String> {
        Ok(self
            .snapshot()?
            .is_some_and(|snapshot| snapshot.generation == generation))
    }
}

impl Drop for StackWorker {
    /// Do not strand a paused worker if the frontend tears down unexpectedly.
    fn drop(&mut self) {
        drop(self.shutdown());
    }
}

/// The dedicated thread consumes frames until the owning frontend closes it.
fn serve(options: &Options, work: &Receiver<Work>, done: &SyncSender<Outcome>) {
    while let Ok(request) = work.recv() {
        let Ok(initial) = request.control.snapshot() else {
            return;
        };
        let result = match run_controlled(request.config, options, &request.control, |_| {}) {
            Err(JobError::Cancelled) => continue,
            Err(error) => Err(error.to_string()),
            Ok(stack) => Ok(stack),
        };
        let outcome = Outcome {
            generation: initial.generation,
            result,
        };
        if done.send(outcome).is_err() {
            return;
        }
    }
}
