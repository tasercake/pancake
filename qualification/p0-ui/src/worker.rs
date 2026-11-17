//! A finite real-compute worker with one command and one pending-preview slot.

use pancake_ui_qualification::{make_source, preview_rgba};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering},
    mpsc::{Receiver, SyncSender, TrySendError},
};

/// Working image width, bounded independently of the preview cache.
pub(crate) const WIDTH: usize = 2048;
/// Working image height.
pub(crate) const HEIGHT: usize = 1536;
/// A command is waiting for the worker.
pub(crate) const QUEUED: u8 = 1;
/// Worker is executing real image batches.
pub(crate) const RUNNING: u8 = 2;
/// Single-image preview completed.
pub(crate) const READY: u8 = 3;
/// Cancellation was accepted between completed batches.
pub(crate) const CANCELLED: u8 = 4;
/// Every requested batch completed normally.
pub(crate) const COMPLETE: u8 = 5;
/// A real compute or property operation failed.
pub(crate) const FAILED: u8 = 6;

/// Atomic observations, updated from real work rather than simulated progress.
#[derive(Debug, Default)]
pub(crate) struct Shared {
    /// Cancellation request, checked only between batches.
    pub(crate) cancel: AtomicBool,
    /// Current worker phase.
    pub(crate) state: AtomicU8,
    /// Number of completed readback batches.
    pub(crate) completed: AtomicU32,
    /// Number of actual calls into the shared image kernel.
    pub(crate) submitted: AtomicU32,
    /// Current accepted job; stale previews cannot replace a newer job.
    pub(crate) job: AtomicU32,
    /// Renderer has confirmed the same device and queue identities.
    pub(crate) interop: AtomicBool,
    /// Live full-precision source/result planes owned by this worker.
    pub(crate) planes_live: AtomicU32,
    /// Observed maximum full-precision plane count.
    pub(crate) planes_peak: AtomicU32,
    /// Live owned RGBA staging frames across producer, queue and presenter.
    pub(crate) rgba_live: AtomicU32,
    /// Observed maximum staging-frame count.
    pub(crate) rgba_peak: AtomicU32,
}

/// Finite requested work; no source-frame collection is queued.
#[derive(Debug)]
pub(crate) struct Command {
    /// Accepted job identity.
    pub(crate) job: u32,
    /// Checker polarity.
    pub(crate) inverted: bool,
    /// Number of actual image batches to process.
    pub(crate) batches: u32,
}

/// Explicit lifecycle messages; shutdown is independent of UI sender ownership.
#[derive(Debug)]
pub(crate) enum Message {
    /// Execute finite image work.
    Work(Command),
    /// Leave the worker after its current batch has drained.
    Shutdown,
}

/// One owned preview waiting for presentation.
#[derive(Debug)]
pub(crate) struct Frame {
    /// Source job identity.
    pub(crate) job: u32,
    /// Explicit bounded RGBA8 staging copy; source processing remains u16.
    pub(crate) rgba: Vec<u8>,
    /// Counter owner remains alive until the staging allocation is released.
    tracker: Arc<Shared>,
}

impl Frame {
    /// Account for a newly produced staging frame.
    fn new(job: u32, rgba: Vec<u8>, tracker: &Arc<Shared>) -> Self {
        let live = tracker
            .rgba_live
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let _old_peak = tracker.rgba_peak.fetch_max(live, Ordering::SeqCst);
        Self {
            job,
            rgba,
            tracker: Arc::clone(tracker),
        }
    }
}

impl Drop for Frame {
    fn drop(&mut self) {
        let _old_live = self.tracker.rgba_live.fetch_sub(1, Ordering::SeqCst);
    }
}

/// RAII accounting for full-precision planes, including recoverable error exits.
#[derive(Debug)]
struct Plane {
    /// Full u16 data, distinct from the lower-precision display copy.
    pixels: Vec<u16>,
    /// Counter lifetime owner.
    tracker: Arc<Shared>,
}

impl Plane {
    /// Retain one real plane and update the observed high-water count.
    fn new(pixels: Vec<u16>, tracker: &Arc<Shared>) -> Self {
        let live = tracker
            .planes_live
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let _old_peak = tracker.planes_peak.fetch_max(live, Ordering::SeqCst);
        Self {
            pixels,
            tracker: Arc::clone(tracker),
        }
    }
}

impl Drop for Plane {
    fn drop(&mut self) {
        let _old_live = self.tracker.planes_live.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Process work until explicit shutdown or channel close; no artificial waits.
pub(crate) fn run(
    device: &cubecl::Device,
    shared: &Arc<Shared>,
    commands: &Receiver<Message>,
    previews: &SyncSender<Frame>,
) {
    while let Ok(Message::Work(command)) = commands.recv() {
        shared.state.store(RUNNING, Ordering::SeqCst);
        if process(device, shared, &command, previews).is_err() {
            shared.state.store(FAILED, Ordering::SeqCst);
        }
    }
}

/// Complete genuine blur/readback work, observing cancellation before new work.
///
/// # Errors
///
/// Returns input, compute, readback, preview or closed-consumer errors.
fn process(
    device: &cubecl::Device,
    shared: &Arc<Shared>,
    command: &Command,
    previews: &SyncSender<Frame>,
) -> Result<(), String> {
    for batch in 0..command.batches {
        if shared.cancel.load(Ordering::SeqCst) {
            shared.state.store(CANCELLED, Ordering::SeqCst);
            return Ok(());
        }
        let input = Plane::new(
            make_source(WIDTH, HEIGHT, command.inverted ^ (batch & 1 != 0))?,
            shared,
        );
        let _previous = shared.submitted.fetch_add(1, Ordering::SeqCst);
        let output = Plane::new(
            pancake::blur_horizontal_u16(device, WIDTH, HEIGHT, &input.pixels)?,
            shared,
        );
        let frame = Frame::new(
            command.job,
            preview_rgba(&output.pixels, WIDTH, HEIGHT, 4)?,
            shared,
        );
        shared
            .completed
            .store(batch.saturating_add(1), Ordering::SeqCst);
        match previews.try_send(frame) {
            Ok(()) | Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => return Err("preview consumer closed".to_owned()),
        }
    }
    shared.state.store(
        if shared.cancel.load(Ordering::SeqCst) {
            CANCELLED
        } else if command.batches == 1 {
            READY
        } else {
            COMPLETE
        },
        Ordering::SeqCst,
    );
    Ok(())
}

/// Accept work only when no job is active; the channel itself has capacity one.
pub(crate) fn request(
    sender: &SyncSender<Message>,
    shared: &Shared,
    inverted: bool,
    batches: u32,
) -> bool {
    if matches!(shared.state.load(Ordering::SeqCst), QUEUED | RUNNING) {
        return false;
    }
    let job = shared.job.fetch_add(1, Ordering::SeqCst).saturating_add(1);
    shared.cancel.store(false, Ordering::SeqCst);
    shared.completed.store(0, Ordering::SeqCst);
    shared.submitted.store(0, Ordering::SeqCst);
    shared.state.store(QUEUED, Ordering::SeqCst);
    if sender
        .try_send(Message::Work(Command {
            job,
            inverted,
            batches,
        }))
        .is_err()
    {
        shared.state.store(FAILED, Ordering::SeqCst);
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{Frame, Plane, Shared};
    use std::sync::{Arc, atomic::Ordering, mpsc};

    /// Shutdown must not depend on the lifetime of retained UI callback senders.
    ///
    /// # Errors
    ///
    /// Returns thread/channel errors instead of hanging during window teardown.
    #[test]
    fn explicit_shutdown_ignores_retained_command_owners() -> Result<(), Box<dyn std::error::Error>>
    {
        let shared = Arc::new(Shared::default());
        let (sender, commands) = mpsc::sync_channel(1);
        let (previews, _receiver) = mpsc::sync_channel(1);
        let device = cubecl::Device::cpu()?;
        let worker = std::thread::spawn(move || {
            super::run(&device, &shared, &commands, &previews);
        });
        sender.send(super::Message::Shutdown)?;
        worker.join().map_err(|_| "worker shutdown failed")?;
        drop(sender);
        Ok(())
    }

    /// Full preview slots release rejected frames, and all owners release on close.
    ///
    /// # Panics
    ///
    /// Panics if actual RAII ownership differs from bounded queue/plane counts.
    #[test]
    fn bounded_slots_release_real_owned_allocations() {
        let shared = Arc::new(Shared::default());
        let (sender, receiver) = mpsc::sync_channel(1);
        drop(sender.try_send(Frame::new(1, vec![0; 4], &shared)));
        drop(sender.try_send(Frame::new(1, vec![255; 4], &shared)));
        assert_eq!(
            shared.rgba_live.load(Ordering::SeqCst),
            1,
            "only one queued preview may remain"
        );
        assert_eq!(
            shared.rgba_peak.load(Ordering::SeqCst),
            2,
            "queue plus producer is the observed peak"
        );
        drop(receiver);
        assert_eq!(
            shared.rgba_live.load(Ordering::SeqCst),
            0,
            "closing releases queued bytes"
        );
        let source = Plane::new(vec![0], &shared);
        let filtered = Plane::new(vec![65_535], &shared);
        assert_eq!(
            shared.planes_peak.load(Ordering::SeqCst),
            2,
            "source and filtered planes coexist"
        );
        drop((source, filtered));
        assert_eq!(
            shared.planes_live.load(Ordering::SeqCst),
            0,
            "full precision buffers are released"
        );
    }
}
