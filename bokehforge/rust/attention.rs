//! Native sessions for the pretrained Inspection blend.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};

use numpy::ndarray::Array2;
use numpy::{IntoPyArray, PyArray2, PyReadonlyArray3, PyUntypedArrayMethods};
use ort::ep::{ArenaExtendStrategy, CUDA, CoreML, ExecutionProvider, coreml::ComputeUnits};
use ort::session::{Session, builder::SessionBuilder};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use sha2::{Digest, Sha256};

use crate::{c_slice, parallel};

pub(crate) mod image;
mod saliency;
mod sam2;
#[cfg(test)]
mod tests;

const EXPORT_VERSION: &str = "onnx-v3";
const GRAPH_FILES: &[&str] = &[
    "reliqs.onnx",
    "tempsal.onnx",
    "sam_encoder.onnx",
    "sam_decoder.onnx",
];

static ATTENTION: Mutex<Option<Runtime>> = Mutex::new(None);

fn lock_runtime() -> Result<MutexGuard<'static, Option<Runtime>>, &'static str> {
    if crate::WORKERS.current_thread_index().is_some() {
        return Err("Inspection inference requires an external caller thread");
    }
    ATTENTION
        .lock()
        .map_err(|_| "Inspection runtime lock was poisoned")
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum Backend {
    Cpu,
    Cuda,
    CoreMl,
}

impl Backend {
    fn select(device: &str) -> Result<Self, String> {
        let accelerator = if cfg!(target_vendor = "apple") {
            Self::CoreMl
        } else {
            Self::Cuda
        };
        match device {
            "cpu" => Ok(Self::Cpu),
            "cuda" => Ok(Self::Cuda),
            "mps" => Ok(Self::CoreMl),
            "gpu" => Ok(accelerator),
            "auto" => {
                let available = match accelerator {
                    Self::Cuda => CUDA::default().is_available(),
                    Self::CoreMl => CoreML::default().is_available(),
                    Self::Cpu => unreachable!(),
                }
                .map_err(|error| {
                    format!("Could not query Inspection execution providers: {error}")
                })?;
                Ok(if available { accelerator } else { Self::Cpu })
            }
            _ => Err(format!("Unknown Inspection device {device:?}")),
        }
    }
}

#[derive(Eq, PartialEq)]
struct Configuration {
    original_models: PathBuf,
    library: Option<PathBuf>,
    python: PathBuf,
}

#[derive(Clone)]
struct PreviewMap {
    width: usize,
    height: usize,
    pixels: Arc<[f32]>,
    digest: String,
}

struct CachedMap {
    preview_key: [u8; 32],
    map: PreviewMap,
}

struct Models {
    reliqs: Session,
    tempsal: Session,
    sam2: sam2::Sam2,
    last: Option<CachedMap>,
}

struct Runtime {
    config: Configuration,
    exports: Option<PathBuf>,
    initialized: bool,
    auto_backend: Option<Backend>,
    models: HashMap<Backend, Models>,
}

impl Runtime {
    fn new(config: Configuration) -> Self {
        Self {
            config,
            exports: None,
            initialized: false,
            auto_backend: None,
            models: HashMap::new(),
        }
    }

    fn initialize(&mut self) -> Result<(), String> {
        if !self.initialized {
            let library = self.config.library.as_ref().ok_or_else(|| {
                "Install BokehForge with the inspection extra to enable ONNX Runtime.".to_owned()
            })?;
            ort::init_from(library)
                .map_err(|error| format!("Could not load ONNX Runtime: {error}"))?
                .with_name("BokehForge Inspection")
                .commit();
            self.initialized = true;
        }
        Ok(())
    }

    fn load(&mut self, backend: Backend) -> Result<(), String> {
        if self.models.contains_key(&backend) {
            return Ok(());
        }
        if backend == Backend::Cuda {
            preload_cuda(self.config.library.as_deref().expect("runtime initialized"))?;
        }
        if self.exports.is_none() {
            self.exports = Some(exported_models(&self.config)?);
        }
        let path = self.exports.as_ref().expect("model paths initialized");
        let models = Models {
            reliqs: load_session(&path.join("reliqs.onnx"), backend, false)?,
            tempsal: load_session(&path.join("tempsal.onnx"), backend, false)?,
            sam2: sam2::Sam2::load(path, backend)?,
            last: None,
        };
        self.models.insert(backend, models);
        Ok(())
    }

    fn models(&mut self, device: &str) -> Result<(&mut Models, Backend), String> {
        self.initialize()?;
        let backend = if device == "auto" {
            if let Some(backend) = self.auto_backend {
                backend
            } else {
                let preferred = Backend::select(device)?;
                let backend = match self.load(preferred) {
                    Ok(()) => preferred,
                    Err(_) if preferred != Backend::Cpu => {
                        self.load(Backend::Cpu)?;
                        Backend::Cpu
                    }
                    Err(error) => return Err(error),
                };
                self.auto_backend = Some(backend);
                backend
            }
        } else {
            let backend = Backend::select(device)?;
            self.load(backend)?;
            backend
        };
        Ok((
            self.models.get_mut(&backend).expect("models loaded"),
            backend,
        ))
    }
}

fn session_builder(backend: Backend, allow_tf32: bool) -> ort::Result<SessionBuilder> {
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8);
    let builder = Session::builder()?
        .with_intra_threads(workers)?
        .with_intra_op_spinning(false)?;
    Ok(match backend {
        Backend::Cpu => builder,
        Backend::Cuda => builder.with_execution_providers([CUDA::default()
            .with_tf32(allow_tf32)
            .with_arena_extend_strategy(ArenaExtendStrategy::SameAsRequested)
            .build()
            .error_on_failure()])?,
        Backend::CoreMl => builder.with_execution_providers([CoreML::default()
            .with_compute_units(ComputeUnits::All)
            .build()
            .error_on_failure()])?,
    })
}

fn load_session(path: &Path, backend: Backend, allow_tf32: bool) -> Result<Session, String> {
    let build = || session_builder(backend, allow_tf32)?.commit_from_file(path);
    build().map_err(|error| {
        format!(
            "Could not load {} using {backend:?}: {error}",
            path.display()
        )
    })
}

#[cfg(target_os = "linux")]
fn preload_cuda(library: &Path) -> Result<(), String> {
    // NVIDIA's wheels locate each component in a separate package directory.
    // System installations are located by the dynamic linker itself.
    let Some(packages) = library
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    else {
        return Ok(());
    };
    let cuda = packages.join("nvidia");
    for (package, name) in [
        ("cuda_runtime", "libcudart.so.12"),
        ("nvjitlink", "libnvJitLink.so.12"),
        ("cublas", "libcublasLt.so.12"),
        ("cublas", "libcublas.so.12"),
        ("cufft", "libcufft.so.11"),
        ("curand", "libcurand.so.10"),
        ("cusparse", "libcusparse.so.12"),
        ("cudnn", "libcudnn.so.9"),
    ] {
        let path = cuda.join(package).join("lib").join(name);
        if path.is_file() {
            ort::util::preload_dylib(&path).map_err(|error| {
                format!("Could not load CUDA library {}: {error}", path.display())
            })?;
        }
    }
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn preload_cuda(_: &Path) -> Result<(), String> {
    Ok(())
}

fn exported_models(config: &Configuration) -> Result<PathBuf, String> {
    let folder = config.original_models.join(EXPORT_VERSION);
    if verify_files(&folder).is_ok() {
        return Ok(folder);
    }
    let output = Command::new(&config.python)
        .args(["-m", "bokehforge._inspection_export"])
        .arg(&config.original_models)
        .arg(EXPORT_VERSION)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .map_err(|error| format!("Could not start the Inspection model exporter: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Could not prepare the Inspection ONNX models: {}",
            stderr.trim()
        ));
    }
    verify_files(&folder)?;
    Ok(folder)
}

fn verify_files(folder: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(folder.join("files.sha256"))
        .map_err(|error| format!("Could not read Inspection model manifest: {error}"))?;
    let mut seen = HashSet::new();
    for line in manifest.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        let [name, len, expected_sha] = fields.as_slice() else {
            return Err("Invalid Inspection model manifest".into());
        };
        if !matches!(
            Path::new(name).components().collect::<Vec<_>>().as_slice(),
            [Component::Normal(_)]
        ) || !seen.insert(*name)
        {
            return Err("Invalid Inspection model filename".into());
        }
        let path = folder.join(name);
        let mut file = File::open(&path).map_err(|error| {
            format!(
                "Could not open Inspection model {}: {error}",
                path.display()
            )
        })?;
        let mut hash = Sha256::new();
        let bytes = std::io::copy(&mut file, &mut hash).map_err(|error| {
            format!(
                "Could not verify Inspection model {}: {error}",
                path.display()
            )
        })?;
        if len.parse::<u64>() != Ok(bytes) || format!("{:x}", hash.finalize()) != *expected_sha {
            return Err(format!(
                "Inspection model {} failed checksum verification",
                path.display()
            ));
        }
    }
    if GRAPH_FILES.iter().any(|file| !seen.contains(file)) {
        return Err("The Inspection model cache is incomplete".into());
    }
    Ok(())
}

type SamMasks = (Vec<Vec<bool>>, Vec<f32>);
type Predictions = (Vec<f32>, SamMasks, Vec<f32>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum InferenceLane {
    Shared,
    External,
    Serial,
}

pub(crate) fn cuda_allocation_failure(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    [
        "cuda_error_out_of_memory",
        "out of device memory",
        "out of memory",
        "failed to allocate memory for requested buffer",
        "cudnn_status_alloc_failed",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn run_predictions(
    reliqs: impl FnOnce() -> Result<Vec<f32>, String> + Send,
    sam: impl FnOnce() -> Result<SamMasks, String> + Send,
    tempsal: impl FnOnce() -> Result<Vec<f32>, String> + Send,
    concurrent: bool,
) -> Result<Predictions, String> {
    run_model_lanes(
        reliqs,
        sam,
        tempsal,
        if concurrent {
            InferenceLane::Shared
        } else {
            InferenceLane::Serial
        },
    )
}

fn run_model_lanes(
    reliqs: impl FnOnce() -> Result<Vec<f32>, String> + Send,
    sam: impl FnOnce() -> Result<SamMasks, String> + Send,
    tempsal: impl FnOnce() -> Result<Vec<f32>, String> + Send,
    lane: InferenceLane,
) -> Result<Predictions, String> {
    if lane == InferenceLane::Serial {
        return Ok((reliqs()?, sam()?, tempsal()?));
    }
    let saliency = || reliqs().map(|reliqs| (reliqs, tempsal()));
    let (saliency, masks) = if lane == InferenceLane::External {
        std::thread::scope(|scope| {
            let masks = scope.spawn(sam);
            (
                saliency(),
                masks
                    .join()
                    .unwrap_or_else(|_| Err("Inspection SAM lane panicked".to_owned())),
            )
        })
    } else {
        rayon::join(saliency, sam)
    };
    // Resolve errors in model order after both branches finish.
    let (reliqs, temporal) = saliency?;
    Ok((reliqs, masks?, temporal?))
}

fn model_predictions(
    models: &mut Models,
    view: &image::Rgb8,
    concurrent: bool,
    lane: InferenceLane,
) -> Result<Predictions, String> {
    let reliqs = || saliency::reliqs(&mut models.reliqs, view);
    let sam = || models.sam2.masks(view);
    let tempsal = || saliency::tempsal(&mut models.tempsal, view);
    if concurrent && lane == InferenceLane::External {
        run_model_lanes(reliqs, sam, tempsal, lane)
    } else {
        run_predictions(reliqs, sam, tempsal, concurrent)
    }
}

fn preview(
    target: &[f32],
    height: usize,
    width: usize,
    device: &str,
) -> Result<PreviewMap, String> {
    preview_on_lane(target, height, width, device, InferenceLane::Shared)
}

fn preview_on_lane(
    target: &[f32],
    height: usize,
    width: usize,
    device: &str,
    lane: InferenceLane,
) -> Result<PreviewMap, String> {
    if width == 0
        || height == 0
        || width.checked_mul(height).and_then(|n| n.checked_mul(3)) != Some(target.len())
    {
        return Err("Expected a nonempty packed RGB reference".into());
    }
    let view = image::reference_preview(target, width, height);
    let mut hasher = Sha256::new();
    hasher.update(view.width.to_le_bytes());
    hasher.update(view.height.to_le_bytes());
    hasher.update(&view.pixels);
    let preview_key = hasher.finalize().into();
    let mut runtime = lock_runtime()?;
    let runtime = runtime
        .as_mut()
        .ok_or("Inspection runtime has not been configured")?;
    let (models, backend) = runtime.models(device)?;
    if let Some(cached) = &models.last
        && cached.preview_key == preview_key
    {
        return Ok(cached.map.clone());
    }

    let concurrent = lane != InferenceLane::Serial
        && backend == Backend::Cuda
        && crate::WORKERS.current_num_threads() >= 4;
    let (detail, predictions) = if lane == InferenceLane::External {
        let mut detail = None;
        let predictions = crate::WORKERS.in_place_scope(|cpu| {
            cpu.spawn(|_| detail = Some(image::detail_light(&view)));
            model_predictions(models, &view, concurrent, lane)
        });
        (detail.expect("Inspection detail completed"), predictions)
    } else {
        parallel(|| {
            rayon::join(
                || image::detail_light(&view),
                || model_predictions(models, &view, concurrent, lane),
            )
        })
    };
    let (reliqs, (masks, scores), temporal) = match predictions {
        Err(error)
            if concurrent && lane != InferenceLane::External && cuda_allocation_failure(&error) =>
        {
            // The paired calls have ended and dropped their transient outputs.
            // Keep the same provider and graphs on the serial retry.
            model_predictions(models, &view, false, InferenceLane::Serial)?
        }
        result => result?,
    };
    let pixels = view.width * view.height;
    if [reliqs.len(), temporal.len(), detail.len()]
        .into_iter()
        .any(|len| len != pixels)
        || masks.len() != scores.len()
        || masks.iter().any(|mask| mask.len() != pixels)
    {
        return Err("Inspection models produced an invalid output shape".into());
    }
    let mask_heat = image::mask_heat(&masks, &scores, &detail);
    let raw = image::blend_components([&reliqs, &detail, &mask_heat, &temporal]);
    if raw.iter().any(|value| !(0.0..=1.0).contains(value)) {
        return Err("Inspection models produced values outside the valid range".into());
    }
    let mut hasher = Sha256::new();
    for value in &raw {
        hasher.update(value.to_le_bytes());
    }
    let map = PreviewMap {
        width: view.width,
        height: view.height,
        pixels: raw.into(),
        digest: format!("{:x}", hasher.finalize()),
    };
    models.last = Some(CachedMap {
        preview_key,
        map: map.clone(),
    });
    Ok(map)
}

pub(crate) fn cuda_warmed(device: &str, width: usize, height: usize) -> bool {
    let Ok(runtime) = ATTENTION.try_lock() else {
        return false;
    };
    let Some(runtime) = runtime.as_ref() else {
        return false;
    };
    if !(device == "cuda" || (device == "auto" && runtime.auto_backend == Some(Backend::Cuda))) {
        return false;
    }
    let [width, height] = image::reference_dimensions(width, height);
    runtime
        .models
        .get(&Backend::Cuda)
        .and_then(|models| models.last.as_ref())
        .is_some_and(|cached| cached.map.width == width && cached.map.height == height)
}

pub(crate) fn generate(
    target: &[f32],
    height: usize,
    width: usize,
    device: &str,
) -> Result<(Vec<f32>, String), String> {
    generate_on_lane(target, height, width, device, InferenceLane::Shared)
}

pub(crate) fn generate_external(
    target: &[f32],
    height: usize,
    width: usize,
    device: &str,
) -> Result<(Vec<f32>, String), String> {
    generate_on_lane(target, height, width, device, InferenceLane::External)
}

pub(crate) fn generate_serial(
    target: &[f32],
    height: usize,
    width: usize,
    device: &str,
) -> Result<(Vec<f32>, String), String> {
    generate_on_lane(target, height, width, device, InferenceLane::Serial)
}

fn generate_on_lane(
    target: &[f32],
    height: usize,
    width: usize,
    device: &str,
    lane: InferenceLane,
) -> Result<(Vec<f32>, String), String> {
    let map = preview_on_lane(target, height, width, device, lane)?;
    let pixels = if (map.width, map.height) == (width, height) {
        map.pixels.to_vec()
    } else {
        image::resize_map_bilinear(&map.pixels, map.width, map.height, width, height)
    };
    Ok((pixels, map.digest))
}

#[pyfunction]
#[pyo3(signature = (original_models, library, python))]
fn configure_attention(
    original_models: PathBuf,
    library: Option<PathBuf>,
    python: PathBuf,
) -> PyResult<()> {
    let config = Configuration {
        original_models,
        library,
        python,
    };
    let mut runtime = lock_runtime().map_err(PyRuntimeError::new_err)?;
    if let Some(runtime) = runtime.as_ref() {
        if runtime.config != config {
            return Err(PyRuntimeError::new_err(
                "Inspection runtime was already configured",
            ));
        }
    } else {
        *runtime = Some(Runtime::new(config));
    }
    Ok(())
}

fn as_numpy<'py>(py: Python<'py>, map: &PreviewMap) -> Bound<'py, PyArray2<f32>> {
    Array2::from_shape_vec((map.height, map.width), map.pixels.to_vec())
        .expect("inspection map dimensions checked")
        .into_pyarray(py)
}

#[pyfunction]
fn inspection_preview<'py>(
    py: Python<'py>,
    target: PyReadonlyArray3<'py, f32>,
    device: &str,
) -> PyResult<(Bound<'py, PyArray2<f32>>, String)> {
    let &[height, width, channels] = target.shape() else {
        unreachable!("PyReadonlyArray3 has exactly three axes");
    };
    let samples = c_slice(&target)?;
    if channels != 3 || samples.iter().any(|value| !value.is_finite()) {
        return Err(PyValueError::new_err("Expected a finite RGB reference"));
    }
    let map = py
        .detach(|| preview(samples, height, width, device))
        .map_err(PyRuntimeError::new_err)?;
    Ok((as_numpy(py, &map), map.digest))
}

#[pyfunction]
fn inspection_generate<'py>(
    py: Python<'py>,
    target: PyReadonlyArray3<'py, f32>,
    device: &str,
) -> PyResult<(Bound<'py, PyArray2<f32>>, String)> {
    let &[height, width, channels] = target.shape() else {
        unreachable!("PyReadonlyArray3 has exactly three axes");
    };
    let samples = c_slice(&target)?;
    if channels != 3 || samples.iter().any(|value| !value.is_finite()) {
        return Err(PyValueError::new_err("Expected a finite RGB reference"));
    }
    let (map, digest) = py
        .detach(|| generate(samples, height, width, device))
        .map_err(PyRuntimeError::new_err)?;
    let array = Array2::from_shape_vec((height, width), map)
        .expect("inspection map dimensions checked")
        .into_pyarray(py);
    Ok((array, digest))
}

#[pyfunction]
fn cached_attention<'py>(
    py: Python<'py>,
    digest: &str,
) -> PyResult<Option<Bound<'py, PyArray2<f32>>>> {
    let map = py
        .detach(|| {
            let runtime = lock_runtime()?;
            let cached = runtime.as_ref().and_then(|runtime| {
                runtime
                    .models
                    .values()
                    .filter_map(|model| model.last.as_ref())
                    .find(|entry| entry.map.digest == digest)
            });
            Ok::<_, &str>(cached.map(|entry| entry.map.clone()))
        })
        .map_err(PyRuntimeError::new_err)?;
    Ok(map.as_ref().map(|entry| as_numpy(py, entry)))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(configure_attention, module)?)?;
    module.add_function(wrap_pyfunction!(inspection_preview, module)?)?;
    module.add_function(wrap_pyfunction!(inspection_generate, module)?)?;
    module.add_function(wrap_pyfunction!(cached_attention, module)?)
}
