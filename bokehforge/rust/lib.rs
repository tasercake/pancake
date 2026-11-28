use std::sync::LazyLock;

use numpy::{
    Element, PyArrayDescrMethods, PyArrayMethods, PyReadonlyArray, PyUntypedArrayMethods,
    ndarray::Dimension,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};

mod accelerator;
mod align;
mod attention;
mod benchmark;
mod delivery_io;
mod engine;
mod pixels;
mod pool;
mod preview;
mod quality;
mod registration;
mod score;
mod srgb;

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
    accelerator::register(module)?;
    align::register(module)?;
    attention::register(module)?;
    benchmark::register(module)?;
    delivery_io::register(module)?;
    engine::register(module)?;
    pixels::register(module)?;
    pool::register(module)?;
    preview::register(module)?;
    score::register(module)?;
    Ok(())
}
