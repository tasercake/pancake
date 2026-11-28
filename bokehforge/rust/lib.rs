use std::sync::LazyLock;

use numpy::{
    Element, PyArrayDescrMethods, PyArrayMethods, PyReadonlyArray, PyUntypedArrayMethods,
    ndarray::Dimension,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};

mod align;
mod damage;
mod pixels;
mod pool;
mod preview;
mod score;

static WORKERS: LazyLock<ThreadPool> = LazyLock::new(|| {
    let available = std::thread::available_parallelism().map_or(1, usize::from);
    ThreadPoolBuilder::new()
        .num_threads(available.min(8))
        .thread_name(|index| format!("bokehforge-{index}"))
        .build()
        .expect("failed to start BokehForge worker threads")
});

pub(crate) fn parallel<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    WORKERS.install(work)
}

pub(crate) fn c_slice<'a, T: Element, D: Dimension>(
    array: &'a PyReadonlyArray<'_, T, D>,
) -> PyResult<&'a [T]> {
    if !array.is_c_contiguous()
        || !array.data().is_aligned()
        || array.dtype().is_native_byteorder() == Some(false)
    {
        return Err(PyValueError::new_err(
            "Expected an aligned, native-endian, row-major array",
        ));
    }
    Ok(array.as_slice()?)
}

#[pymodule]
fn _rust(module: &Bound<'_, PyModule>) -> PyResult<()> {
    align::register(module)?;
    damage::register(module)?;
    pixels::register(module)?;
    pool::register(module)?;
    preview::register(module)?;
    score::register(module)?;
    Ok(())
}
