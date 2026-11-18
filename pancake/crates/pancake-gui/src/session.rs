//! Shared-job execution and native source and result presentation.

use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use pancake::config::{AlignmentMode, BitDepth, DevicePreference, JobConfig, JobSettings, Method};
use pancake::image_io::{decode, export};
use pancake::job::{Generation, JobSnapshot, JobState, Options, StackResult, StackWorker};
use slint::{ComponentHandle, Image, ModelRc, SharedString, Timer, TimerMode, VecModel};
use slint_interpreter::{ComponentInstance, Value};

use super::panel::{self, Picker};
use super::preview::Preview;
use super::show_error;
use super::sources::{Sources, folder_files};

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
    results: Vec<StackResult>,
    /// Active result ordinal when the viewer displays a result.
    selected: Option<usize>,
    /// Sum of retained full-precision RGB bytes.
    retained_bytes: usize,
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
            .map(|result| SharedString::from(method_label(result.settings().method)))
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
        let canvas = result.canvas();
        let crop = result.crop();
        window.set_property("preview-x", u32::try_from(crop.x())?.into())?;
        window.set_property("preview-y", u32::try_from(crop.y())?.into())?;
        window.set_property("base-width", u32::try_from(canvas.width())?.into())?;
        window.set_property("base-height", u32::try_from(canvas.height())?.into())?;
        window.set_property(
            "selected-result-method",
            SharedString::from(method_id(result.settings().method)).into(),
        )?;
        window.set_property("selected-source", (-1_i32).into())?;
        window.set_property("selected-result", i32::try_from(index)?.into())?;
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
        self.results.push(result);
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
    options: Options,
) -> UiResult<Timer> {
    let mut session = Session {
        sources: Sources::default(),
        worker: StackWorker::new(options)?,
        generation: None,
        results: Vec::new(),
        selected: None,
        retained_bytes: 0,
    };
    if let Err(error) = session.sources.add(paths) {
        show_error(window, &error);
    }
    session.refresh_lists(window)?;
    if !session.sources.paths().is_empty()
        && let Err(error) = session.show_source(window, 0)
    {
        show_error(window, &error.to_string());
    }

    let session = Rc::new(RefCell::new(session));
    let actions: [(&str, Action); 11] = [
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
        ("close-request", close),
    ];
    for (name, action) in actions {
        register(window, &session, name, action)?;
    }
    let timer = Timer::default();
    let weak = window.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(16), move || {
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
    add_sources(session, window, &files)
}

/// Import supported paths below a selected native directory.
///
/// # Errors
///
/// Returns native panel, folder walk, or source errors.
fn import_folder(session: &mut Session, window: &ComponentInstance, _: &[Value]) -> UiResult {
    if let Some(folder) = panel::select(window, Picker::Folder)?.first() {
        add_sources(session, window, &folder_files(folder)?)?;
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
    match ordinal(args.first()) {
        Ok(index) => session.sources.remove(index)?,
        Err(_) => session.sources.clear(),
    }
    session.refresh_lists(window)?;
    if !session.sources.paths().is_empty() {
        session.show_source(window, 0)?;
    } else {
        window.set_property("preview", Image::default().into())?;
        window.set_property("preview-width", 0_i32.into())?;
        window.set_property("preview-height", 0_i32.into())?;
        window.set_property("selected-source", (-1_i32).into())?;
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
    let index = session.selected.ok_or("No computed result is selected.")?;
    let result = session
        .results
        .get(index)
        .ok_or("The selected result was removed.")?;
    if let Some(destination) = panel::select(window, Picker::Save(name))?.first() {
        export(
            destination,
            result.image(),
            depth,
            result.settings().quality,
            result.sources(),
        )?;
    }
    Ok(())
}

/// Close the real window event loop after the user requests it.
///
/// # Errors
///
/// Returns platform shutdown errors.
fn close(_: &mut Session, _: &ComponentInstance, _: &[Value]) -> UiResult {
    slint::quit_event_loop()?;
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
