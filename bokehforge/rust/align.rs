use std::mem::size_of;

use numpy::{IntoPyArray, PyArray2, ndarray::Array2};
use pyo3::buffer::{Element, PyBuffer};
use pyo3::exceptions::{PyBufferError, PyValueError};
use pyo3::prelude::*;

struct Buffer<T, const N: usize> {
    buffer: PyBuffer<T>,
    shape: [usize; N],
    strides: [isize; N],
}

impl<T: Element, const N: usize> Buffer<T, N> {
    fn get(object: &Bound<'_, PyAny>, format: &[u8], writeable: bool) -> PyResult<Self> {
        let alignment_error =
            || PyValueError::new_err("Expected an aligned native floating-point buffer");
        let buffer = PyBuffer::<T>::get(object).map_err(|error| {
            if error.is_instance_of::<PyBufferError>(object.py()) {
                alignment_error()
            } else {
                error
            }
        })?;
        if writeable && buffer.readonly() {
            return Err(PyValueError::new_err(
                "Alignment sample output must be writeable",
            ));
        }
        if buffer.format().to_bytes() != format
            || buffer
                .suboffsets()
                .is_some_and(|values| values.iter().any(|&v| v >= 0))
        {
            return Err(alignment_error());
        }
        let shape = buffer.shape().try_into().map_err(|_| alignment_error())?;
        let mut strides: [isize; N] = buffer.strides().try_into().map_err(|_| alignment_error())?;
        for stride in &mut strides {
            if *stride % size_of::<T>() as isize != 0 {
                return Err(PyValueError::new_err(
                    "Expected aligned floating-point strides",
                ));
            }
            *stride /= size_of::<T>() as isize;
        }
        Ok(Self {
            buffer,
            shape,
            strides,
        })
    }

    #[inline]
    fn data(&self) -> *mut T {
        self.buffer.buf_ptr().cast()
    }
}

struct SampleGrid<'a> {
    image: &'a Buffer<f32, 3>,
    height: usize,
    width: usize,
    row: isize,
    column: isize,
    channel: isize,
}

impl<'a> SampleGrid<'a> {
    fn new(image: &'a Buffer<f32, 3>, rgb: bool) -> PyResult<Self> {
        let (y, x, channel) = if rgb { (0, 1, 2) } else { (1, 2, 0) };
        if image.shape[y] == 0 || image.shape[x] == 0 || image.shape[channel] != 3 {
            return Err(shape_error());
        }
        Ok(Self {
            image,
            height: image.shape[y],
            width: image.shape[x],
            row: image.strides[y],
            column: image.strides[x],
            channel: image.strides[channel],
        })
    }

    #[inline]
    fn sample(&self, y: f64, x: f64) -> [f32; 3] {
        let top = y.floor();
        let left = x.floor();
        // Rust's float-to-integer cast saturates at zero for negative coordinates.
        let y0 = (top as usize).min(self.height - 1) as isize * self.row;
        let x0 = (left as usize).min(self.width - 1) as isize * self.column;
        let load = |row, column, channel| {
            // The spatial indices are clamped to the image and the channel
            // iterates over its three values. All strides are aligned.
            unsafe { self.image.data().offset(row + column + channel).read() }
        };
        if y == top && x == left {
            // SciPy interpolation accumulates from positive zero on grid points too.
            return [0, self.channel, 2 * self.channel].map(|channel| 0.0 + load(y0, x0, channel));
        }
        let y1 = ((top + 1.0) as usize).min(self.height - 1) as isize * self.row;
        let x1 = ((left + 1.0) as usize).min(self.width - 1) as isize * self.column;
        let wy0 = 1.0 - (y - top);
        let wx0 = 1.0 - (x - left);
        let wy1 = 1.0 - wy0;
        let wx1 = 1.0 - wx0;
        [0, self.channel, 2 * self.channel].map(|channel| {
            // Follow SciPy order-1 interpolation's weight and accumulation order.
            let mut value = 0.0;
            value += (f64::from(load(y0, x0, channel)) * wy0) * wx0;
            value += (f64::from(load(y0, x1, channel)) * wy0) * wx1;
            value += (f64::from(load(y1, x0, channel)) * wy1) * wx0;
            value += (f64::from(load(y1, x1, channel)) * wy1) * wx1;
            value as f32
        })
    }

    #[inline]
    fn native_delta(&self, y: f64, x: f64) -> [f64; 3] {
        let sample = |y, x| {
            self.sample(y, x)
                .map(|value| f64::from(value.clamp(0.0, 1.0)))
        };
        let center = sample(y, x);
        let above = sample(y - 3.0, x);
        let below = sample(y + 3.0, x);
        let left = sample(y, x - 3.0);
        let right = sample(y, x + 3.0);
        std::array::from_fn(|c| center[c] - (above[c] + below[c] + left[c] + right[c]) * 0.25)
    }
}

fn shape_error() -> PyErr {
    PyValueError::new_err("Alignment sample buffer shapes do not match")
}

#[inline]
fn read_point(points: &Buffer<f64, 2>, index: usize) -> (f64, f64) {
    // Points has shape (2, count) and index is within the sample axis.
    unsafe {
        let point = points.data().offset(index as isize * points.strides[1]);
        (point.read(), point.offset(points.strides[0]).read())
    }
}

#[inline]
fn sample_point(points: &Buffer<f64, 2>, index: usize) -> PyResult<(f64, f64)> {
    let (y, x) = read_point(points, index);
    if y.is_finite() && x.is_finite() {
        Ok((y, x))
    } else {
        Err(PyValueError::new_err(
            "Alignment sample coordinates must be finite",
        ))
    }
}

#[pyfunction]
fn sample_linear(
    py: Python<'_>,
    image: &Bound<'_, PyAny>,
    points: &Bound<'_, PyAny>,
    output: &Bound<'_, PyAny>,
    rgb: &Bound<'_, PyAny>,
) -> PyResult<()> {
    let rgb = rgb.is_truthy()?;
    let image = Buffer::<f32, 3>::get(image, b"f", false)?;
    let points = Buffer::<f64, 2>::get(points, b"d", false)?;
    let output = Buffer::<f32, 2>::get(output, b"f", true)?;
    let grid = SampleGrid::new(&image, rgb)?;
    let count = points.shape[1];
    let (sample_axis, channel_axis) = if rgb { (0, 1) } else { (1, 0) };
    if points.shape[0] != 2 || output.shape[sample_axis] != count || output.shape[channel_axis] != 3
    {
        return Err(shape_error());
    }
    py.detach(|| {
        for index in 0..count {
            let (y, x) = sample_point(&points, index)?;
            let values = grid.sample(y, x);
            for (channel, value) in values.into_iter().enumerate() {
                let offset = index as isize * output.strides[sample_axis]
                    + channel as isize * output.strides[channel_axis];
                // Each offset indexes a valid element of the writable output buffer.
                unsafe { output.data().offset(offset).write(value) };
            }
        }
        Ok(())
    })
}

#[pyfunction]
fn sample_native_delta<'py>(
    py: Python<'py>,
    image: &Bound<'py, PyAny>,
    points: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let image = Buffer::<f32, 3>::get(image, b"f", false)?;
    let points = Buffer::<f64, 2>::get(points, b"d", false)?;
    let grid = SampleGrid::new(&image, true)?;
    if points.shape[0] != 2 {
        return Err(shape_error());
    }
    let count = points.shape[1];
    let output = py.detach(|| -> PyResult<_> {
        let mut output = Vec::with_capacity(count * 3);
        for index in 0..count {
            let (y, x) = sample_point(&points, index)?;
            output.extend(grid.native_delta(y, x));
        }
        Ok(rgb_deltas(count, output))
    })?;
    Ok(output.into_pyarray(py))
}

type EvidenceSamples<'py> = (
    f64,
    Option<Bound<'py, PyAny>>,
    Option<Bound<'py, PyArray2<f32>>>,
);

enum Samples {
    Texture(Array2<f32>),
    RgbDelta(Array2<f64>),
}

fn rgb_deltas(count: usize, values: Vec<f64>) -> Array2<f64> {
    Array2::from_shape_vec((count, 3), values).expect("three delta channels per sample")
}

#[pyfunction(signature = (image, points, values, bounds, step))]
fn sample_pose_evidence<'py>(
    py: Python<'py>,
    image: &Bound<'py, PyAny>,
    points: &Bound<'py, PyAny>,
    values: &Bound<'py, PyAny>,
    bounds: [f64; 2],
    step: Option<[f64; 2]>,
) -> PyResult<EvidenceSamples<'py>> {
    let image = Buffer::<f32, 3>::get(image, b"f", false)?;
    let points = Buffer::<f64, 2>::get(points, b"d", false)?;
    let values = Buffer::<f32, 2>::get(values, b"f", false)?;
    let grid = SampleGrid::new(&image, step.is_none())?;
    let count = points.shape[1];
    if points.shape[0] != 2 || values.shape != [3, count] {
        return Err(shape_error());
    }
    let (support, samples, selected) = py.detach(|| {
        let mut valid = Vec::with_capacity(count);
        for index in 0..count {
            let (y, x) = read_point(&points, index);
            if y >= 0.0 && y <= bounds[0] && x >= 0.0 && x <= bounds[1] {
                valid.push((index, y, x));
            }
        }
        let support = valid.len() as f64 / count.max(1) as f64;
        if valid.len() < 4 {
            return (support, None, None);
        }

        let samples = match step {
            Some([dy, dx]) => {
                let mut samples = Array2::zeros((3, valid.len()));
                for (sample, &(_, y, x)) in valid.iter().enumerate() {
                    let pixel = grid.sample((y + 0.5) / dy - 0.5, (x + 0.5) / dx - 0.5);
                    for (channel, value) in pixel.into_iter().enumerate() {
                        samples[[channel, sample]] = value;
                    }
                }
                Samples::Texture(samples)
            }
            None => {
                let values = valid
                    .iter()
                    .flat_map(|&(_, y, x)| grid.native_delta(y, x))
                    .collect();
                Samples::RgbDelta(rgb_deltas(valid.len(), values))
            }
        };

        // Match the layout of NumPy's Boolean column selection so Pearson
        // centering sees the same strides and reduction order.
        let selected = (valid.len() != count).then(|| {
            Array2::from_shape_fn((valid.len(), 3), |(sample, channel)| {
                let offset = valid[sample].0 as isize * values.strides[1]
                    + channel as isize * values.strides[0];
                // The valid index and channel are within the checked shape.
                unsafe { values.data().offset(offset).read() }
            })
            .reversed_axes()
        });
        (support, Some(samples), selected)
    });
    Ok((
        support,
        samples.map(|samples| match samples {
            Samples::Texture(a) => a.into_pyarray(py).into_any(),
            Samples::RgbDelta(a) => a.into_pyarray(py).into_any(),
        }),
        selected.map(|a| a.into_pyarray(py)),
    ))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(sample_linear, module)?)?;
    module.add_function(wrap_pyfunction!(sample_native_delta, module)?)?;
    module.add_function(wrap_pyfunction!(sample_pose_evidence, module)?)?;
    Ok(())
}
