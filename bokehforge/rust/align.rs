use std::mem::size_of;

use numpy::{
    IntoPyArray, PyArray2, PyArray3, PyUntypedArrayMethods,
    ndarray::{Array2, Array3},
};
use pyo3::buffer::{Element, PyBuffer};
use pyo3::exceptions::{PyBufferError, PyValueError};
use pyo3::prelude::*;

use crate::c_slice;
use crate::engine::{python_value, rgb_buffer, valid_rgb};
use crate::registration::{self, ImageView, Placement, Pose};

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
        // A float-to-integer cast saturates at zero for negative coordinates.
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

type PoseValues = (f64, f64, f64, f64);
type PlacementRow = (PoseValues, [usize; 4], f64, Py<PyAny>, Vec<Py<PyAny>>);

fn geometry_placement((y, x, angle, scale): PoseValues, bounds: [usize; 4]) -> PyResult<Placement> {
    if ![y, x, angle, scale].iter().all(|v| v.is_finite()) || scale <= 0.0 {
        return Err(PyValueError::new_err(
            "Expected a finite alignment pose with positive scale",
        ));
    }
    Ok(Placement {
        pose: Pose { y, x, angle, scale },
        bounds,
        weight: 1.0,
        evidence: serde_json::Value::Null,
        padding: Vec::new(),
    })
}

#[pyfunction]
fn registration_align(
    py: Python<'_>,
    candidate: &Bound<'_, PyAny>,
    reference: &Bound<'_, PyAny>,
) -> PyResult<(Vec<PlacementRow>, Py<PyAny>)> {
    let candidate = rgb_buffer(candidate)?;
    let reference = rgb_buffer(reference)?;
    let view = |pixels, shape: &[usize]| ImageView::new(pixels, shape[0], shape[1]);
    let crop = view(c_slice(&candidate)?, candidate.shape());
    let target = view(c_slice(&reference)?, reference.shape());
    let (placements, trace) = py.detach(|| {
        valid_rgb(crop)?;
        valid_rgb(target)?;
        Ok::<_, crate::engine::Error>(registration::align(crop, target))
    })?;
    let rows = placements
        .into_iter()
        .map(|placement| {
            let Pose { y, x, angle, scale } = placement.pose;
            let padding = placement
                .padding
                .into_iter()
                .map(|value| python_value(py, value, ""))
                .collect::<PyResult<_>>()?;
            Ok((
                (y, x, angle, scale),
                placement.bounds,
                placement.weight,
                python_value(py, placement.evidence, "")?,
                padding,
            ))
        })
        .collect::<PyResult<_>>()?;
    Ok((rows, python_value(py, trace, "")?))
}

#[pyfunction]
fn registration_overlap(
    pose: PoseValues,
    bounds: [usize; 4],
    shape: [usize; 2],
) -> PyResult<(f64, f64, Vec<[f64; 2]>)> {
    if shape.contains(&0) || shape[0].checked_mul(shape[1]).is_none() {
        return Err(PyValueError::new_err("Invalid reference grid"));
    }
    Ok(registration::overlap(
        &geometry_placement(pose, bounds)?,
        shape,
    ))
}

type Projected<'py> = (
    Bound<'py, PyArray3<f32>>,
    Option<Bound<'py, PyArray2<bool>>>,
);

#[pyfunction]
fn registration_project<'py>(
    py: Python<'py>,
    candidate: &Bound<'py, PyAny>,
    shape: [usize; 2],
    pose: PoseValues,
    bounds: [usize; 4],
) -> PyResult<Projected<'py>> {
    let placement = geometry_placement(pose, bounds)?;
    let candidate = rgb_buffer(candidate)?;
    let crop = ImageView::new(
        c_slice(&candidate)?,
        candidate.shape()[0],
        candidate.shape()[1],
    );
    let [top, left, bottom, right] = bounds;
    let [height, width] = shape;
    if shape.contains(&0)
        || height
            .checked_mul(width)
            .and_then(|n| n.checked_mul(3))
            .is_none()
        || !(top < bottom && left < right && bottom <= crop.h && right <= crop.w)
    {
        return Err(PyValueError::new_err(
            "Invalid delivery bounds or reference grid",
        ));
    }
    let (pixels, support) = py.detach(|| registration::project(crop, shape, &placement));
    let pixels = Array3::from_shape_vec((height, width, 3), pixels.into_owned())
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
    let support = support
        .map(|mask| {
            Array2::from_shape_vec(
                (height, width),
                mask.into_iter().map(|value| value != 0).collect(),
            )
            .map(|mask| mask.into_pyarray(py))
            .map_err(|error| PyValueError::new_err(error.to_string()))
        })
        .transpose()?;
    Ok((pixels.into_pyarray(py), support))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(sample_linear, module)?)?;
    module.add_function(wrap_pyfunction!(sample_native_delta, module)?)?;
    module.add_function(wrap_pyfunction!(sample_pose_evidence, module)?)?;
    module.add_function(wrap_pyfunction!(registration_align, module)?)?;
    module.add_function(wrap_pyfunction!(registration_overlap, module)?)?;
    module.add_function(wrap_pyfunction!(registration_project, module)?)?;
    Ok(())
}
