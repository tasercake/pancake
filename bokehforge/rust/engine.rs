use std::fmt;
use std::io::Write;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use flate2::{Compression, write::ZlibEncoder};
use numpy::{
    PyArray, PyArrayDescrMethods, PyArrayMethods, PyReadonlyArray, PyUntypedArray,
    PyUntypedArrayMethods, dtype,
    ndarray::{Dimension, Ix2, Ix3},
};
use pyo3::{
    create_exception,
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::{PyDict, PyList, PyTuple},
};
use rayon::prelude::*;
use serde_json::{Value, json};

use crate::{
    accelerator, attention, c_slice, parallel, quality,
    registration::{self, ImageView},
    score::{compensated_sum, inspection_gain, placement_score},
};

const DETAILS: [&str; 4] = ["summary", "tiles", "attention", "full"];
const DEVICES: [&str; 5] = ["auto", "cpu", "gpu", "cuda", "mps"];
const DAMAGE_SCALE: u8 = 5;
const INSPECTION_MIX: f64 = 0.5;
const INSPECTION_MAX_MULTIPLIER: f64 = 2.0;

create_exception!(bokehforge, DeviceOutOfMemory, PyRuntimeError);

#[derive(Debug)]
pub(crate) enum Error {
    Invalid(String),
    Runtime(String),
    OutOfMemory(String),
}

impl Error {
    fn operation(message: String) -> Self {
        let lower = message.to_ascii_lowercase();
        if [
            "out of memory",
            "out of device memory",
            "out-of-memory",
            "cuda_error_out_of_memory",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            Self::OutOfMemory(message)
        } else {
            Self::Runtime(message)
        }
    }

    pub(crate) fn is_out_of_memory(&self) -> bool {
        matches!(self, Self::OutOfMemory(_))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) | Self::Runtime(message) | Self::OutOfMemory(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<Error> for PyErr {
    fn from(error: Error) -> Self {
        match error {
            Error::Invalid(message) => PyValueError::new_err(message),
            Error::Runtime(message) => PyRuntimeError::new_err(message),
            Error::OutOfMemory(message) => DeviceOutOfMemory::new_err(message),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Options<'a> {
    pub details: Option<&'a str>,
    pub device: &'a str,
    pub heatmap: Option<&'a [f32]>,
    pub attention: bool,
    pub retry_on_oom: bool,
}

pub(crate) struct Delivery {
    pub features: Value,
    pub diagnostics: Value,
    pub explanation: Option<Value>,
    pub alignment: Value,
    pub digest: Option<String>,
    score: f64,
}

impl Delivery {
    pub(crate) fn score(&self) -> f64 {
        self.score
    }
}

fn check_device(device: &str) -> Result<(), Error> {
    if DEVICES.contains(&device) {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "Expected a scoring device in ('auto', 'cpu', 'gpu', 'cuda', 'mps'), got {device:?}"
        )))
    }
}

pub(crate) fn select_backend(device: &str) -> Result<&'static str, Error> {
    check_device(device)?;
    let probe =
        |accelerator: &'static str| accelerator::availability(accelerator).map(|()| accelerator);
    let either =
        || probe("cuda").or_else(|cuda| probe("mps").map_err(|mps| format!("{cuda}. {mps}")));
    let (result, requirements) = match device {
        "cpu" => return Ok("cpu"),
        "auto" => return Ok(either().unwrap_or("cpu")),
        "gpu" => (either(), "CUDA or Apple Metal (MPS)"),
        "cuda" => (probe("cuda"), "CUDA with a working NVIDIA driver and GPU"),
        "mps" => (probe("mps"), "Apple Metal (MPS) on Apple Silicon"),
        _ => unreachable!("validated device"),
    };
    result.map_err(|error| {
        Error::Runtime(format!(
            "--device {device} requires {requirements}. {error}"
        ))
    })
}

pub(crate) fn valid_rgb(image: ImageView<'_>) -> Result<(), Error> {
    if image.h == 0
        || image.w == 0
        || image.h.checked_mul(image.w).and_then(|n| n.checked_mul(3)) != Some(image.data.len())
    {
        return Err(Error::Invalid(
            "The benchmark requires a nonempty RGB image with three channels.".to_owned(),
        ));
    }
    let has_invalid = |samples: &[f32]| samples.iter().any(|value| !value.is_finite());
    let invalid = if image.data.len() >= 3_000_000 {
        parallel(|| image.data.par_chunks(64 * 1024).any(has_invalid))
    } else {
        has_invalid(image.data)
    };
    if invalid {
        return Err(Error::Invalid(
            "The image contains non-finite pixel values (NaN or infinity).".to_owned(),
        ));
    }
    Ok(())
}

fn heatmap_error([height, width]: [usize; 2]) -> Error {
    Error::Invalid(format!(
        "Expected a finite floating-point inspection map in [0, 1] with shape ({height}, {width})"
    ))
}

fn damage_with_retry<T>(
    backend: &mut &'static str,
    retry_on_oom: bool,
    mut compare: impl FnMut(&str) -> Result<T, Error>,
) -> Result<T, Error> {
    match compare(backend) {
        Err(error) if *backend != "cpu" && retry_on_oom && error.is_out_of_memory() => {
            *backend = "cpu";
            compare(backend)
        }
        outcome => outcome,
    }
}

struct InspectionWeights<'a> {
    heatmap: &'a [f32],
    mean: f64,
    mix: f64,
    weights: Vec<f32>,
    range: [f32; 2],
}

impl<'a> InspectionWeights<'a> {
    fn new(heatmap: &'a [f32], [height, width]: [usize; 2]) -> Result<Self, Error> {
        if heatmap.len() != height * width {
            return Err(heatmap_error([height, width]));
        }
        let statistics = |samples: &[f32]| {
            let mut low = 1.0_f32;
            let mut peak = 0.0_f32;
            let mut finite = true;
            let sum = compensated_sum(samples.iter().map(|&sample| {
                low = low.min(sample);
                peak = peak.max(sample);
                finite &= sample.is_finite();
                f64::from(sample)
            }));
            (sum, low, peak, finite)
        };
        let (sum, low, peak, finite) = if heatmap.len() >= 1_000_000 {
            let chunks: Vec<_> =
                parallel(|| heatmap.par_chunks(64 * 1024).map(statistics).collect());
            let sum = compensated_sum(chunks.iter().map(|&(sum, _, _, _)| sum));
            let (low, peak, finite) = chunks.into_iter().fold(
                (1.0_f32, 0.0_f32, true),
                |(low, peak, finite), (_, chunk_low, chunk_peak, chunk_finite)| {
                    (
                        low.min(chunk_low),
                        peak.max(chunk_peak),
                        finite && chunk_finite,
                    )
                },
            );
            (sum, low, peak, finite)
        } else {
            statistics(heatmap)
        };
        if !finite || low < 0.0 || peak > 1.0 {
            return Err(heatmap_error([height, width]));
        }
        let mean = sum / heatmap.len() as f64;
        let mut result = Self {
            heatmap,
            mean,
            mix: 0.0,
            weights: Vec::new(),
            range: [1.0, 1.0],
        };
        if let Some((gain, mix)) = inspection_gain(
            mean,
            f64::from(peak),
            INSPECTION_MIX,
            INSPECTION_MAX_MULTIPLIER,
        ) {
            let weight = |&sample: &f32| (1.0 + (f64::from(sample) - mean) * gain) as f32;
            result.mix = mix;
            result.weights = if heatmap.len() >= 1_000_000 {
                parallel(|| heatmap.par_iter().map(weight).collect())
            } else {
                heatmap.iter().map(weight).collect()
            };
            result.range = [weight(&low), weight(&peak)];
        }
        Ok(result)
    }

    fn describe(&self, [height, width]: [usize; 2], details: &str) -> Result<Value, Error> {
        let mut value = json!({
            "size_wh": [width, height],
            "range": [0, 1],
            "mean": self.mean,
            "mix": self.mix,
            "damage_multiplier_range": self.range,
        });
        if matches!(details, "attention" | "full") {
            let encode = || -> std::io::Result<Vec<u8>> {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                let mut buffer = [0_u8; 16 * 1024];
                for chunk in self.heatmap.chunks(buffer.len() / size_of::<f32>()) {
                    for (bytes, sample) in buffer
                        .as_chunks_mut::<{ size_of::<f32>() }>()
                        .0
                        .iter_mut()
                        .zip(chunk)
                    {
                        bytes.copy_from_slice(&sample.to_le_bytes());
                    }
                    encoder.write_all(&buffer[..std::mem::size_of_val(chunk)])?;
                }
                encoder.finish()
            };
            let compressed = encode().map_err(|exc| Error::Runtime(exc.to_string()))?;
            value["encoding"] = json!("base64+zlib+float32-le");
            value["data"] = Value::String(STANDARD.encode(compressed));
        }
        Ok(value)
    }
}

pub(crate) fn measure(
    candidate: ImageView<'_>,
    target: ImageView<'_>,
    options: Options<'_>,
) -> Result<Delivery, Error> {
    if let Some(details) = options.details.filter(|level| !DETAILS.contains(level)) {
        return Err(Error::Invalid(format!(
            "Expected score details to be None or one of ('summary', 'tiles', 'attention', 'full'), got {details:?}"
        )));
    }
    valid_rgb(target)?;
    valid_rgb(candidate)?;
    check_device(options.device)?;

    let target_shape = [target.h, target.w];
    let generate_attention = options.attention && options.heatmap.is_none();
    let mut backend = if options.device != "auto" {
        Some(select_backend(options.device)?)
    } else {
        None
    };
    let supplied_inspection = options
        .heatmap
        .map(|samples| InspectionWeights::new(samples, target_shape))
        .transpose()?;
    let align = || registration::align(candidate, target);
    let inference_device = if options.device == "gpu" {
        backend.expect("explicit accelerator selected")
    } else {
        options.device
    };
    let generate = || {
        attention::generate(target.data, target.h, target.w, inference_device)
            .map_err(Error::operation)
    };
    let ((placements, mut alignment), generated) = if generate_attention {
        if target.h * target.w >= 1_000_000 {
            let (aligned, generated) = parallel(|| rayon::join(align, generate));
            (aligned, Some(generated?))
        } else {
            let generated = generate()?;
            (align(), Some(generated))
        }
    } else {
        (align(), None)
    };
    let (generated_map, digest) = generated.unzip();
    let inspection = match generated_map.as_deref() {
        Some(samples) => Some(InspectionWeights::new(samples, target_shape)?),
        None => supplied_inspection,
    };
    let weights = inspection
        .as_ref()
        .map(|map| (map.weights.as_slice(), map.mix));

    let mut rows = Vec::with_capacity(placements.len());
    let mut explanations = Vec::new();
    let mut leading = None;
    let mut contributions = Vec::with_capacity(placements.len());
    for placement in placements {
        let (geometry, resolution, polygon) = registration::overlap(&placement, target_shape);
        let mut comparison = if geometry <= 0.0 {
            None
        } else if matches!(
            placement.evidence["source"].as_str(),
            Some("exact_native_delivery_in_reference" | "exact_reference_in_native_delivery")
        ) {
            Some(
                quality::measure(
                    &[],
                    target.data,
                    target.h,
                    target.w,
                    None,
                    Some(registration::native_overlap(&placement, target_shape)),
                    weights,
                    options.details,
                    "cpu",
                )
                .map_err(Error::operation)?,
            )
        } else {
            let (projected, support) = registration::project(candidate, target_shape, &placement);
            if support
                .as_ref()
                .is_some_and(|mask| !mask.iter().any(|&value| value != 0))
            {
                None
            } else {
                if backend.is_none() {
                    backend = Some(select_backend(options.device)?);
                }
                let compare = |device: &str| {
                    quality::measure(
                        &projected,
                        target.data,
                        target.h,
                        target.w,
                        support.as_deref(),
                        None,
                        weights,
                        options.details,
                        device,
                    )
                    .map_err(Error::operation)
                };
                Some(damage_with_retry(
                    backend.as_mut().expect("damage backend selected"),
                    options.device == "auto" && options.retry_on_oom,
                    compare,
                )?)
            }
        };

        let quality = comparison
            .as_ref()
            .map_or(0.0, |comparison| comparison.score);
        let (coverage, contribution) =
            placement_score(geometry, resolution, placement.weight, quality);
        let [top, left, bottom, right] = placement.bounds;
        let mut row = placement.pose.describe();
        let fields = json!({
            "evidence": placement.evidence,
            "candidate_content_box_xyxy": [left, top, right, bottom],
            "padding": placement.padding,
            "reference_overlap_polygon_xy": polygon,
            "geometric_coverage": geometry,
            "resolution_cap": resolution,
            "coverage": coverage,
            "weight": placement.weight,
            "overlap_score": quality,
            "score_contribution": contribution,
            "comparison_status": if comparison.is_some() { "measured" } else { "no_measured_reference_centers" }
        });
        if let (Some(row), Value::Object(fields)) = (row.as_object_mut(), fields) {
            row.extend(fields);
        }
        if options.details.is_some() {
            let mut explanation = row.clone();
            explanation["comparison"] = comparison
                .as_mut()
                .and_then(|comparison| comparison.explanation.take())
                .unwrap_or(Value::Null);
            explanations.push(explanation);
        }
        if leading.is_none()
            && let Some(comparison) = comparison
        {
            leading = Some((comparison.features, comparison.diagnostics));
        }
        contributions.push(contribution);
        rows.push(row);
    }
    let score = compensated_sum(contributions);
    let json_score = if rows.is_empty() {
        json!(0)
    } else {
        json!(score)
    };
    alignment["placements"] = Value::Array(rows);
    let mut explanation = options.details.map(|details| {
        let mut value = json!({
            "schema_version": 2,
            "profile": if inspection.is_some() {
                "reference-fidelity-v3-inspection-v2"
            } else {
                "reference-fidelity-v3"
            },
            "detail_level": details,
            "score": json_score,
            "damage_scale": DAMAGE_SCALE,
            "reference_size_wh": alignment["reference_size_wh"],
            "candidate_size_wh": alignment["candidate_size_wh"],
            "confidence": alignment["confidence"],
            "alignment_status": alignment["status"],
            "reason": alignment["reason"],
        });
        value["placements"] = Value::Array(explanations);
        value
    });
    if let (Some(inspection), Some(explanation), Some(details)) =
        (&inspection, &mut explanation, options.details)
    {
        explanation["attention"] = inspection.describe(target_shape, details)?;
    }
    let (features, diagnostics) = leading.unwrap_or_else(|| (json!({}), json!({})));
    Ok(Delivery {
        features,
        diagnostics,
        explanation,
        alignment,
        digest,
        score,
    })
}

fn normalized_f32<'py, D: Dimension>(
    object: &Bound<'py, PyAny>,
) -> PyResult<PyReadonlyArray<'py, f32, D>> {
    if let Ok(array) = object.cast::<PyArray<f32, D>>() {
        let readonly = array.try_readonly()?;
        if c_slice(&readonly).is_ok() {
            return Ok(readonly);
        }
    }
    let py = object.py();
    py.import("numpy")?
        .call_method1("require", (object, dtype::<f32>(py), ("C", "A", "E")))?
        .extract()
}

pub(crate) fn rgb_buffer<'py>(
    object: &Bound<'py, PyAny>,
) -> PyResult<PyReadonlyArray<'py, f32, Ix3>> {
    let image = object
        .cast::<PyUntypedArray>()
        .map_err(|_| PyValueError::new_err("The benchmark requires an RGB image array."))?;
    if !matches!(image.shape(), &[h, w, 3] if h != 0 && w != 0) {
        let shape = PyTuple::new(object.py(), image.shape().iter().copied())?;
        return Err(PyValueError::new_err(format!(
            "The benchmark requires a nonempty RGB image with three channels. The image has array shape {shape}."
        )));
    }
    let kind = image.dtype();
    if kind.kind() != b'f' || kind.itemsize() != size_of::<f32>() {
        return Err(PyValueError::new_err(format!(
            "The image uses {kind} samples. The benchmark requires 32-bit floating-point linear-sRGB samples."
        )));
    }
    normalized_f32(object)
}

fn heatmap_buffer<'py>(
    object: &Bound<'py, PyAny>,
    shape: [usize; 2],
) -> PyResult<PyReadonlyArray<'py, f32, Ix2>> {
    let array = object
        .cast::<PyUntypedArray>()
        .map_err(|_| heatmap_error(shape))?;
    if array.shape() != shape || array.dtype().kind() != b'f' {
        return Err(heatmap_error(shape).into());
    }
    // Check non-f32 values before narrowing so a sample just outside the range
    // cannot be rounded onto an endpoint by the conversion.
    if array.dtype().itemsize() != size_of::<f32>()
        && (!array.call_method0("min")?.ge(0)? || !array.call_method0("max")?.le(1)?)
    {
        return Err(heatmap_error(shape).into());
    }
    normalized_f32(object)
}

pub(crate) fn python_value(py: Python<'_>, value: Value, key: &str) -> PyResult<Py<PyAny>> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(value) => Ok(value.into_pyobject(py)?.to_owned().into_any().unbind()),
        Value::String(value) => Ok(value.into_pyobject(py)?.into_any().unbind()),
        Value::Number(value) => {
            if let Some(value) = value.as_u64() {
                Ok(value.into_pyobject(py)?.into_any().unbind())
            } else if let Some(value) = value.as_i64() {
                Ok(value.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(value
                    .as_f64()
                    .expect("JSON number")
                    .into_pyobject(py)?
                    .into_any()
                    .unbind())
            }
        }
        Value::Array(values) => {
            let values: Vec<_> = values
                .into_iter()
                .map(|value| python_value(py, value, ""))
                .collect::<PyResult<_>>()?;
            if matches!(key, "native_box_xyxy" | "view_size_wh") {
                Ok(PyTuple::new(py, values)?.into_any().unbind())
            } else {
                Ok(PyList::new(py, values)?.into_any().unbind())
            }
        }
        Value::Object(values) => {
            let result = PyDict::new(py);
            for (key, value) in values {
                let value = python_value(py, value, &key)?;
                result.set_item(key, value)?;
            }
            Ok(result.into_any().unbind())
        }
    }
}

#[pyfunction]
pub(crate) fn validate_rgb(py: Python<'_>, pixels: &Bound<'_, PyAny>) -> PyResult<()> {
    let pixels = rgb_buffer(pixels)?;
    let shape = pixels.shape();
    let image = ImageView::new(c_slice(&pixels)?, shape[0], shape[1]);
    py.detach(|| valid_rgb(image))?;
    Ok(())
}

#[pyfunction]
fn damage_backend(py: Python<'_>, device: &str) -> PyResult<&'static str> {
    Ok(py.detach(|| select_backend(device))?)
}

#[pyfunction]
fn damage_available(py: Python<'_>, device: &str) -> bool {
    py.detach(|| accelerator::available(device))
}

type PyDelivery = (
    Py<PyAny>,
    Py<PyAny>,
    Option<Py<PyAny>>,
    Py<PyAny>,
    Option<String>,
);

#[pyfunction]
#[pyo3(signature = (candidate, target, heatmap, options))]
fn measure_delivery(
    py: Python<'_>,
    candidate: &Bound<'_, PyAny>,
    target: &Bound<'_, PyAny>,
    heatmap: Option<&Bound<'_, PyAny>>,
    options: (Option<String>, String, bool, bool),
) -> PyResult<PyDelivery> {
    let target = rgb_buffer(target)?;
    let candidate = rgb_buffer(candidate)?;
    let target_shape = target.shape();
    let candidate_shape = candidate.shape();
    let heatmap = heatmap
        .map(|map| heatmap_buffer(map, [target_shape[0], target_shape[1]]))
        .transpose()?;
    let target = ImageView::new(c_slice(&target)?, target_shape[0], target_shape[1]);
    let candidate = ImageView::new(c_slice(&candidate)?, candidate_shape[0], candidate_shape[1]);
    let heatmap = heatmap.as_ref().map(c_slice).transpose()?;
    let (details, device, attention, retry_on_oom) = options;
    let result = py.detach(|| {
        measure(
            candidate,
            target,
            Options {
                details: details.as_deref(),
                device: &device,
                heatmap,
                attention,
                retry_on_oom,
            },
        )
    })?;
    Ok((
        python_value(py, result.features, "")?,
        python_value(py, result.diagnostics, "")?,
        result
            .explanation
            .map(|value| python_value(py, value, ""))
            .transpose()?,
        python_value(py, result.alignment, "")?,
        result.digest,
    ))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add(
        "DeviceOutOfMemory",
        module.py().get_type::<DeviceOutOfMemory>(),
    )?;
    module.add_function(wrap_pyfunction!(damage_backend, module)?)?;
    module.add_function(wrap_pyfunction!(damage_available, module)?)?;
    module.add_function(wrap_pyfunction!(validate_rgb, module)?)?;
    module.add_function(wrap_pyfunction!(measure_delivery, module)?)?;
    module.add_function(wrap_pyfunction!(_measure_overlap, module)?)?;
    Ok(())
}

/// Measure a prealigned overlap through the same quality engine as a delivery.
#[pyfunction(signature = (candidate, target, support=None))]
fn _measure_overlap(
    py: Python<'_>,
    candidate: &Bound<'_, PyAny>,
    target: &Bound<'_, PyAny>,
    support: Option<numpy::PyReadonlyArray2<'_, u8>>,
) -> PyResult<(Py<PyAny>, Py<PyAny>)> {
    let candidate = rgb_buffer(candidate)?;
    let target = rgb_buffer(target)?;
    let [height, width] = [target.shape()[0], target.shape()[1]];
    if candidate.shape() != target.shape()
        || support
            .as_ref()
            .is_some_and(|mask| mask.shape() != [height, width])
    {
        return Err(PyValueError::new_err(
            "Images and support must share the overlap grid",
        ));
    }
    let candidate = c_slice(&candidate)?;
    let target = c_slice(&target)?;
    let support = support.as_ref().map(c_slice).transpose()?;
    let result = py.detach(|| {
        valid_rgb(ImageView::new(candidate, height, width))?;
        valid_rgb(ImageView::new(target, height, width))?;
        quality::measure(
            candidate, target, height, width, support, None, None, None, "cpu",
        )
        .map_err(Error::operation)
    })?;
    Ok((
        python_value(py, result.features, "")?,
        python_value(py, result.diagnostics, "")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{Error, damage_with_retry};

    #[test]
    fn accelerator_exhaustion_retries_on_cpu_when_enabled() {
        for retry in [false, true] {
            let mut backend = "cuda";
            let mut attempts = Vec::new();
            let result = damage_with_retry(&mut backend, retry, |device| {
                attempts.push(device.to_owned());
                if device == "cuda" {
                    Err(Error::operation("CUDA out of memory".to_owned()))
                } else {
                    Ok(())
                }
            });
            assert_eq!(result.is_ok(), retry);
            if retry {
                assert_eq!(attempts, ["cuda", "cpu"]);
                assert_eq!(backend, "cpu");
            } else {
                assert_eq!(attempts, ["cuda"]);
                assert_eq!(backend, "cuda");
            }
        }
    }

    #[test]
    fn other_accelerator_errors_are_returned_without_retry() {
        let mut backend = "mps";
        let mut attempts = 0;
        let result: Result<(), _> = damage_with_retry(&mut backend, true, |_| {
            attempts += 1;
            Err(Error::operation("Metal command buffer failed".to_owned()))
        });
        assert!(matches!(result, Err(Error::Runtime(_))));
        assert_eq!(attempts, 1);
        assert_eq!(backend, "mps");
    }

    #[test]
    fn cubecl_allocation_and_launch_errors_are_retriable() {
        for message in [
            "out of device memory allocating 33554432 bytes",
            "An out-of-memory error happened during launch",
            "CUDA_ERROR_OUT_OF_MEMORY",
        ] {
            assert!(Error::operation(message.to_owned()).is_out_of_memory());
        }
    }
}
