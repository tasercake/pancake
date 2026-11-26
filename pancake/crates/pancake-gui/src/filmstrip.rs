//! Asynchronous, bounded source previews for the desktop filmstrip.

use std::error::Error;
use std::fmt;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use pancake::image_io::decode;
use slint::{ComponentHandle as _, Image, Model as _, ModelRc, VecModel, Weak};
use slint_interpreter::{ComponentInstance, Value};

use super::preview::Thumbnail;

#[cfg(test)]
mod tests;

/// One neighboring group fits normal desktop widths and high-DPI thumbnails.
const BAND_SIZE: usize = 24;
/// Coalesce sub-tile and neighboring scroll events into the same request.
const BAND_STEP: usize = 8;
/// Cover an ultra-wide viewport without retaining hundreds of decoded tiles.
const MAX_BAND: usize = 96;

/// Visible thumbnail model and its coalescing decode thread.
pub struct Filmstrip {
    /// Indexed placeholders and current downsampled images on the UI thread.
    model: Rc<VecModel<Image>>,
    /// Exact sources behind the current model's ordinals.
    paths: Vec<PathBuf>,
    /// Decoding is limited to this nearby viewport band.
    visible: Range<usize>,
    /// Generation invalidation and single in-flight pending selection.
    mailbox: Arc<Mailbox>,
}

impl Filmstrip {
    /// Attach the indexed model and start one event-driven decode thread.
    /// # Errors
    /// Returns a dynamic view binding or thread startup error.
    pub fn new(window: &ComponentInstance) -> Result<Self, Box<dyn Error>> {
        let model = Rc::new(VecModel::default());
        window.set_property("thumbnails", ModelRc::from(Rc::clone(&model)).into())?;
        let mailbox = Arc::new(Mailbox::default());
        let receiver = Arc::clone(&mailbox);
        let weak = window.as_weak();
        let _thread = thread::Builder::new()
            .name("pancake-filmstrip".into())
            .spawn(move || decode_requests(&receiver, &weak))?;
        Ok(Self {
            model,
            paths: Vec::new(),
            visible: 0..0,
            mailbox,
        })
    }

    /// Change the thumbnail ordinals only when actual inputs have changed.
    /// # Errors
    /// Returns a worker coordination error.
    pub fn update_sources(&mut self, paths: &[PathBuf]) -> Result<(), &'static str> {
        if paths == self.paths {
            return Ok(());
        }
        self.reset()?;
        self.paths = paths.to_vec();
        self.model.set_vec(vec![Image::default(); paths.len()]);
        self.request(0, 0)
    }

    /// Invalidate in-flight images when the current workspace is replaced.
    /// # Errors
    /// Returns a worker coordination error.
    pub fn reset(&mut self) -> Result<(), &'static str> {
        self.mailbox.submit(Vec::new())?;
        self.paths.clear();
        self.visible = 0..0;
        self.model.set_vec(Vec::new());
        Ok(())
    }

    /// Focus a frame already selected in the viewer or through a keyboard action.
    /// # Errors
    /// Returns a worker coordination error.
    pub fn focus(&mut self, index: usize) -> Result<(), &'static str> {
        if self.visible.contains(&index) {
            Ok(())
        } else {
            self.request(index, self.visible.len().saturating_sub(BAND_STEP))
        }
    }

    /// Request the current viewport band, retaining its already decoded images.
    /// # Errors
    /// Returns a worker coordination error.
    pub fn request(&mut self, first: usize, count: usize) -> Result<(), &'static str> {
        let band = view_band(first, self.paths.len(), count);
        if band == self.visible {
            return Ok(());
        }
        for ordinal in self
            .visible
            .clone()
            .filter(|ordinal| !band.contains(ordinal))
        {
            self.model.set_row_data(ordinal, Image::default());
        }
        let missing = band
            .clone()
            .filter(|&ordinal| {
                self.model
                    .row_data(ordinal)
                    .is_none_or(|item| item.size().width == 0)
            })
            .filter_map(|ordinal| self.paths.get(ordinal).map(|path| (ordinal, path.clone())))
            .collect();
        self.mailbox.submit(missing)?;
        self.visible = band;
        Ok(())
    }
}

impl fmt::Debug for Filmstrip {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Filmstrip")
            .field("paths", &self.paths)
            .field("visible", &self.visible)
            .finish_non_exhaustive()
    }
}

impl Drop for Filmstrip {
    fn drop(&mut self) {
        self.mailbox.close();
    }
}

/// A clear request generation prevents a replaced path reusing an ordinal.
#[derive(Debug)]
struct Request {
    /// Only this revision may publish samples for these ordinals.
    revision: u64,
    /// A contiguous band's missing images.
    frames: Vec<(usize, PathBuf)>,
}

/// Publication queue, with one pending selection even during rapid scroll.
#[derive(Debug, Default)]
struct Mailbox {
    /// Pending work and closure state shared by both threads.
    state: Mutex<MailboxState>,
    /// Wake the worker when a new request or closure arrives.
    ready: Condvar,
    /// Invalidate completed or pending samples without locking the UI callback.
    revision: AtomicU64,
}

/// Synchronized pending request and shutdown flag.
#[derive(Debug, Default)]
struct MailboxState {
    /// At most the newest decode request.
    pending: Option<Request>,
    /// A destroyed workspace no longer accepts decodes.
    closed: bool,
}

impl Mailbox {
    /// Get the revision accepted by the currently active view.
    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    /// Supersede older work, or just invalidate it when no frames are missing.
    /// # Errors
    /// Returns a poisoned synchronization state error.
    fn submit(&self, frames: Vec<(usize, PathBuf)>) -> Result<(), &'static str> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "The filmstrip queue is unavailable.")?;
        let revision = self.revision.fetch_add(1, Ordering::AcqRel).wrapping_add(1);
        state.pending = (!frames.is_empty()).then_some(Request { revision, frames });
        let available = state.pending.is_some();
        drop(state);
        if available {
            self.ready.notify_one();
        }
        Ok(())
    }

    /// Block until the native app requests work or closes.
    fn receive(&self) -> Option<Request> {
        let state = self.state.lock().ok()?;
        let mut state = self
            .ready
            .wait_while(state, |state| !state.closed && state.pending.is_none())
            .ok()?;
        state.pending.take()
    }

    /// Invalidate undelivered samples and wake the decoder for shutdown.
    fn close(&self) {
        let _prior = self.revision.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut state) = self.state.lock() {
            state.pending = None;
            state.closed = true;
            self.ready.notify_one();
        }
    }
}

/// Select a bounded band containing the first visible frame.
fn view_band(first: usize, count: usize, viewport_count: usize) -> Range<usize> {
    let start = (first.min(count.saturating_sub(1)) / BAND_STEP).saturating_mul(BAND_STEP);
    let band_size = viewport_count
        .saturating_add(BAND_STEP)
        .clamp(BAND_SIZE, MAX_BAND);
    start..start.saturating_add(band_size).min(count)
}

/// Decode real source rasters one at a time and push each accepted thumbnail.
fn decode_requests(mailbox: &Arc<Mailbox>, weak: &Weak<ComponentInstance>) {
    while let Some(request) = mailbox.receive() {
        for (ordinal, path) in request.frames {
            if mailbox.revision() != request.revision {
                break;
            }
            let Ok(thumbnail) = decode(&path).and_then(|frame| Thumbnail::new(&frame)) else {
                continue;
            };
            let delivery = Arc::clone(mailbox);
            if weak
                .upgrade_in_event_loop(move |window| {
                    if request.revision == delivery.revision()
                        && let Ok(Value::Model(model)) = window.get_property("thumbnails")
                    {
                        model.set_row_data(ordinal, thumbnail.into_image().into());
                    }
                })
                .is_err()
            {
                return;
            }
        }
    }
}
