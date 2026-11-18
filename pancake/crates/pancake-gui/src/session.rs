//! Shared-job execution and native source and result presentation.

use std::cell::RefCell;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use pancake::config::{AlignmentMode, BitDepth, DevicePreference, JobConfig, JobSettings, Method};
use pancake::image_io::{RgbImage, decode, export};
use pancake::job::{Generation, JobSnapshot, JobState, Options, StackResult, StackWorker};
use pancake::registration::Bounds;
use slint::{
    CloseRequestResponse, ComponentHandle, Image, ModelRc, SharedString, Timer, TimerMode, VecModel,
};
use slint_interpreter::{ComponentInstance, Value};

use super::history::RecentFolders;
use super::panel::{self, Picker};
use super::preview::Preview;
use super::show_error;
use super::sources::{Sources, folder_files};
use super::{delivery, desktop_menu};

/// Recoverable native application errors.
type UiResult<T = ()> = Result<T, Box<dyn Error>>;

/// Every published view property describes the same workspace state.
type Action = fn(&mut Session, &ComponentInstance, &[Value]) -> UiResult;

/// A user session owns the reusable shared worker and immutable deliveries.
#[derive(Debug)]
struct Session {
    /// Imported paths in canonical processing order.
    sources: Sources,
    /// Real shared CLI/GUI compute engine.
    worker: StackWorker,
    /// Active worker generation until acknowledged or consumed.
    generation: Option<Generation>,
    /// Full-precision result ownership independent of any rendered preview.
    results: Vec<Retained>,
    /// Profile-local recently opened directories.
    history: RecentFolders,
    /// User transition awaiting an explicit confirmation.
    pending: Option<Transition>,
    /// Active result ordinal when the viewer displays a result.
    selected: Option<usize>,
    /// Sum of retained full-precision RGB bytes.
    retained_bytes: usize,
}

/// Immutable job delivery and optional same-sample crop.
#[derive(Debug)]
struct Retained {
    /// Original complete computation.
    stack: StackResult,
    /// Independently retained manual crop, when applied.
    manual: Option<(Bounds, RgbImage)>,
    /// Whether this image representation has been written successfully.
    saved: bool,
}

impl Retained {
    /// Current result pixels without transport or display quantization.
    fn image(&self) -> &RgbImage {
        self.manual
            .as_ref()
            .map_or_else(|| self.stack.image(), |(_, image)| image)
    }

    /// Current rectangle in the full result canvas.
    fn bounds(&self) -> Bounds {
        self.manual
            .as_ref()
            .map_or_else(|| self.stack.crop(), |(bounds, _)| *bounds)
    }

    /// Only offer a distinct, valid crop that has not already been applied.
    fn can_crop(&self) -> bool {
        self.manual.is_none()
            && self
                .stack
                .safe_crop()
                .is_some_and(|bounds| bounds != self.stack.crop())
    }
}

/// User-reversible workspace transition.
#[derive(Debug)]
enum Transition {
    /// Discard the current workspace and begin from empty.
    New,
    /// Close the application window.
    Close,
    /// Replace the workspace with one actual recent folder.
    Replace(PathBuf),
}

impl Session {
    /// Render names and available result choices from actual collections.
    ///
    /// # Errors
    ///
    /// Returns a Slint binding error.
    fn refresh_lists(&self, window: &ComponentInstance) -> UiResult {
        let names = self
            .sources
            .paths()
            .iter()
            .map(|path| {
                path.file_name().map_or_else(
                    || path.display().to_string().into(),
                    |name| SharedString::from(name.to_string_lossy().as_ref()),
                )
            })
            .collect::<Vec<_>>();
        let results = self
            .results
            .iter()
            .map(|result| SharedString::from(method_label(result.stack.settings().method)))
            .collect::<Vec<_>>();
        window.set_property("frames", ModelRc::new(VecModel::from(names)).into())?;
        window.set_property("results", ModelRc::new(VecModel::from(results)).into())?;
        Ok(())
    }

    /// Present a real decoded source without changing the viewer transform.
    ///
    /// # Errors
    ///
    /// Returns a missing source, decode, geometry, or dynamic binding error.
    fn show_source(&mut self, window: &ComponentInstance, index: usize) -> UiResult {
        let path = self
            .sources
            .paths()
            .get(index)
            .ok_or("Source no longer exists.")?;
        let preview = Preview::new(&decode(path)?)?;
        configure_image(window, &preview)?;
        window.set_property("base-width", preview.width.into())?;
        window.set_property("base-height", preview.height.into())?;
        window.set_property("preview-x", 0_i32.into())?;
        window.set_property("preview-y", 0_i32.into())?;
        window.set_property("selected-result-method", SharedString::default().into())?;
        window.set_property("selected-result", (-1_i32).into())?;
        window.set_property("selected-source", i32::try_from(index)?.into())?;
        window.set_property("result-can-crop", false.into())?;
        window.set_property("result-cropped", false.into())?;
        window.set_property("compare", false.into())?;
        self.selected = None;
        Ok(())
    }

    /// Present an immutable computed result at its true first-frame position.
    ///
    /// # Errors
    ///
    /// Returns missing result, geometry, or dynamic binding errors.
    fn show_result(&mut self, window: &ComponentInstance, index: usize) -> UiResult {
        let result = self.results.get(index).ok_or("Result no longer exists.")?;
        let preview = Preview::new(result.image())?;
        configure_image(window, &preview)?;
        let canvas = result.stack.canvas();
        let crop = result.bounds();
        window.set_property("preview-x", u32::try_from(crop.x())?.into())?;
        window.set_property("preview-y", u32::try_from(crop.y())?.into())?;
        window.set_property("base-width", u32::try_from(canvas.width())?.into())?;
        window.set_property("base-height", u32::try_from(canvas.height())?.into())?;
        window.set_property(
            "selected-result-method",
            SharedString::from(method_id(result.stack.settings().method)).into(),
        )?;
        window.set_property("selected-source", (-1_i32).into())?;
        window.set_property("selected-result", i32::try_from(index)?.into())?;
        window.set_property("result-can-crop", result.can_crop().into())?;
        window.set_property("result-cropped", (crop != canvas).into())?;
        window.set_property("compare", false.into())?;
        self.selected = Some(index);
        Ok(())
    }

    /// Admit a decoded result without evicting older results implicitly.
    ///
    /// # Errors
    ///
    /// Returns a retained-data limit or arithmetic error.
    fn retain(&mut self, result: StackResult) -> UiResult<usize> {
        let bytes = size_of_val(result.image().samples());
        let retained = self
            .retained_bytes
            .checked_add(bytes)
            .ok_or("Retained result size overflowed.")?;
        if retained > 2_147_483_648 {
            return Err("Retained result storage exceeds the workspace's 2 GiB limit.".into());
        }
        let index = self.results.len();
        self.results.push(Retained {
            stack: result,
            manual: None,
            saved: false,
        });
        self.retained_bytes = retained;
        Ok(index)
    }

    /// Observe real milestones and consume one matching completed calculation.
    ///
    /// # Errors
    ///
    /// Returns a worker failure or an inability to present its result.
    fn poll(&mut self, window: &ComponentInstance) -> UiResult {
        let Some(generation) = self.generation else {
            return Ok(());
        };
        let snapshot = self.worker.snapshot()?.ok_or("Stack disappeared.")?;
        if snapshot.generation != generation {
            return Err("Stack generation changed unexpectedly.".into());
        }
        render_status(window, snapshot)?;
        if snapshot.state == JobState::Idle {
            self.settle(window)?;
        } else if let Some(outcome) = self.worker.take_result(generation)? {
            self.settle(window)?;
            let index = self.retain(outcome?)?;
            self.refresh_lists(window)?;
            self.show_result(window, index)?;
        }
        Ok(())
    }

    /// Reenable controls only after worker-owned cleanup or result transfer.
    ///
    /// # Errors
    ///
    /// Returns dynamic binding errors.
    fn settle(&mut self, window: &ComponentInstance) -> UiResult {
        self.generation = None;
        window.set_property("running", false.into())?;
        window.set_property("can-pause", false.into())?;
        window.set_property("can-resume", false.into())?;
        window.set_property("can-cancel", false.into())?;
        Ok(())
    }
}

/// Set pixels before publishing the dimensions that make the canvas visible.
///
/// # Errors
///
/// Returns dynamic image and shape binding errors.
fn configure_image(window: &ComponentInstance, preview: &Preview) -> UiResult {
    window.set_property("preview", preview.picture.clone().into())?;
    window.set_property("preview-width", preview.width.into())?;
    window.set_property("preview-height", preview.height.into())?;
    Ok(())
}

/// Display the last worker-confirmed state and its permitted actions.
///
/// # Errors
///
/// Returns dynamic status/button binding errors.
fn render_status(window: &ComponentInstance, snapshot: JobSnapshot) -> UiResult {
    let state = snapshot.state;
    let message = format!(
        "{state:?} · {:?} · {} of {}",
        snapshot.stage, snapshot.completed, snapshot.total
    );
    window.set_property("stack-status", SharedString::from(message).into())?;
    window.set_property("can-pause", (state == JobState::Running).into())?;
    window.set_property("can-resume", (state == JobState::Paused).into())?;
    window.set_property(
        "can-cancel",
        matches!(
            state,
            JobState::Running | JobState::Pausing | JobState::Paused
        )
        .into(),
    )?;
    Ok(())
}

/// Register native UI actions and import the paths delivered to the executable.
///
/// # Errors
///
/// Returns dynamic binding or shared-worker startup errors.
pub(super) fn configure(
    window: &ComponentInstance,
    paths: &[PathBuf],
    profile: &Path,
    options: Options,
) -> UiResult<Timer> {
    let mut session = Session {
        sources: Sources::default(),
        worker: StackWorker::new(options)?,
        generation: None,
        results: Vec::new(),
        history: RecentFolders::open(profile)?,
        pending: None,
        selected: None,
        retained_bytes: 0,
    };
    render_recent(window, &session.history)?;
    if let Err(error) = session.sources.add(paths) {
        show_error(window, &error);
    }
    session.refresh_lists(window)?;
    if !session.sources.paths().is_empty() {
        if let Err(error) = session.show_source(window, 0) {
            show_error(window, &error.to_string());
        } else {
            remember_inputs(&mut session, window, paths)?;
        }
    }

    let session = Rc::new(RefCell::new(session));
    let actions: &[(&str, Action)] = &[
        ("select-source", select_source),
        ("select-result", select_result),
        ("import-files", import_files),
        ("import-folder", import_folder),
        ("remove-sources", remove_sources),
        ("stack", start),
        ("pause", pause),
        ("resume", resume),
        ("cancel", cancel),
        ("export-save", save),
        ("export-batch", save_batch),
        ("export-viewport", save_viewport),
        ("crop-result", crop_result),
        ("compare-preview", compare_preview),
        ("close-request", close),
        ("new-request", new),
        ("cancel-transition", cancel_transition),
        ("confirm-transition", confirm_transition),
        ("file-menu", file_menu),
        ("open-recent", open_recent),
        ("clear-recent", clear_recent),
        ("source-menu", source_menu),
        ("reveal-source", reveal_source),
    ];
    for &(name, action) in actions {
        register(window, &session, name, action)?;
    }
    let close_session = Rc::clone(&session);
    let close_window = window.as_weak();
    window.window().on_close_requested(move || {
        if let Some(window) = close_window.upgrade() {
            match close(&mut close_session.borrow_mut(), &window, &[]) {
                Ok(()) => {}
                Err(error) => show_error(&window, &error.to_string()),
            }
        }
        CloseRequestResponse::KeepWindowShown
    });
    let timer = Timer::default();
    let weak = window.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
        panel::service_events();
        if let Some(window) = weak.upgrade()
            && let Err(error) = session.borrow_mut().poll(&window)
        {
            show_error(&window, &error.to_string());
        }
    });
    Ok(timer)
}

/// Send actual native actions into the active workspace.
///
/// # Errors
///
/// Returns an invalid callback name or signature.
fn register(
    window: &ComponentInstance,
    session: &Rc<RefCell<Session>>,
    name: &str,
    action: Action,
) -> UiResult {
    let weak = window.as_weak();
    let session = Rc::clone(session);
    window.set_callback(name, move |arguments| {
        if let Some(window) = weak.upgrade()
            && let Err(error) = action(&mut session.borrow_mut(), &window, arguments)
        {
            show_error(&window, &error.to_string());
        }
        Value::Void
    })?;
    Ok(())
}

/// Only an actual current source ordinal can change the view.
///
/// # Errors
///
/// Returns invalid input, decoding, or view errors.
fn select_source(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    session.show_source(window, ordinal(args.first())?)
}

/// Only an actual retained result ordinal can change the view.
///
/// # Errors
///
/// Returns invalid index or view errors.
fn select_result(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    session.show_result(window, ordinal(args.first())?)
}

/// Import paths selected with the native image picker.
///
/// # Errors
///
/// Returns native panel, source validation, or decode errors.
fn import_files(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    let files = panel::select(window, Picker::Files)?;
    add_sources(session, window, &files)?;
    remember_inputs(session, window, &files)
}

/// Import supported paths below a selected native directory.
///
/// # Errors
///
/// Returns native panel, folder walk, or source errors.
fn import_folder(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    if let Some(folder) = panel::select(window, Picker::Folder)?.first() {
        let files = folder_files(folder)?;
        if !files.is_empty() {
            add_sources(session, window, &files)?;
            session.history.remember(folder)?;
            render_recent(window, &session.history)?;
        }
    }
    Ok(())
}

/// Apply a real source addition, then make the accepted set visible.
///
/// # Errors
///
/// Returns path validation, decoding, or display-binding errors.
fn add_sources(session: &mut Session, window: &ComponentInstance, files: &[PathBuf]) -> UiResult {
    if files.is_empty() {
        return Ok(());
    }
    session.sources.add(files)?;
    session.refresh_lists(window)?;
    session.show_source(window, 0)?;
    show_error(window, "");
    Ok(())
}

/// Discard selected sources and clear empty source presentation.
///
/// # Errors
///
/// Returns display-binding errors.
fn remove_sources(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    let argument = args.first().cloned().ok_or("Missing removal selection.")?;
    let ordinal = i32::try_from(argument).map_err(|_| "Removal selection must be an integer.")?;
    if ordinal == -1 {
        session.sources.clear();
    } else {
        session.sources.remove(usize::try_from(ordinal)?)?;
    }
    session.refresh_lists(window)?;
    if !session.sources.paths().is_empty() {
        session.show_source(window, 0)?;
    } else {
        session.selected = None;
        clear_image(window)?;
    }
    Ok(())
}

/// Capture a validated snapshot of the current native processing controls.
///
/// # Errors
///
/// Returns invalid fields and explicitly unrecognized default device settings.
fn settings(window: &ComponentInstance) -> UiResult<JobSettings> {
    let methods = [
        Method::Laplacian,
        Method::Hybrid,
        Method::WeightedAverage,
        Method::DepthMap,
        Method::ExposureFusion,
    ];
    let alignments = [
        AlignmentMode::Off,
        AlignmentMode::Translation,
        AlignmentMode::Euclidean,
        AlignmentMode::Similarity,
        AlignmentMode::Affine,
        AlignmentMode::Breathing,
    ];
    let method = *methods
        .get(ordinal(Some(&window.get_property("method-index")?))?)
        .ok_or("Invalid stacking method.")?;
    let alignment = *alignments
        .get(ordinal(Some(&window.get_property("alignment-index")?))?)
        .ok_or("Invalid alignment mode.")?;
    let device = match value::<i32>(window, "device-index")? {
        0 => std::env::var("PANCAKE_DEFAULT_DEVICE")
            .ok()
            .map_or(Ok(DevicePreference::Auto), |name| name.parse())?,
        1 => DevicePreference::Cpu,
        2 => DevicePreference::Auto,
        _ => return Err("Invalid processing device preference.".into()),
    };
    Ok(JobSettings {
        method,
        alignment,
        kernel_size: value::<SharedString>(window, "kernel-size")?.parse()?,
        pyramid_levels: value::<SharedString>(window, "pyramid-levels")?.parse()?,
        scale_factor: value::<SharedString>(window, "scale-factor")?.parse()?,
        auto_crop: value(window, "crop-enabled")?,
        device,
        ..JobSettings::default()
    })
}

/// Read and validate one typed public view property.
///
/// # Errors
///
/// Returns undeclared-property and mismatched-value errors.
fn value<T: TryFrom<Value>>(window: &ComponentInstance, key: &str) -> UiResult<T> {
    T::try_from(window.get_property(key)?).map_err(|_| format!("Invalid value for {key}.").into())
}

/// Confirm a toolkit integer communicated through a dynamic callback.
///
/// # Errors
///
/// Returns missing, mistyped or negative index diagnostics.
fn ordinal(argument: Option<&Value>) -> UiResult<usize> {
    let argument = argument.cloned().ok_or("Missing selection index.")?;
    let ordinal = i32::try_from(argument).map_err(|_| "Selection must be an integer.")?;
    Ok(usize::try_from(ordinal)?)
}

/// Submit the same validated immutable stack performed by the headless CLI.
///
/// # Errors
///
/// Returns stack settings, inputs or worker errors.
fn start(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    let config = JobConfig::new(session.sources.paths(), None, settings(window)?)?;
    session.generation = Some(session.worker.start(config)?);
    show_error(window, "");
    window.set_property("running", true.into())?;
    Ok(())
}

/// Request a pause on the real generation without predicting acknowledgement.
///
/// # Errors
///
/// Returns control synchronization errors.
fn pause(session: &mut Session, _: &ComponentInstance, _: &[Value]) -> UiResult {
    if let Some(generation) = session.generation {
        let _accepted = session.worker.pause(generation)?;
    }
    Ok(())
}

/// Resume only the currently retained worker generation.
///
/// # Errors
///
/// Returns control synchronization errors.
fn resume(session: &mut Session, _: &ComponentInstance, _: &[Value]) -> UiResult {
    if let Some(generation) = session.generation {
        let _accepted = session.worker.resume(generation)?;
    }
    Ok(())
}

/// Request cancellation and leave status to real worker observation.
///
/// # Errors
///
/// Returns control synchronization errors.
fn cancel(session: &mut Session, _: &ComponentInstance, _: &[Value]) -> UiResult {
    if let Some(generation) = session.generation {
        let _accepted = session.worker.cancel(generation)?;
    }
    Ok(())
}

/// Encode full-precision pixels using the shared atomic exporter.
///
/// # Errors
///
/// Returns invalid format or a native panel and publication error.
fn save(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    let formats = ["pancake.png", "pancake.jpg", "pancake.tif", "pancake.exr"];
    let name = formats
        .get(ordinal(args.first())?)
        .ok_or("Invalid export format.")?;
    let depth = if ordinal(args.get(1))? == 0 {
        BitDepth::Eight
    } else {
        BitDepth::Sixteen
    };
    let Some(destination) = panel::select(window, Picker::Save(name))?
        .into_iter()
        .next()
    else {
        session.pending = None;
        return Ok(());
    };
    let index = session.selected.ok_or("No computed result is selected.")?;
    let result = session
        .results
        .get_mut(index)
        .ok_or("The selected result was removed.")?;
    export(
        &destination,
        result.image(),
        depth,
        result.stack.settings().quality,
        result.stack.sources(),
    )?;
    result.saved = true;
    if matches!(session.pending, Some(Transition::Close))
        && session.results.iter().all(|result| result.saved)
    {
        slint::quit_event_loop()?;
    }
    session.pending = None;
    Ok(())
}

/// Save all supported batch integer formats using the same result pixels.
///
/// # Errors
///
/// Returns native selection or image persistence errors.
fn save_batch(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    let index = session.selected.ok_or("No computed result is selected.")?;
    let result = session
        .results
        .get_mut(index)
        .ok_or("The selected result was removed.")?;
    if let Some(folder) = panel::select(window, Picker::Folder)?.first() {
        delivery::batch(
            folder,
            result.image(),
            result.stack.sources(),
            result.stack.settings().quality,
        )?;
        result.saved = true;
    }
    Ok(())
}

/// Export the renderer's current viewport, independent of the result raster.
///
/// # Errors
///
/// Returns native selection, snapshot geometry or image persistence errors.
fn save_viewport(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    let visible = delivery::viewport(window, args)?;
    if let Some(path) = panel::select(window, Picker::Save("Viewport.png"))?.first() {
        export(path, &visible, BitDepth::Eight, 95, session.sources.paths())?;
    }
    Ok(())
}

/// Retain an exact cropped subset without discarding the original result.
///
/// # Errors
///
/// Returns shared crop, retention limit, or display errors.
fn crop_result(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    let index = session.selected.ok_or("No computed result is selected.")?;
    let result = session
        .results
        .get_mut(index)
        .ok_or("The selected result was removed.")?;
    if result.can_crop()
        && let (Some(bounds), Some(image)) =
            (result.stack.safe_crop(), result.stack.manually_cropped()?)
    {
        let retained = session
            .retained_bytes
            .checked_add(size_of_val(image.samples()))
            .ok_or("Retained crop size overflowed.")?;
        if retained > 2_147_483_648 {
            return Err("The cropped result would exceed the workspace's 2 GiB limit.".into());
        }
        result.manual = Some((bounds, image));
        result.saved = false;
        session.retained_bytes = retained;
        session.show_result(window, index)?;
    }
    Ok(())
}

/// Load the real comparison source and only then turn on split presentation.
///
/// # Errors
///
/// Returns unavailable sources, preview generation, or dynamic binding errors.
fn compare_preview(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    let result = session
        .selected
        .and_then(|index| session.results.get(index))
        .ok_or("No result is selected.")?;
    let source = result
        .stack
        .sources()
        .first()
        .ok_or("No comparison source is available.")?;
    let comparison = Preview::new(&decode(source)?)?;
    window.set_property("comparison-preview", comparison.picture.into())?;
    window.set_property("compare", true.into())?;
    Ok(())
}

/// Request a normal close or present a prompt for unsaved work.
///
/// # Errors
///
/// Returns a transition or platform shutdown error.
fn close(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    transition(session, window, Transition::Close)
}

/// Request an empty workspace through the same confirmation boundary.
///
/// # Errors
///
/// Returns native binding or workspace transition errors.
fn new(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    transition(session, window, Transition::New)
}

/// Undo a pending prompt without changing the current workspace.
///
/// # Errors
///
/// Returns dynamic binding errors.
fn cancel_transition(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    session.pending = None;
    set_prompt(window, "")
}

/// Commit a transition explicitly acknowledged by the user.
///
/// # Errors
///
/// Returns actual workspace cleanup, import or shutdown errors.
fn confirm_transition(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    if let Some(action) = session.pending.take() {
        set_prompt(window, "")?;
        apply_transition(session, window, action)?;
    }
    Ok(())
}

/// Ask before changing unsaved work or replacing an existing source set.
///
/// # Errors
///
/// Returns active-job, view state or transition errors.
fn transition(session: &mut Session, window: &ComponentInstance, action: Transition) -> UiResult {
    if session.generation.is_some() {
        if matches!(action, Transition::Close) {
            return slint::quit_event_loop().map_err(Into::into);
        }
        return Err("Wait for the running stack to finish or stop it first.".into());
    }
    let unsaved = session.results.iter().any(|result| !result.saved);
    let prompt = match action {
        Transition::New if unsaved => Some("new"),
        Transition::Close if unsaved => Some("close"),
        Transition::Replace(_) if unsaved || !session.sources.paths().is_empty() => Some("replace"),
        _ => None,
    };
    if let Some(prompt) = prompt {
        session.pending = Some(action);
        set_prompt(window, prompt)
    } else {
        apply_transition(session, window, action)
    }
}

/// Expose the actual prompt after a user-initiated transition.
///
/// # Errors
///
/// Returns a dynamic view binding error.
fn set_prompt(window: &ComponentInstance, prompt: &str) -> UiResult {
    window.set_property("prompt", SharedString::from(prompt).into())?;
    Ok(())
}

/// Apply an accepted workspace transition and clear only replaced workspace state.
///
/// # Errors
///
/// Returns native view, directory enumeration or shutdown errors.
fn apply_transition(
    session: &mut Session,
    window: &ComponentInstance,
    action: Transition,
) -> UiResult {
    if matches!(action, Transition::Close) {
        slint::quit_event_loop()?;
        return Ok(());
    }
    let mut new_sources = Sources::default();
    if let Transition::Replace(folder) = action {
        let files = folder_files(&folder)?;
        if files.is_empty() {
            return Err("The recent folder has no supported images.".into());
        }
        new_sources.add(&files)?;
        session.history.remember(&folder)?;
    }
    session.sources = new_sources;
    session.selected = None;
    session.results.clear();
    session.retained_bytes = 0;
    clear_image(window)?;
    session.refresh_lists(window)?;
    render_recent(window, &session.history)?;
    if !session.sources.paths().is_empty() {
        session.show_source(window, 0)?;
    }
    Ok(())
}

/// Reset presentation to the actual empty source/selection state.
///
/// # Errors
///
/// Returns dynamic view binding errors.
fn clear_image(window: &ComponentInstance) -> UiResult {
    window.set_property("preview", Image::default().into())?;
    window.set_property("preview-width", 0_i32.into())?;
    window.set_property("preview-height", 0_i32.into())?;
    window.set_property("selected-source", (-1_i32).into())?;
    window.set_property("selected-result", (-1_i32).into())?;
    window.set_property("result-can-crop", false.into())?;
    window.set_property("result-cropped", false.into())?;
    window.set_property("compare", false.into())?;
    Ok(())
}

/// Stable semantic method identifier for the currently shown result.
const fn method_id(method: Method) -> &'static str {
    match method {
        Method::Laplacian => "laplacian",
        Method::Hybrid => "hybrid",
        Method::WeightedAverage => "weighted_average",
        Method::DepthMap => "depth_map",
        Method::ExposureFusion => "exposure_fusion",
    }
}

/// Localized display label for a method in the retained result gallery.
const fn method_label(method: Method) -> &'static str {
    match method {
        Method::Laplacian => "Laplacian",
        Method::Hybrid => "Hybrid",
        Method::WeightedAverage => "Weighted average",
        Method::DepthMap => "Depth map",
        Method::ExposureFusion => "Exposure fusion",
    }
}

/// Publish names derived from the committed folder history.
///
/// # Errors
///
/// Returns a dynamic binding error.
fn render_recent(window: &ComponentInstance, history: &RecentFolders) -> UiResult {
    let names = history
        .entries()
        .iter()
        .map(|path| {
            SharedString::from(
                path.file_name()
                    .map_or_else(|| path.as_os_str(), |name| name)
                    .to_string_lossy()
                    .as_ref(),
            )
        })
        .collect::<Vec<_>>();
    window.set_property("recent-folders", ModelRc::new(VecModel::from(names)).into())?;
    Ok(())
}

/// Add real imported directories to this profile's recent history.
///
/// # Errors
///
/// Returns native state, file access or history persistence errors.
fn remember_inputs(
    session: &mut Session,
    window: &ComponentInstance,
    paths: &[PathBuf],
) -> UiResult {
    let current: std::collections::BTreeSet<_> = paths
        .iter()
        .filter_map(|path| path.canonicalize().ok())
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();
    for folder in current {
        session.history.remember(&folder)?;
    }
    render_recent(window, &session.history)
}

/// Open the Linux native File menu with actual recent entries.
///
/// # Errors
///
/// Returns native menu, transition, or import errors.
#[cfg(target_os = "linux")]
fn file_menu(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    use desktop_menu::FileAction;
    match desktop_menu::show_file(session.history.entries())? {
        Some(FileAction::OpenFiles) => import_files(session, window, &[]),
        Some(FileAction::OpenFolder) => import_folder(session, window, &[]),
        Some(FileAction::OpenRecent(path)) => {
            transition(session, window, Transition::Replace(path))
        }
        Some(FileAction::ClearRecent) => clear_recent(session, window, &[]),
        Some(FileAction::Close) => close(session, window, &[]),
        None => Ok(()),
    }
}

/// Open the in-window menu when a native MenuBar is unavailable.
///
/// # Errors
///
/// Returns a dynamic view binding error.
#[cfg(not(target_os = "linux"))]
fn file_menu(_: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    window.set_property("file-open", true.into())?;
    Ok(())
}

/// Read the selected real recent directory before requesting replacement.
///
/// # Errors
///
/// Returns invalid index or replacement errors.
fn open_recent(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    let path = session
        .history
        .entries()
        .get(ordinal(args.first())?)
        .cloned()
        .ok_or("The recent folder was removed.")?;
    transition(session, window, Transition::Replace(path))
}

/// Reset only the active profile's real folder history.
///
/// # Errors
///
/// Returns persistence or view errors.
fn clear_recent(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    session.history.clear()?;
    render_recent(window, &session.history)
}

/// Use the desktop context menu for an actual source row.
///
/// # Errors
///
/// Returns GTK, file operation or selection errors.
#[cfg(target_os = "linux")]
fn source_menu(session: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    use desktop_menu::SourceAction;
    match desktop_menu::show_source()? {
        Some(SourceAction::Remove) => remove_sources(session, window, args),
        Some(SourceAction::Reveal) => reveal_source(session, window, args),
        None => Ok(()),
    }
}

/// Present the selected row's native context actions.
///
/// # Errors
///
/// Returns dynamic view or index-format errors.
#[cfg(not(target_os = "linux"))]
fn source_menu(_: &mut Session, window: &ComponentInstance, args: &[Value]) -> UiResult {
    window.set_property(
        "context-index",
        i32::try_from(ordinal(args.first())?)?.into(),
    )?;
    Ok(())
}

/// Reveal the actual source by handing it to the platform file manager.
///
/// # Errors
///
/// Returns stale selection or native file manager errors.
fn reveal_source(session: &mut Session, _: &ComponentInstance, args: &[Value]) -> UiResult {
    let path = session
        .sources
        .paths()
        .get(ordinal(args.first())?)
        .ok_or("Source no longer exists.")?;
    desktop_menu::reveal(path)?;
    Ok(())
}
