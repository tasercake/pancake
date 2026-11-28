//! Ordered benchmark driver. Python supplies catalog decoding, TIFF codecs and plugins.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Instant;

use numpy::{IntoPyArray, PyReadonlyArray3, PyUntypedArrayMethods, ndarray::Array3};
use pyo3::exceptions::{PyFileNotFoundError, PyOSError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyCFunction, PyDict, PyList, PyTuple};
use sha2::{Digest, Sha256};

use crate::{c_slice, delivery_io, engine, registration::ImageView, score::cohort_score};

struct StdoutRedirect<'py> {
    sys: Bound<'py, PyModule>,
    previous: Bound<'py, PyAny>,
}

impl<'py> StdoutRedirect<'py> {
    fn new(py: Python<'py>) -> PyResult<Self> {
        let sys = py.import("sys")?;
        let previous = sys.getattr("stdout")?;
        sys.setattr("stdout", sys.getattr("stderr")?)?;
        Ok(Self { sys, previous })
    }
}

impl Drop for StdoutRedirect<'_> {
    fn drop(&mut self) {
        let _ = self.sys.setattr("stdout", &self.previous);
    }
}

fn text(error: &PyErr, py: Python<'_>) -> PyResult<String> {
    Ok(format!(
        "{}: {}",
        error.get_type(py).name()?,
        error.value(py).str()?
    ))
}

fn field<'py>(row: &Bound<'py, PyDict>, key: &str) -> PyResult<Bound<'py, PyAny>> {
    row.as_any().get_item(key)
}

fn mark_error(
    row: &Bound<'_, PyDict>,
    status: &str,
    error: String,
    unavailable: bool,
) -> PyResult<()> {
    row.set_item("status", status)?;
    row.set_item("error", error)?;
    if unavailable {
        row.set_item("reference_status", "error")?;
        row.set_item("scene_score", row.py().None())?;
    }
    Ok(())
}

fn describe_scene<'py>(
    catalog: &Bound<'py, PyAny>,
    scene: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyDict>> {
    let py = scene.py();
    let metadata = scene.getattr("metadata")?;
    let frames = scene.getattr("frame_ids")?;
    let references = scene.getattr("reference_ids")?;
    let inputs = PyList::empty(py);
    for id in frames.try_iter()?.chain(references.try_iter()?) {
        let frame = catalog.call_method1("frame", (id?,))?;
        inputs.append((
            frame.getattr("id")?,
            frame.getattr("source_sha256")?,
            frame.getattr("asset_sha256")?,
            frame.getattr("page")?,
        ))?;
    }
    let row = PyDict::new(py);
    row.set_item("scene_id", scene.getattr("id")?)?;
    for key in ["dataset_id", "leakage_group_id", "reference_type"] {
        row.set_item(key, metadata.get_item(key)?)?;
    }
    row.set_item("frame_count", frames.len()?)?;
    row.set_item("reference_ids", references)?;
    let fingerprint = PyTuple::new(py, [metadata.get_item("recipe_sha256")?, inputs.into_any()])?;
    row.set_item("input_sha256", hash_json(fingerprint.as_any())?)?;
    Ok(row)
}

fn mean_stack<'py>(
    frames: &Bound<'py, PyAny>,
    shape: (usize, usize, usize),
    count: usize,
) -> PyResult<Bound<'py, PyAny>> {
    let size = shape
        .0
        .checked_mul(shape.1)
        .and_then(|size| size.checked_mul(shape.2))
        .ok_or_else(|| PyValueError::new_err("Focal grid is too large"))?;
    let mut total = Vec::new();
    total
        .try_reserve_exact(size)
        .map_err(|error| pyo3::exceptions::PyMemoryError::new_err(error.to_string()))?;
    total.resize(size, 0.0_f64);
    for frame in frames.try_iter()? {
        let pixels: PyReadonlyArray3<'_, f32> = frame?.getattr("pixels")?.extract()?;
        if pixels.shape() != [shape.0, shape.1, shape.2] {
            return Err(PyValueError::new_err(
                "The mean stacker requires equal-sized focal grids",
            ));
        }
        let pixels = pixels.as_array();
        frames.py().detach(|| {
            for (sum, &pixel) in total.iter_mut().zip(pixels.iter()) {
                *sum += f64::from(pixel);
            }
        });
    }
    let result = total
        .into_iter()
        .map(|sample| (sample / count as f64) as f32)
        .collect();
    Ok(Array3::from_shape_vec(shape, result)
        .expect("validated focal grid")
        .into_pyarray(frames.py())
        .into_any())
}

struct Driver<'py> {
    bridge: Bound<'py, PyModule>,
    args: Bound<'py, PyAny>,
    catalog: Bound<'py, PyAny>,
    scorer: Scorer,
    algorithm: Bound<'py, PyAny>,
    candidate_dir: Option<PathBuf>,
    output: PathBuf,
    decode_workers: usize,
    use_mean: bool,
}

impl<'py> Driver<'py> {
    fn stack(
        &self,
        scene: &Bound<'py, PyAny>,
        row: &Bound<'py, PyDict>,
        shape: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = scene.py();
        let frames = self.bridge.getattr("_read_frames")?.call1((
            &self.catalog,
            scene,
            row,
            self.decode_workers,
        ))?;
        let run = || {
            let input_error = self.bridge.getattr("_InputError")?;
            let frame_ids = scene.getattr("frame_ids")?;
            let count = frame_ids.len()?;
            let result = if self.use_mean {
                mean_stack(&frames, shape.extract()?, count)
            } else {
                let input = self.bridge.getattr("StackInput")?.call1((
                    scene.getattr("id")?,
                    frame_ids,
                    shape,
                    &frames,
                ))?;
                let _redirect = StdoutRedirect::new(py)?;
                self.algorithm.call1((input,))
            };
            let result = match result {
                Ok(array) => array,
                Err(error) if error.matches(py, &input_error)? => return Err(error),
                Err(error) if error.is_instance_of::<pyo3::exceptions::PyException>(py) => {
                    mark_error(row, "algorithm_error", text(&error, py)?, false)?;
                    py.None().into_bound(py)
                }
                Err(error) => return Err(error),
            };
            // A callable may return early. Consume remaining frames for the same input checks.
            for frame in frames.try_iter()? {
                frame?;
            }
            if field(row, "frames_read")?.extract::<usize>()? != count {
                return Err(PyErr::from_value(input_error.call1((
                    "The focal frame stream ended before the scene was fully decoded",
                ))?));
            }
            Ok(result)
        };
        let result = run();
        frames.call_method0("close")?;
        result
    }

    fn reference(
        &self,
        scene: &Bound<'py, PyAny>,
        row: &Bound<'py, PyDict>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let read = || {
            let decoded = self
                .catalog
                .call_method1("decode", (scene.getattr("reference_ids")?.get_item(0)?,))?;
            let rgb = decoded.getattr("data")?;
            self.bridge.getattr("validate_rgb")?.call1((&rgb,))?;
            if !decoded.getattr("alpha")?.is_none() {
                return Err(PyValueError::new_err(
                    "The RGB benchmark reference cannot have native alpha",
                ));
            }
            Ok(rgb)
        };
        match read() {
            Ok(rgb) => {
                row.set_item("reference_status", "eligible")?;
                row.set_item("target_shape", rgb.getattr("shape")?)?;
                row.set_item("scene_score", 0.0)?;
                Ok(Some(rgb))
            }
            Err(error) if error.is_instance_of::<pyo3::exceptions::PyException>(scene.py()) => {
                mark_error(row, "reference_error", text(&error, scene.py())?, true)?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn candidate(
        &self,
        path: &Path,
        row: &Bound<'py, PyDict>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let read = || -> PyResult<_> {
            let result = match delivery_io::read_delivery_tiff(row.py(), path.to_owned())? {
                Some(pixels) => pixels,
                None => self
                    .bridge
                    .getattr("_read_candidate_fallback")?
                    .call1((path,))?,
            };
            row.set_item("candidate_sha256", row.py().detach(|| hash_file(path))?)?;
            Ok(result)
        };
        let py = row.py();
        match read() {
            Ok(result) => Ok(Some(result)),
            Err(error) if error.is_instance_of::<PyFileNotFoundError>(py) => {
                mark_error(row, "missing_output", text(&error, py)?, false)?;
                Ok(None)
            }
            Err(error) if error.is_instance_of::<pyo3::exceptions::PyException>(py) => {
                mark_error(
                    row,
                    "invalid_output",
                    error.value(py).str()?.extract()?,
                    false,
                )?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn prepare(
        &self,
        scene: &Bound<'py, PyAny>,
        row: &Bound<'py, PyDict>,
    ) -> PyResult<Option<Bound<'py, PyAny>>> {
        let py = row.py();
        row.set_item("frames_read", 0)?;
        row.set_item("scene_score", py.None())?;
        row.set_item("status", "ok")?;
        let Some(reference) = self.reference(scene, row)? else {
            return Ok(None);
        };
        let start = Instant::now();
        let result = if let Some(directory) = &self.candidate_dir {
            let id: String = scene.getattr("id")?.extract()?;
            self.candidate(&directory.join(format!("{id}.tif")), row)
        } else {
            self.stack(scene, row, &reference.getattr("shape")?)
                .map(Some)
        };
        row.set_item("seconds", start.elapsed().as_secs_f64())?;
        let Some(result) = (match result {
            Ok(result) => result,
            Err(error) if error.matches(py, self.bridge.getattr("_InputError")?)? => {
                mark_error(row, "input_error", error.value(py).str()?.extract()?, true)?;
                None
            }
            Err(error) => return Err(error),
        }) else {
            return Ok(None);
        };
        if !field(row, "status")?.eq("ok")? {
            return Ok(None);
        }
        if let Err(error) = self.bridge.getattr("validate_rgb")?.call1((&result,)) {
            if !error.is_instance_of::<PyValueError>(py) {
                return Err(error);
            }
            mark_error(
                row,
                "invalid_output",
                error.value(py).str()?.extract()?,
                false,
            )?;
            return Ok(None);
        }
        Ok(Some(
            self.bridge
                .getattr("_PreparedScene")?
                .call1((scene, row, result, reference))?,
        ))
    }

    fn finish(&self, scene: &Bound<'py, PyAny>, outcome: Outcome) -> PyResult<()> {
        let row = scene.getattr("row")?.cast_into::<PyDict>()?;
        let explanation = match outcome {
            Outcome::Retry => match self.scorer.attempt(scene, &row, false)? {
                Outcome::Complete(explanation) => explanation,
                Outcome::Retry => unreachable!("serial scores do not defer allocation failures"),
            },
            Outcome::Complete(explanation) => explanation,
        };
        self.bridge
            .getattr("_save_scene")?
            .call1((scene, &self.args, explanation))?;
        Ok(())
    }

    fn replay_fits(&self, batch: &[Bound<'py, PyAny>]) -> PyResult<bool> {
        self.bridge
            .getattr("_cuda_replay_fits")?
            .call1((PyList::new(self.args.py(), batch)?, &self.scorer.device))?
            .extract()
    }

    fn batch(&self, batch: &mut Vec<Bound<'py, PyAny>>) -> PyResult<()> {
        if batch.len() == 2 && self.replay_fits(batch)? {
            let py = self.args.py();
            let scorer = &self.scorer;
            let scenes: Vec<_> = batch.iter().cloned().map(Bound::unbind).collect();
            let results = py.detach(move || {
                std::thread::scope(|scope| {
                    let jobs: Vec<_> = scenes
                        .into_iter()
                        .map(|scene| {
                            scope.spawn(move || {
                                Python::attach(|py| {
                                    let scene = scene.bind(py);
                                    let row =
                                        scene.getattr("row")?.cast_into::<PyDict>()?.copy()?;
                                    let outcome = scorer.attempt(scene, &row, true)?;
                                    Ok::<_, PyErr>((row.unbind(), outcome))
                                })
                            })
                        })
                        .collect();
                    jobs.into_iter()
                        .map(|job| {
                            job.join().map_err(|_| {
                                PyRuntimeError::new_err("Benchmark replay worker panicked")
                            })?
                        })
                        .collect::<PyResult<Vec<_>>>()
                })
            })?;
            for (scene, (row, outcome)) in batch.drain(..).zip(results) {
                if matches!(outcome, Outcome::Complete(_)) {
                    scene.getattr("row")?.call_method1("update", (row,))?;
                }
                self.finish(&scene, outcome)?;
            }
        } else {
            for scene in batch.drain(..) {
                self.finish(&scene, Outcome::Retry)?;
            }
        }
        Ok(())
    }
}

enum Outcome {
    Complete(Py<PyAny>),
    Retry,
}

struct Scorer {
    override_delivery: Py<PyAny>,
    inspection: Py<PyAny>,
    prepare_attention: Py<PyAny>,
    allocation_failed: Py<PyAny>,
    device: String,
    details: Option<&'static str>,
    attention: bool,
}

impl Scorer {
    fn native(
        &self,
        scene: &Bound<'_, PyAny>,
        row: &Bound<'_, PyDict>,
        in_batch: bool,
    ) -> PyResult<Py<PyAny>> {
        let py = row.py();
        if self.attention {
            self.prepare_attention.bind(py).call0()?;
        }
        let candidate = engine::rgb_buffer(&scene.getattr("result")?)?;
        let target = engine::rgb_buffer(&scene.getattr("reference")?)?;
        let source = ImageView::new(
            c_slice(&candidate)?,
            candidate.shape()[0],
            candidate.shape()[1],
        );
        let reference = ImageView::new(c_slice(&target)?, target.shape()[0], target.shape()[1]);
        let options = engine::Options {
            details: self.details,
            device: &self.device,
            heatmap: None,
            attention: self.attention,
            retry_on_oom: !in_batch,
        };
        let delivery = py.detach(|| engine::measure(source, reference, options))?;
        row.set_item("scene_score", delivery.score())?;
        row.set_item(
            "alignment",
            engine::python_value(py, delivery.alignment, "")?,
        )?;
        if delivery
            .features
            .as_object()
            .is_some_and(|features| !features.is_empty())
        {
            row.set_item(
                "score_features",
                engine::python_value(py, delivery.features, "")?,
            )?;
            row.set_item(
                "artifact_diagnostics",
                engine::python_value(py, delivery.diagnostics, "")?,
            )?;
        }
        if let Some(digest) = delivery.digest {
            row.set_item("inspection_map_sha256", digest)?;
        }
        delivery.explanation.map_or_else(
            || Ok(py.None()),
            |value| engine::python_value(py, value, ""),
        )
    }

    fn overridden(
        &self,
        scene: &Bound<'_, PyAny>,
        row: &Bound<'_, PyDict>,
        in_batch: bool,
    ) -> PyResult<Py<PyAny>> {
        let py = row.py();
        let generator = if self.inspection.bind(py).is_none() {
            py.None().into_bound(py)
        } else {
            let inspection = self.inspection.clone_ref(py);
            let row = row.clone().unbind();
            PyCFunction::new_closure(py, None, None, move |args: &Bound<'_, PyTuple>, _| {
                let py = args.py();
                let generated = inspection
                    .bind(py)
                    .call_method1("generate", (args.get_item(0)?,))?;
                row.bind(py)
                    .set_item("inspection_map_sha256", generated.get_item(1)?)?;
                Ok::<_, PyErr>(generated.get_item(0)?.unbind())
            })?
            .into_any()
        };
        let kwargs = PyDict::new(py);
        kwargs.set_item("details", self.details)?;
        kwargs.set_item("device", &self.device)?;
        kwargs.set_item("generate", generator)?;
        kwargs.set_item("retry_on_oom", !in_batch)?;
        let measured = self.override_delivery.bind(py).call(
            (scene.getattr("result")?, scene.getattr("reference")?),
            Some(&kwargs),
        )?;
        row.set_item("scene_score", measured.getattr("score")?)?;
        row.set_item("alignment", measured.getattr("alignment")?)?;
        let features = measured.getattr("features")?;
        if features.is_truthy()? {
            row.set_item("score_features", features)?;
            row.set_item("artifact_diagnostics", measured.getattr("diagnostics")?)?;
        }
        Ok(measured.getattr("explanation")?.unbind())
    }

    fn attempt(
        &self,
        scene: &Bound<'_, PyAny>,
        row: &Bound<'_, PyDict>,
        in_batch: bool,
    ) -> PyResult<Outcome> {
        let py = row.py();
        let start = Instant::now();
        let result = if self.override_delivery.bind(py).is_none() {
            self.native(scene, row, in_batch)
        } else {
            self.overridden(scene, row, in_batch)
        };
        row.set_item("scoring_seconds", start.elapsed().as_secs_f64())?;
        match result {
            Ok(explanation) => Ok(Outcome::Complete(explanation)),
            Err(error) if error.is_instance_of::<pyo3::exceptions::PyException>(py) => {
                if in_batch
                    && self
                        .allocation_failed
                        .bind(py)
                        .call1((error.value(py),))?
                        .extract()?
                {
                    return Ok(Outcome::Retry);
                }
                mark_error(row, "scoring_error", text(&error, py)?, true)?;
                Ok(Outcome::Complete(py.None()))
            }
            Err(error) => Err(error),
        }
    }
}

#[pyfunction]
fn benchmark_cuda_memory(_py: Python<'_>, _device: &str) -> Option<usize> {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    return _py.detach(|| {
        if !matches!(_device, "cuda" | "gpu" | "auto")
            || engine::select_backend(_device).ok()? != "cuda"
        {
            return None;
        }
        let cuda = cudarc::driver::CudaContext::new(0).ok()?;
        cuda.mem_get_info().ok().map(|(free, _)| free)
    });
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    None
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            return Ok(format!("{:x}", hash.finalize()));
        }
        hash.update(&buffer[..count]);
    }
}

fn hash_json(value: &Bound<'_, PyAny>) -> PyResult<String> {
    let py = value.py();
    let options = PyDict::new(py);
    options.set_item("sort_keys", true)?;
    options.set_item("separators", (",", ":"))?;
    let data: String = py
        .import("json")?
        .getattr("dumps")?
        .call((value,), Some(&options))?
        .extract()?;
    Ok(format!("{:x}", Sha256::digest(data)))
}

fn write_json(
    value: &Bound<'_, PyAny>,
    path: &Path,
    temporary: &Path,
    pretty: bool,
) -> PyResult<()> {
    let py = value.py();
    let options = PyDict::new(py);
    options.set_item("allow_nan", false)?;
    if pretty {
        options.set_item("indent", 2)?;
        options.set_item("sort_keys", true)?;
    } else {
        options.set_item("separators", (",", ":"))?;
    }
    let mut data: String = py
        .import("json")?
        .getattr("dumps")?
        .call((value,), Some(&options))?
        .extract()?;
    data.push('\n');
    py.detach(|| {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let result = fs::write(temporary, data).and_then(|()| fs::rename(temporary, path));
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        Ok::<_, std::io::Error>(result?)
    })?;
    Ok(())
}

#[pyfunction]
fn save_benchmark_scene(
    scene: &Bound<'_, PyAny>,
    args: &Bound<'_, PyAny>,
    explanation: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let py = scene.py();
    let row = scene.getattr("row")?.cast_into::<PyDict>()?;
    if !field(&row, "status")?.eq("ok")? {
        return Ok(());
    }
    let metadata = scene.getattr("scene")?;
    let id: String = metadata.getattr("id")?.extract()?;
    let output: PathBuf = args.getattr("output")?.extract()?;
    if args.getattr("save_images")?.is_truthy()? {
        let relative = format!("images/{id}.tif");
        let path = output.join(&relative);
        let write = || {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            py.import("bokehforge.bench")?
                .getattr("encode_tiff")?
                .call1((&path, scene.getattr("result")?))?;
            Ok::<_, PyErr>(())
        };
        match write() {
            Ok(()) => row.set_item("image", relative)?,
            Err(error) if error.is_instance_of::<pyo3::exceptions::PyException>(py) => {
                if let Err(err) = fs::remove_file(&path)
                    && err.kind() != std::io::ErrorKind::NotFound
                {
                    return Err(err.into());
                }
                mark_error(&row, "image_error", text(&error, py)?, false)?;
            }
            Err(error) => return Err(error),
        }
    }
    if !explanation.is_none() {
        let relative = format!("score-explanations/{id}.json");
        let path = output.join(&relative);
        let payload = explanation.cast::<PyDict>()?.copy()?;
        payload.set_item("scene_id", id)?;
        payload.set_item(
            "reference_id",
            metadata.getattr("reference_ids")?.get_item(0)?,
        )?;
        if let Some(digest) = row.get_item("inspection_map_sha256")? {
            payload.set_item("inspection_map_sha256", digest)?;
        }
        match write_json(payload.as_any(), &path, &path.with_extension("tmp"), false) {
            Ok(()) => row.set_item("score_explanation_path", relative)?,
            Err(error)
                if error.is_instance_of::<PyOSError>(py)
                    || error.is_instance_of::<PyValueError>(py) =>
            {
                row.set_item("score_explanation_error", text(&error, py)?)?;
                if field(&row, "status")?.eq("ok")? {
                    row.set_item("status", "score_explanation_error")?;
                }
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

#[pyfunction]
#[pyo3(signature = (rows, *, explanation=None))]
fn benchmark_score(
    rows: &Bound<'_, PyAny>,
    explanation: Option<&Bound<'_, PyDict>>,
) -> PyResult<(Option<f64>, usize)> {
    let py = rows.py();
    let mut entries = Vec::new();
    let mut unavailable = false;
    for row in rows.try_iter()? {
        let row = row?;
        unavailable |= row.get_item("reference_status")?.eq("error")?;
        if let Some(score) = row.get_item("scene_score")?.extract::<Option<f64>>()? {
            entries.push((
                row.get_item("dataset_id")?.extract()?,
                row.get_item("leakage_group_id")?.extract()?,
                score,
            ));
        }
    }
    let (score, groups, datasets) = cohort_score(entries, unavailable, explanation.is_some());
    if !unavailable && score.is_none() {
        return Err(PyErr::from_value(
            py.import("statistics")?
                .getattr("StatisticsError")?
                .call1(("fmean requires at least one data point",))?,
        ));
    }
    if let Some(explanation) = explanation {
        let data = PyDict::new(py);
        for (dataset, mean, weight, groups) in datasets {
            let item = PyDict::new(py);
            item.set_item("score", mean)?;
            item.set_item("weight", weight)?;
            let group_details = PyDict::new(py);
            for (group, mean, weight, count) in groups {
                let detail = PyDict::new(py);
                detail.set_item("score", mean)?;
                detail.set_item("weight", weight)?;
                detail.set_item("scene_count", count)?;
                group_details.set_item(group, detail)?;
            }
            item.set_item("groups", group_details)?;
            data.set_item(dataset, item)?;
        }
        explanation.set_item("score", score)?;
        explanation.set_item(
            "status",
            if unavailable {
                "evaluation_error"
            } else {
                "ok"
            },
        )?;
        explanation.set_item("datasets", data)?;
    }
    Ok((score, groups))
}

fn print_progress(py: Python<'_>, message: String) -> PyResult<()> {
    let options = PyDict::new(py);
    options.set_item("file", py.import("sys")?.getattr("stderr")?)?;
    options.set_item("flush", true)?;
    py.import("builtins")?
        .getattr("print")?
        .call((message,), Some(&options))?;
    Ok(())
}

fn report_scoring<'py>(
    bridge: &Bound<'py, PyModule>,
    attention: bool,
) -> PyResult<Bound<'py, PyDict>> {
    let py = bridge.py();
    let scoring = PyDict::new(py);
    scoring.set_item("family_weights", bridge.getattr("FAMILY_WEIGHTS")?)?;
    scoring.set_item("damage_scale", bridge.getattr("SCORE_DAMAGE_SCALE")?)?;
    let versions = PyDict::new(py);
    let version = bridge.getattr("version")?;
    for name in ["bokehforge", "numpy", "scipy", "tifffile"] {
        versions.set_item(name, version.call1((name,))?)?;
    }
    scoring.set_item("runtime", versions)?;
    let version: String = py.import("sys")?.getattr("version")?.extract()?;
    scoring.set_item("python", version.split_whitespace().next())?;
    scoring.set_item(
        "aggregation",
        "equal datasets; equal leakage groups within each dataset",
    )?;
    let alignment = PyDict::new(py);
    alignment.set_item("schema_version", bridge.getattr("ALIGNMENT_VERSION")?)?;
    alignment.set_item(
        "max_rotation_degrees",
        bridge.getattr("MAX_ROTATION_DEGREES")?,
    )?;
    alignment.set_item(
        "coverage_penalty",
        "minimum of relative intersection area and contributing delivery samples",
    )?;
    scoring.set_item("alignment", alignment)?;
    if attention {
        let inspection = PyDict::new(py);
        inspection.set_item("map_version", bridge.getattr("INSPECTION_MAP_VERSION")?)?;
        inspection.set_item("max_mix", bridge.getattr("INSPECTION_MIX")?)?;
        inspection.set_item(
            "max_multiplier",
            bridge.getattr("INSPECTION_MAX_MULTIPLIER")?,
        )?;
        scoring.set_item("inspection", inspection)?;
    }
    Ok(scoring)
}

#[pyfunction]
fn benchmark(args: Bound<'_, PyAny>, bridge: Bound<'_, PyModule>) -> PyResult<i32> {
    let py = args.py();
    let output: PathBuf = args.getattr("output")?.extract()?;
    if output.exists() {
        return Err(PyValueError::new_err(format!(
            "Output directory already exists: {}",
            output.display()
        )));
    }
    let root = args.getattr("root")?;
    let config = args.getattr("config")?;
    let root = if root.is_none() {
        bridge.getattr("data_root")?.call1((&config,))?
    } else {
        root.call_method0("expanduser")?.call_method0("resolve")?
    };
    let start = Instant::now();
    let options = PyDict::new(py);
    options.set_item("datasets", args.getattr("dataset")?)?;
    options.set_item("config", config)?;
    options.set_item("lock", args.getattr("lock")?)?;
    let catalog = bridge
        .getattr("SceneCatalog")?
        .call((&root,), Some(&options))?;
    let available: Vec<String> = catalog.getattr("scene_ids")?.extract()?;
    let requested: Option<Vec<String>> = args.getattr("scene")?.extract()?;
    let selected: BTreeSet<_> = requested
        .unwrap_or_else(|| available.clone())
        .into_iter()
        .collect();
    let known: BTreeSet<_> = available.iter().cloned().collect();
    let unknown: Vec<_> = selected.difference(&known).collect();
    if !unknown.is_empty() {
        return Err(PyValueError::new_err(format!(
            "Unknown scene IDs: {}",
            PyList::new(py, unknown)?.repr()?
        )));
    }
    let scenes: Vec<_> = available
        .into_iter()
        .filter(|id| selected.contains(id))
        .map(|id| catalog.call_method1("scene", (id,)))
        .collect::<PyResult<_>>()?;
    if scenes.is_empty() {
        return Err(PyValueError::new_err(
            "No benchmark scenes in the selected datasets",
        ));
    }
    let require_reference = bridge.getattr("_require_benchmark_reference")?;
    for scene in &scenes {
        require_reference.call1((scene,))?;
    }
    let candidate = args.getattr("candidate_dir")?;
    let candidate_dir: Option<PathBuf> = candidate.extract()?;
    let spec: Option<String> = args.getattr("algorithm")?.extract()?;
    let use_mean = spec.as_deref() == Some("mean");
    let (algorithm, implementation) = if let Some(directory) = &candidate_dir {
        if !directory.is_dir() {
            return Err(PyValueError::new_err(format!(
                "Candidate directory does not exist: {}",
                directory.display()
            )));
        }
        let implementation = PyDict::new(py);
        implementation.set_item("candidate_dir", candidate.call_method0("resolve")?.str()?)?;
        (py.None().into_bound(py), implementation.into_any())
    } else {
        let loaded = bridge.getattr("_load_algorithm")?.call1((spec,))?;
        (loaded.get_item(0)?, loaded.get_item(1)?)
    };
    let rows = PyList::new(
        py,
        scenes
            .iter()
            .map(|scene| describe_scene(&catalog, scene))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    let cohort_hash = hash_json(rows.as_any())?;
    let attention = !args.getattr("no_attention")?.is_truthy()?;
    let overrides = bridge.getattr("_benchmark_override")?.call1((attention,))?;
    let explain: bool = args.getattr("explain_score")?.extract()?;
    let scorer = Scorer {
        override_delivery: overrides.get_item(0)?.unbind(),
        inspection: overrides.get_item(1)?.unbind(),
        prepare_attention: bridge.getattr("prepare_attention")?.unbind(),
        allocation_failed: bridge.getattr("_allocation_failed")?.unbind(),
        device: args.getattr("device")?.extract()?,
        details: explain.then_some("full"),
        attention,
    };
    let decode_workers = args.getattr("decode_workers")?.extract()?;
    let root_name = root.call_method0("resolve")?.str()?;
    let progress: bool = args.getattr("progress")?.extract()?;
    let replay = candidate_dir.is_some() && attention && !explain && scenes.len() > 1;
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(&output)?;
    if progress {
        print_progress(
            py,
            format!("Benchmarking {} scenes from {root_name}", scenes.len()),
        )?;
    }
    let driver = Driver {
        args,
        bridge,
        catalog,
        scorer,
        algorithm,
        candidate_dir,
        output,
        decode_workers,
        use_mean,
    };
    let mut previous_dataset = String::new();
    let mut batch = Vec::with_capacity(2);
    {
        let _redirect = replay.then(|| StdoutRedirect::new(py)).transpose()?;
        for (index, (scene, row)) in scenes.iter().zip(rows.iter()).enumerate() {
            let index = index + 1;
            let row = row.cast_into::<PyDict>()?;
            if progress {
                let dataset: String = field(&row, "dataset_id")?.extract()?;
                if dataset != previous_dataset || index % 25 == 0 {
                    print_progress(
                        py,
                        format!(
                            "{index}/{} {} ({} frames)",
                            scenes.len(),
                            scene.getattr("id")?,
                            field(&row, "frame_count")?
                        ),
                    )?;
                }
                previous_dataset = dataset;
            }
            let Some(prepared) = driver.prepare(scene, &row)? else {
                continue;
            };
            if replay {
                batch.push(prepared);
                if batch.len() == 2 || !driver.replay_fits(&batch)? {
                    driver.batch(&mut batch)?;
                }
            } else {
                let outcome = {
                    let _redirect = StdoutRedirect::new(py)?;
                    driver.scorer.attempt(&prepared, &row, false)?
                };
                driver.finish(&prepared, outcome)?;
            }
        }
        if !batch.is_empty() {
            driver.batch(&mut batch)?;
        }
    }

    let aggregation = explain.then(|| PyDict::new(py));
    let (score, scored_groups) = benchmark_score(rows.as_any(), aggregation.as_ref())?;
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut references = BTreeMap::<String, usize>::new();
    for row in rows.iter() {
        *statuses
            .entry(row.get_item("status")?.extract()?)
            .or_default() += 1;
        *references
            .entry(row.get_item("reference_status")?.extract()?)
            .or_default() += 1;
    }
    let failed = statuses.keys().any(|status| status != "ok");
    let report = PyDict::new(py);
    report.set_item("schema_version", 4)?;
    report.set_item("score", score)?;
    report.set_item(
        "score_version",
        driver.bridge.getattr(if attention {
            "INSPECTION_SCORE_VERSION"
        } else {
            "SCORE_VERSION"
        })?,
    )?;
    report.set_item("scoring", report_scoring(&driver.bridge, attention)?)?;
    report.set_item("scored_groups", scored_groups)?;
    report.set_item("cohort_sha256", cohort_hash)?;
    report.set_item("algorithm", implementation)?;
    report.set_item("data_root", root_name)?;
    report.set_item("decode_workers", decode_workers)?;
    report.set_item("seconds", start.elapsed().as_secs_f64())?;
    report.set_item("scene_status_counts", statuses)?;
    report.set_item("reference_status_counts", references)?;
    report.set_item("quarantine", driver.catalog.getattr("quarantine")?)?;
    report.set_item("scenes", rows)?;
    if let Some(aggregation) = aggregation {
        report.set_item(
            "score_explanation_version",
            driver.bridge.getattr("SCORE_EXPLANATION_VERSION")?,
        )?;
        report.set_item("score_aggregation", aggregation)?;
    }
    write_json(
        report.as_any(),
        &driver.output.join("report.json"),
        &driver.output.join("report.json.tmp"),
        true,
    )?;
    let line = score.map_or_else(|| "unscored".to_owned(), |score| format!("{score:.6}"));
    py.import("builtins")?.getattr("print")?.call1((line,))?;
    Ok(i32::from(failed))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(benchmark, module)?)?;
    module.add_function(wrap_pyfunction!(benchmark_score, module)?)?;
    module.add_function(wrap_pyfunction!(benchmark_cuda_memory, module)?)?;
    module.add_function(wrap_pyfunction!(save_benchmark_scene, module)?)?;
    Ok(())
}
