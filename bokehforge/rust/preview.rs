use numpy::ndarray::Array3;
use numpy::{IntoPyArray, PyArray3, PyReadonlyArray3, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::c_slice;
use crate::srgb::SrgbEncoder;

#[pyclass(frozen)]
struct PreviewEncoder {
    encoder: SrgbEncoder,
}

#[pymethods]
impl PreviewEncoder {
    #[new]
    fn new(cutoffs: Vec<u32>) -> PyResult<Self> {
        let encoder = SrgbEncoder::new(&cutoffs)
            .ok_or_else(|| PyValueError::new_err("Invalid sRGB preview cutoffs"))?;
        Ok(Self { encoder })
    }

    fn encode<'py>(
        &self,
        py: Python<'py>,
        reference: PyReadonlyArray3<'py, f32>,
    ) -> PyResult<Bound<'py, PyArray3<u8>>> {
        let &[height, width, channels] = reference.shape() else {
            unreachable!("PyReadonlyArray3 has exactly three axes")
        };
        let samples = c_slice(&reference)?;
        let encoded = py.detach(|| {
            samples
                .iter()
                .map(|&sample| self.encoder.encode(sample))
                .collect::<Vec<_>>()
        });
        Ok(Array3::from_shape_vec((height, width, channels), encoded)
            .expect("preview output keeps the reference strip shape")
            .into_pyarray(py))
    }
}

pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PreviewEncoder>()
}
