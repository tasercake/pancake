use std::ops::Range;

use numpy::ndarray::{Array2, Dimension};
use numpy::{
    Element, IntoPyArray, PyArray2, PyReadonlyArray, PyReadonlyArray1, PyReadonlyArray2,
    PyUntypedArrayMethods,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::prelude::*;

const DIAGONAL: f64 = std::f64::consts::SQRT_2;
const FILTER_RADII: [usize; 4] = [3, 4, 10, 40];

#[derive(Clone, Copy)]
struct Grid {
    height: usize,
    width: usize,
}

impl Grid {
    fn halo(self, rows: &Range<usize>, radius: usize) -> Range<usize> {
        rows.start.saturating_sub(radius)..(rows.end + radius).min(self.height)
    }

    fn indices(self, rows: Range<usize>) -> Range<usize> {
        rows.start * self.width..rows.end * self.width
    }
}

#[derive(Clone, Copy)]
struct Plane<'a> {
    values: &'a [f32],
    first_row: usize,
}

impl<'a> Plane<'a> {
    fn row(self, row: usize, width: usize) -> &'a [f32] {
        let first = (row - self.first_row) * width;
        &self.values[first..first + width]
    }
}

fn reflect(index: isize, length: usize) -> usize {
    let length = length as isize;
    if (0..length).contains(&index) {
        return index as usize;
    }
    let position = index.rem_euclid(2 * length);
    position.min(2 * length - 1 - position) as usize
}

#[inline]
fn weighted_pairs(sum: &mut [f64], left: &[f32], right: &[f32], weight: f64) {
    for ((accumulator, &a), &b) in sum.iter_mut().zip(left).zip(right) {
        *accumulator += (f64::from(a) + f64::from(b)) * weight;
    }
}

fn blur(grid: Grid, source: Plane<'_>, rows: Range<usize>, weights: &[f64]) -> Vec<f32> {
    let width = grid.width;
    let radius = weights.len() - 1;
    let mut sum = vec![0.; width];
    let mut padded = vec![0.; width + 2 * radius];
    let mut result = vec![0.; rows.len() * width];

    // SciPy filters the vertical axis first and stores the intermediate in
    // float32. For its symmetric kernels the center is accumulated first,
    // followed by the reflected pairs from the outermost inward.
    for (y, output) in rows.zip(result.chunks_exact_mut(width)) {
        let center = source.row(y, width);
        for (accumulator, &sample) in sum.iter_mut().zip(center) {
            *accumulator = f64::from(sample) * weights[radius];
        }
        for (offset, &weight) in (1..=radius).rev().zip(weights) {
            let left = source.row(reflect(y as isize - offset as isize, grid.height), width);
            let right = source.row(reflect(y as isize + offset as isize, grid.height), width);
            weighted_pairs(&mut sum, left, right, weight);
        }
        for (sample, &accumulator) in padded[radius..radius + width].iter_mut().zip(&sum) {
            *sample = accumulator as f32;
        }
        for x in 0..radius {
            padded[x] = padded[radius + reflect(x as isize - radius as isize, width)];
            padded[radius + width + x] = padded[radius + reflect((width + x) as isize, width)];
        }

        for (accumulator, &sample) in sum.iter_mut().zip(&padded[radius..radius + width]) {
            *accumulator = f64::from(sample) * weights[radius];
        }
        for (offset, &weight) in (1..=radius).rev().zip(weights) {
            let left = radius - offset;
            let right = radius + offset;
            weighted_pairs(
                &mut sum,
                &padded[left..left + width],
                &padded[right..right + width],
                weight,
            );
        }
        for (sample, &accumulator) in output.iter_mut().zip(&sum) {
            *sample = accumulator as f32;
        }
    }
    result
}

fn normalize(values: &mut [f32], weights: &[f32]) {
    for (value, &weight) in values.iter_mut().zip(weights) {
        *value = if weight > 0. { *value / weight } else { 0. };
    }
}

#[inline]
fn visibility(error: f32, scale: f32) -> f32 {
    error / (error + scale)
}

#[inline]
fn contrast_pair<const IS_DIAGONAL: bool>(candidate: [f32; 2], target: [f32; 2]) -> [f32; 2] {
    // Subtract in float32, as NumPy does, before promoting the directional
    // distance and its visibility response to float64.
    let mut c = f64::from(candidate[1] - candidate[0]);
    let mut t = f64::from(target[1] - target[0]);
    if IS_DIAGONAL {
        c /= DIAGONAL;
        t /= DIAGONAL;
    }
    let earned = if c * t > 0. { c.abs().min(t.abs()) } else { 0. };
    let loss = t.abs() - earned;
    let gain = c.abs() - earned;
    [(loss / (loss + 5.)) as f32, (gain / (gain + 5.)) as f32]
}

#[inline]
fn record(pair: [f32; 2], cell: &mut [f32; 2]) {
    cell[0] = cell[0].max(pair[0]);
    cell[1] = cell[1].max(pair[1]);
}

fn contrast<const MASKED: bool>(
    grid: Grid,
    candidate: &[f32],
    target: &[f32],
    support: &[u8],
    output: &mut [[f32; 2]],
) {
    let width = grid.width;
    for first in (0..candidate.len()).step_by(width) {
        let last = first + width;
        let c = &candidate[first..last];
        let t = &target[first..last];
        let row = &mut output[first..last];
        for x in 0..width - 1 {
            if !MASKED || (support[first + x] != 0 && support[first + x + 1] != 0) {
                let pair = contrast_pair::<false>([c[x], c[x + 1]], [t[x], t[x + 1]]);
                record(pair, &mut row[x]);
                record(pair, &mut row[x + 1]);
            }
        }

        if last == candidate.len() {
            break;
        }
        let next_c = &candidate[last..last + width];
        let next_t = &target[last..last + width];
        let (previous, next) = output.split_at_mut(last);
        let first_row = &mut previous[first..last];
        let next_row = &mut next[..width];
        for x in 0..width {
            if !MASKED || (support[first + x] != 0 && support[last + x] != 0) {
                let pair = contrast_pair::<false>([c[x], next_c[x]], [t[x], next_t[x]]);
                record(pair, &mut first_row[x]);
                record(pair, &mut next_row[x]);
            }
        }
        for x in 0..width - 1 {
            if !MASKED || (support[first + x] != 0 && support[last + x + 1] != 0) {
                let pair = contrast_pair::<true>([c[x], next_c[x + 1]], [t[x], next_t[x + 1]]);
                record(pair, &mut first_row[x]);
                record(pair, &mut next_row[x + 1]);
            }
            if !MASKED || (support[last + x] != 0 && support[first + x + 1] != 0) {
                let pair = contrast_pair::<true>([next_c[x], c[x + 1]], [next_t[x], t[x + 1]]);
                record(pair, &mut next_row[x]);
                record(pair, &mut first_row[x + 1]);
            }
        }
    }
}

fn extrema(grid: Grid, target: Plane<'_>, rows: Range<usize>) -> [Vec<f32>; 4] {
    let width = grid.width;
    let edge_rows = grid.halo(&rows, 4);
    let mut upper = vec![0.; rows.len() * width];
    let mut lower = vec![0.; rows.len() * width];
    let mut strength = vec![0.; edge_rows.len() * width];
    let mut vertical = vec![[0.; 2]; width];
    // Reflection repeats samples already visited by these extrema windows.
    // Clamping the footprint at the actual image edge gives the same value.
    for (y, edge) in edge_rows.clone().zip(strength.chunks_exact_mut(width)) {
        let top = target.row(y.saturating_sub(1), width);
        let middle = target.row(y, width);
        let bottom = target.row((y + 1).min(grid.height - 1), width);
        for (((pair, &a), &b), &c) in vertical.iter_mut().zip(top).zip(middle).zip(bottom) {
            *pair = [a.max(b).max(c), a.min(b).min(c)];
        }
        let row_offset = rows.contains(&y).then(|| (y - rows.start) * width);
        for (x, strength) in edge.iter_mut().enumerate() {
            let [a, b] = vertical[x.saturating_sub(1)];
            let [c, d] = vertical[x];
            let [e, f] = vertical[(x + 1).min(width - 1)];
            let high = a.max(c).max(e);
            let low = b.min(d).min(f);
            if let Some(offset) = row_offset {
                upper[offset + x] = high;
                lower[offset + x] = low;
            }
            *strength = visibility(high - low, 6.);
        }
    }

    let mut horizontal = strength.clone();
    for (output, edge) in horizontal
        .chunks_exact_mut(width)
        .zip(strength.chunks_exact(width))
    {
        for (x, strongest) in output.iter_mut().enumerate() {
            for &value in &edge[x.saturating_sub(4)..(x + 5).min(width)] {
                *strongest = strongest.max(value);
            }
        }
    }
    let mut near_edge = vec![0f32; rows.len() * width];
    let first = (rows.start - edge_rows.start) * width;
    for (y, row) in rows.zip(near_edge.chunks_exact_mut(width)) {
        let neighbors = y.saturating_sub(4).max(edge_rows.start)..(y + 5).min(edge_rows.end);
        for neighbor in neighbors {
            let index = (neighbor - edge_rows.start) * width;
            for (strongest, &value) in row.iter_mut().zip(&horizontal[index..index + width]) {
                *strongest = strongest.max(value);
            }
        }
    }
    strength.copy_within(first..first + near_edge.len(), 0);
    strength.truncate(near_edge.len());
    [upper, lower, strength, near_edge]
}

struct Inputs<'a> {
    grid: Grid,
    candidate: &'a [f32],
    target: &'a [f32],
    delta: &'a [f32],
    support: Option<&'a [u8]>,
    weights: &'a [Vec<f64>; 4],
    extrema: Option<[&'a [f32]; 4]>,
}

impl Inputs<'_> {
    fn strip(&self, rows: Range<usize>, outputs: [&mut [f32]; 4]) {
        let grid = self.grid;
        let [tone, lost, extra, boundary] = outputs;
        let detail_rows = grid.halo(&rows, 1);
        let filter_rows = grid.halo(&rows, FILTER_RADII[3]);
        let contrast_rows = grid.halo(&detail_rows, FILTER_RADII[2]);
        let support_values = self.support.map(|support| {
            support[grid.indices(filter_rows.clone())]
                .iter()
                .map(|&valid| u8::from(valid != 0) as f32)
                .collect::<Vec<_>>()
        });
        let weights = support_values.as_deref().map(|values| Plane {
            values,
            first_row: filter_rows.start,
        });

        let mask = |values: &[f32], source_rows: Range<usize>| {
            let mask_start = (source_rows.start - filter_rows.start) * grid.width;
            let samples = &values[grid.indices(source_rows)];
            let mask_values = &support_values.as_deref().unwrap_or_default()
                [mask_start..mask_start + samples.len()];
            samples
                .iter()
                .zip(mask_values)
                .map(|(&value, &weight)| value * weight)
                .collect::<Vec<_>>()
        };
        let candidate = Plane {
            values: self.candidate,
            first_row: 0,
        };
        let target = Plane {
            values: self.target,
            first_row: 0,
        };
        let delta = Plane {
            values: self.delta,
            first_row: 0,
        };
        let (masked_c, masked_t, masked_d);
        let (filtered_c, filtered_t, filtered_d) = if weights.is_some() {
            masked_c = mask(self.candidate, contrast_rows.clone());
            masked_t = mask(self.target, contrast_rows.clone());
            masked_d = mask(self.delta, filter_rows.clone());
            (
                Plane {
                    values: &masked_c,
                    first_row: contrast_rows.start,
                },
                Plane {
                    values: &masked_t,
                    first_row: contrast_rows.start,
                },
                Plane {
                    values: &masked_d,
                    first_row: filter_rows.start,
                },
            )
        } else {
            (candidate, target, delta)
        };

        let mut contrasts = vec![[0.; 2]; detail_rows.len() * grid.width];
        let detail_indices = grid.indices(detail_rows.clone());
        let support = self.support.map(|support| &support[detail_indices.clone()]);
        let mut add_contrast = |c: &[f32], t: &[f32]| {
            if let Some(support) = support {
                contrast::<true>(grid, c, t, support, &mut contrasts);
            } else {
                contrast::<false>(grid, c, t, &[], &mut contrasts);
            }
        };
        add_contrast(
            &self.candidate[detail_indices.clone()],
            &self.target[detail_indices],
        );
        let mut local = Vec::new();
        for index in [1, 2] {
            let kernel = &self.weights[index];
            let mut c = blur(grid, filtered_c, detail_rows.clone(), kernel);
            let mut t = blur(grid, filtered_t, detail_rows.clone(), kernel);
            let denominator = weights.map(|source| blur(grid, source, detail_rows.clone(), kernel));
            if let Some(denominator) = &denominator {
                normalize(&mut c, denominator);
                normalize(&mut t, denominator);
            }
            add_contrast(&c, &t);
            if index == 2 {
                local = blur(grid, filtered_d, detail_rows.clone(), kernel);
                if let Some(denominator) = &denominator {
                    normalize(&mut local, denominator);
                }
            }
        }
        let filter_delta = |index: usize| {
            let kernel = &self.weights[index];
            let mut values = blur(grid, filtered_d, rows.clone(), kernel);
            if let Some(source) = weights {
                normalize(&mut values, &blur(grid, source, rows.clone(), kernel));
            }
            values
        };
        let fine = filter_delta(0);
        let broad = filter_delta(3);
        let cpu_extrema;
        let [upper, lower, edge, near] = if let Some(extrema) = self.extrema {
            extrema.map(|plane| &plane[grid.indices(rows.clone())])
        } else {
            cpu_extrema = extrema(grid, target, rows.clone());
            cpu_extrema.each_ref().map(Vec::as_slice)
        };
        let row_offset = (rows.start - detail_rows.start) * grid.width;
        let first = rows.start * grid.width;
        for i in 0..tone.len() {
            let delta = self.delta[first + i];
            let c = self.candidate[first + i];
            let local = local[row_offset + i];
            let [loss, gain] = contrasts[row_offset + i];
            let grain = visibility((delta - fine[i]).abs(), 5.) * (1. - edge[i]);
            let shape = visibility((delta - local).abs(), 6.) * near[i];
            let overshoot = (c - upper[i]).max(lower[i] - c).max(0.);
            tone[i] = visibility(local.abs().max(broad[i].abs()), 8.);
            lost[i] = loss;
            extra[i] = gain.max(grain);
            boundary[i] = shape.max(visibility(overshoot, 6.) * near[i]);
        }
    }

    fn damage(&self) -> [Vec<f32>; 4] {
        let Grid { height, width } = self.grid;
        let len = height * width;
        let mut outputs: [Vec<f32>; 4] = std::array::from_fn(|_| vec![0.; len]);
        let [tone, lost, extra, boundary] = &mut outputs;
        if len < 32_768 {
            self.strip(0..height, [tone, lost, extra, boundary]);
        } else {
            let rows_per_strip = (131_072 / width).clamp(16, 192);
            let size = rows_per_strip * width;
            crate::parallel(|| {
                tone.par_chunks_mut(size)
                    .zip(lost.par_chunks_mut(size))
                    .zip(extra.par_chunks_mut(size))
                    .zip(boundary.par_chunks_mut(size))
                    .enumerate()
                    .for_each(|(index, (((tone, lost), extra), boundary))| {
                        let first = index * rows_per_strip;
                        self.strip(
                            first..first + tone.len() / width,
                            [tone, lost, extra, boundary],
                        );
                    });
            });
        }
        outputs
    }
}

type DamageMaps<'py> = (
    Bound<'py, PyArray2<f32>>,
    Bound<'py, PyArray2<f32>>,
    Bound<'py, PyArray2<f32>>,
    Bound<'py, PyArray2<f32>>,
);

// Copy while Python is attached so filter workers own their inputs.
fn copy_array<T: Element + Copy, D: Dimension>(
    array: &PyReadonlyArray<'_, T, D>,
) -> PyResult<Vec<T>> {
    Ok(crate::c_slice(array)?.to_vec())
}

#[pyfunction]
#[pyo3(signature = (candidate, target, delta, support, weights, extrema=None))]
fn lightness_damage<'py>(
    py: Python<'py>,
    candidate: PyReadonlyArray2<'py, f32>,
    target: PyReadonlyArray2<'py, f32>,
    delta: PyReadonlyArray2<'py, f32>,
    support: Option<PyReadonlyArray2<'py, u8>>,
    weights: [PyReadonlyArray1<'py, f64>; 4],
    extrema: Option<[PyReadonlyArray2<'py, f32>; 4]>,
) -> PyResult<DamageMaps<'py>> {
    let shape = target.shape();
    if shape.contains(&0)
        || candidate.shape() != shape
        || delta.shape() != shape
        || support.as_ref().is_some_and(|mask| mask.shape() != shape)
        || extrema
            .as_ref()
            .is_some_and(|planes| planes.iter().any(|plane| plane.shape() != shape))
    {
        return Err(PyValueError::new_err(
            "Lightness damage input plane shapes do not match or are empty",
        ));
    }
    let (height, width) = (shape[0], shape[1]);
    let mut kernels: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
    for ((kernel, input), radius) in kernels.iter_mut().zip(weights).zip(FILTER_RADII) {
        if input.shape() != [radius + 1] {
            return Err(PyValueError::new_err(
                "Lightness damage Gaussian weight shapes do not match",
            ));
        }
        *kernel = copy_array(&input)?;
    }

    let candidate = copy_array(&candidate)?;
    let target = copy_array(&target)?;
    let delta = copy_array(&delta)?;
    let support = support.as_ref().map(copy_array).transpose()?;
    let extrema = match extrema {
        Some([upper, lower, strength, near]) => Some([
            copy_array(&upper)?,
            copy_array(&lower)?,
            copy_array(&strength)?,
            copy_array(&near)?,
        ]),
        None => None,
    };
    let inputs = Inputs {
        grid: Grid { height, width },
        candidate: &candidate,
        target: &target,
        delta: &delta,
        support: support.as_deref(),
        weights: &kernels,
        extrema: extrema
            .as_ref()
            .map(|planes| planes.each_ref().map(Vec::as_slice)),
    };
    let [tone, lost, extra, boundary] = py.detach(|| inputs.damage());
    let into_numpy = |values| {
        Array2::from_shape_vec((height, width), values)
            .expect("damage planes have the same shape as the inputs")
            .into_pyarray(py)
    };
    Ok((
        into_numpy(tone),
        into_numpy(lost),
        into_numpy(extra),
        into_numpy(boundary),
    ))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(lightness_damage, module)?)?;
    Ok(())
}
