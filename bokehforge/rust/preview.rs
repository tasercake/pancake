use numpy::ndarray::Array3;
use numpy::{IntoPyArray, PyArray3, PyReadonlyArray3, PyUntypedArrayMethods};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use crate::c_slice;

const BUCKET_BITS: u32 = 12;
const BUCKET_SIZE: u32 = 1 << BUCKET_BITS;

#[pyclass(frozen)]
struct PreviewEncoder {
    first: u32,
    last: u32,
    buckets: Vec<u32>,
}

#[pymethods]
impl PreviewEncoder {
    #[new]
    fn new(cutoffs: Vec<u32>) -> PyResult<Self> {
        if cutoffs.len() != 255
            || cutoffs[0] == 0
            || cutoffs[254] > 1_f32.to_bits()
            || cutoffs
                .windows(2)
                .any(|pair| pair[0] >> BUCKET_BITS >= pair[1] >> BUCKET_BITS)
        {
            return Err(PyValueError::new_err("Invalid sRGB preview cutoffs"));
        }
        let first = (cutoffs[0] - 1) & !(BUCKET_SIZE - 1);
        let last = cutoffs[254] | (BUCKET_SIZE - 1);
        let buckets = (first..=last)
            .step_by(BUCKET_SIZE as usize)
            .map(|start| {
                let code = cutoffs.partition_point(|&cutoff| cutoff <= start) as u32;
                let bias = cutoffs
                    .get(code as usize)
                    .map_or(0, |&cutoff| (start + BUCKET_SIZE).saturating_sub(cutoff));
                // A bucket spans at most one cutoff. Adding the low bits
                // crosses into the next byte value exactly at that cutoff.
                (code << BUCKET_BITS) + bias
            })
            .collect();
        Ok(Self {
            first,
            last,
            buckets,
        })
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
                .map(|sample| {
                    // Positive finite f32 values sort by their bit patterns.
                    // A signed comparison puts all negative values below black.
                    let bits =
                        (sample.to_bits() as i32).clamp(self.first as i32, self.last as i32) as u32;
                    let bucket = self.buckets[((bits - self.first) >> BUCKET_BITS) as usize];
                    ((bucket + (bits & (BUCKET_SIZE - 1))) >> BUCKET_BITS) as u8
                })
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
