use numpy::{IntoPyArray, PyArray2, PyReadonlyArray2, PyUntypedArrayMethods, ndarray::Array2};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;

use crate::c_slice;

/// Return the native lightness planes before pooling for numerical comparisons.
#[pyfunction]
#[pyo3(signature = (candidate, target, delta, support, device))]
fn _accelerated_lightness<'py>(
    py: Python<'py>,
    candidate: PyReadonlyArray2<'py, f32>,
    target: PyReadonlyArray2<'py, f32>,
    delta: PyReadonlyArray2<'py, f32>,
    support: Option<PyReadonlyArray2<'py, u8>>,
    device: &str,
) -> PyResult<[Bound<'py, PyArray2<f32>>; 4]> {
    let shape = target.shape();
    if shape.contains(&0)
        || candidate.shape() != shape
        || delta.shape() != shape
        || support.as_ref().is_some_and(|mask| mask.shape() != shape)
    {
        return Err(PyValueError::new_err(
            "Lightness damage input plane shapes do not match or are empty",
        ));
    }
    let (height, width) = (shape[0], shape[1]);
    // Own the inputs while the native backend runs with Python detached.
    let candidate = c_slice(&candidate)?.to_vec();
    let target = c_slice(&target)?.to_vec();
    let delta = c_slice(&delta)?.to_vec();
    let support = support
        .as_ref()
        .map(|mask| c_slice(mask).map(<[_]>::to_vec))
        .transpose()?;
    let maps = py
        .detach(|| {
            super::lightness(
                device,
                height,
                width,
                &candidate,
                &target,
                &delta,
                support.as_deref(),
            )
        })
        .map_err(PyRuntimeError::new_err)?;
    Ok(maps.map(|samples| {
        Array2::from_shape_vec((height, width), samples)
            .expect("damage planes have the same shape as the inputs")
            .into_pyarray(py)
    }))
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(_accelerated_lightness, module)?)?;
    Ok(())
}
