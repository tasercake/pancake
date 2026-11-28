use numpy::{
    IntoPyArray, PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray1, PyReadonlyArray2,
    PyUntypedArrayMethods,
};
use pyo3::{exceptions::PyValueError, prelude::*};

use crate::c_slice;

mod overlap;
pub(crate) use overlap::{OverlapDiagnostics, pool_overlap};

type Edges<'py> = (PyReadonlyArray1<'py, isize>, PyReadonlyArray1<'py, isize>);
type GridInput<'py> = (Edges<'py>, Option<PyReadonlyArray2<'py, i32>>, usize);
type Settings = (f32, f64, f64, bool, usize);
type Statistics = (f64, usize, Option<f64>, f64, usize, usize);
type GridOutput<'py> = (
    Statistics,
    Option<Bound<'py, PyArray1<f64>>>,
    Option<Bound<'py, PyArray1<f64>>>,
);

struct Grid<'a> {
    y: &'a [isize],
    x: &'a [isize],
    coverage: Option<&'a [i32]>,
    nominal_pixels: usize,
}

struct PooledGrid {
    stats: Statistics,
    tail: TailEstimate,
    tiles: Option<Vec<f64>>,
    partition_input: Option<Vec<f64>>,
}

struct GridSelection {
    worst: f64,
    worst_index: usize,
    cutoff: f64,
    tail_count: usize,
    eligible_count: usize,
    tiles: Option<Vec<f64>>,
}

impl GridSelection {
    fn top<'a>(&self, eligible: &'a [f64]) -> &'a [f64] {
        &eligible[self.eligible_count - self.tail_count..]
    }
}

struct TailEstimate {
    value: f64,
    error: f64,
}

impl TailEstimate {
    fn lower_bound(&self) -> f64 {
        if self.error == 0.0 {
            self.value
        } else if self.error.is_finite() {
            (self.value - self.error).next_down()
        } else {
            f64::NEG_INFINITY
        }
    }

    fn needs_partition(&self, best_lower_bound: f64) -> bool {
        self.error != 0.0
            && (!self.error.is_finite() || (self.value + self.error).next_up() >= best_lower_bound)
    }
}

fn checked_edges<'a>(
    edges: &'a Edges<'_>,
    height: usize,
    width: usize,
) -> PyResult<(&'a [isize], &'a [isize])> {
    fn valid(edges: &[isize], end: usize) -> bool {
        edges.len() >= 2
            && edges[0] == 0
            && edges[edges.len() - 1] as usize == end
            && edges.windows(2).all(|pair| pair[0] < pair[1])
    }
    let y = c_slice(&edges.0)?;
    let x = c_slice(&edges.1)?;
    if !valid(y, height) || !valid(x, width) {
        return Err(PyValueError::new_err("Invalid tile edges for array shape"));
    }
    Ok((y, x))
}

fn compact_prefix<F>(y: &[isize], x: &[isize], mut update: F) -> Vec<f64>
where
    F: FnMut(usize, &mut [f64]),
{
    let width = x[x.len() - 1] as usize;
    let mut columns = vec![0.0; width];
    let mut integral = vec![0.0; y.len() * x.len()];
    // Match the original vertical cumsum and sample the horizontal cumsum
    // on the tile boundaries, preserving the order of the additions.
    for (row, edges) in integral[x.len()..]
        .chunks_exact_mut(x.len())
        .zip(y.windows(2))
    {
        for index in edges[0] as usize..edges[1] as usize {
            update(index * width, &mut columns);
        }
        let mut total = columns[0];
        let mut start = 1;
        for (output, &edge) in row[1..].iter_mut().zip(&x[1..]) {
            let stop = edge as usize;
            for &value in &columns[start..stop] {
                total += value;
            }
            *output = total;
            start = stop;
        }
    }
    integral
}

fn damage_prefix(
    image: &[f32],
    support: Option<&[u8]>,
    y: &[isize],
    x: &[isize],
    threshold: f32,
) -> (Vec<f64>, usize) {
    let mut affected = 0;
    let integral = if let Some(support) = support {
        compact_prefix(y, x, |start, columns| {
            let stop = start + columns.len();
            for ((column, &value), &valid) in columns
                .iter_mut()
                .zip(&image[start..stop])
                .zip(&support[start..stop])
            {
                let covered = valid != 0;
                *column += f64::from(value) * f64::from(covered as u8);
                affected += ((value >= threshold) & covered) as usize;
            }
        })
    } else {
        compact_prefix(y, x, |start, columns| {
            let stop = start + columns.len();
            for (column, &value) in columns.iter_mut().zip(&image[start..stop]) {
                *column += f64::from(value);
                affected += (value >= threshold) as usize;
            }
        })
    };
    (integral, affected)
}

fn coverage_prefix(mask: &[u8], y: &[isize], x: &[isize]) -> Vec<f64> {
    compact_prefix(y, x, |start, columns| {
        let stop = start + columns.len();
        for (column, &valid) in columns.iter_mut().zip(&mask[start..stop]) {
            *column += f64::from((valid != 0) as u8);
        }
    })
}

fn image_support<'a>(
    support: Option<&'a PyReadonlyArray2<'_, u8>>,
    shape: &[usize],
) -> PyResult<Option<&'a [u8]>> {
    support
        .map(|support| {
            if support.shape() != shape {
                return Err(PyValueError::new_err(
                    "Image and support must have the same shape",
                ));
            }
            c_slice(support)
        })
        .transpose()
}

#[pyfunction]
#[pyo3(signature = (image, support, edges, threshold))]
fn damage_tile_prefix<'py>(
    py: Python<'py>,
    image: PyReadonlyArray2<'py, f32>,
    support: Option<PyReadonlyArray2<'py, u8>>,
    edges: Edges<'py>,
    threshold: f32,
) -> PyResult<(Bound<'py, PyArray2<f64>>, usize)> {
    let (y, x) = checked_edges(&edges, image.shape()[0], image.shape()[1])?;
    let support = image_support(support.as_ref(), image.shape())?;
    let image = c_slice(&image)?;
    let (integral, affected) = py.detach(|| damage_prefix(image, support, y, x, threshold));
    Ok((
        integral.into_pyarray(py).reshape([y.len(), x.len()])?,
        affected,
    ))
}

#[pyfunction]
fn coverage_tile_prefix<'py>(
    py: Python<'py>,
    mask: PyReadonlyArray2<'py, u8>,
    edges: Edges<'py>,
) -> PyResult<Bound<'py, PyArray2<f64>>> {
    let (y, x) = checked_edges(&edges, mask.shape()[0], mask.shape()[1])?;
    let mask = c_slice(&mask)?;
    let integral = py.detach(|| coverage_prefix(mask, y, x));
    integral.into_pyarray(py).reshape([y.len(), x.len()])
}

// NumPy reduces a contiguous float64 block in groups of eight, with
// recursive pairwise decomposition for longer blocks.
fn pairwise_sum(values: &[f32]) -> f64 {
    match values.len() {
        0..8 => values
            .iter()
            .fold(-0.0, |sum, &value| sum + f64::from(value)),
        8..=128 => {
            let (chunks, rest) = values.as_chunks::<8>();
            let mut lanes = chunks[0].map(f64::from);
            for chunk in &chunks[1..] {
                for (sum, &value) in lanes.iter_mut().zip(chunk) {
                    *sum += f64::from(value);
                }
            }
            let [a, b, c, d, e, f, g, h] = lanes;
            rest.iter()
                .fold(((a + b) + (c + d)) + ((e + f) + (g + h)), |sum, &value| {
                    sum + f64::from(value)
                })
        }
        size => {
            let (head, tail) = values.split_at((size / 2) & !7);
            pairwise_sum(head) + pairwise_sum(tail)
        }
    }
}

fn damage_sum(image: &[f32], support: Option<&[u8]>, buffer_size: usize) -> f64 {
    let mut total = 0.0;
    // Converting float32 to float64 uses buffered chunks. The masked loop
    // reduces each contiguous run of valid pixels within such a chunk.
    if let Some(support) = support {
        for (values, mask) in image.chunks(buffer_size).zip(support.chunks(buffer_size)) {
            let mut start = 0;
            while start < mask.len() {
                let skipped = mask[start..]
                    .iter()
                    .take_while(|&&value| value == 0)
                    .count();
                start += skipped;
                let covered = mask[start..]
                    .iter()
                    .take_while(|&&value| value != 0)
                    .count();
                if covered > 0 {
                    total += pairwise_sum(&values[start..start + covered]);
                    start += covered;
                }
            }
        }
    } else {
        for values in image.chunks(buffer_size) {
            total += pairwise_sum(values);
        }
    }
    total
}

fn tail_tile_count(fraction: f64, eligible_count: usize) -> usize {
    ((fraction * eligible_count as f64).ceil() as usize).max(1)
}

impl Grid<'_> {
    fn totals<'a>(&'a self, integral: &'a [f64], stride: usize) -> impl Iterator<Item = f64> + 'a {
        self.y.windows(2).flat_map(move |y| {
            let top = &integral[y[0] as usize * stride..][..stride];
            let bottom = &integral[y[1] as usize * stride..][..stride];
            self.x.windows(2).map(move |x| {
                let (left, right) = (x[0] as usize, x[1] as usize);
                bottom[right] - top[right] - bottom[left] + top[left]
            })
        })
    }

    fn values<'a>(
        &'a self,
        integral: &'a [f64],
        stride: usize,
        damage_max: f64,
    ) -> impl Iterator<Item = f64> + 'a {
        // A short edge tile still uses the full nominal area.
        let denominator = self.nominal_pixels as f64;
        self.totals(integral, stride)
            .map(move |total| (total / denominator).clamp(0.0, damage_max))
    }

    fn contains(&self, index: usize) -> bool {
        self.coverage.is_none_or(|coverage| coverage[index] > 0)
    }

    fn select(
        &self,
        integral: &[f64],
        stride: usize,
        damage_max: f64,
        fraction: f64,
        keep_tiles: bool,
        eligible: &mut Vec<f64>,
    ) -> Option<GridSelection> {
        let size = (self.y.len() - 1) * (self.x.len() - 1);
        let mut tiles = keep_tiles.then(|| Vec::with_capacity(size));
        let mut worst = f64::NEG_INFINITY;
        let mut worst_index = 0;
        eligible.clear();
        for (index, value) in self.values(integral, stride, damage_max).enumerate() {
            if value > worst || (value.is_nan() && !worst.is_nan()) {
                worst = value;
                worst_index = index;
            }
            if self.contains(index) {
                eligible.push(value);
            }
            if let Some(tiles) = &mut tiles {
                tiles.push(value);
            }
        }
        let eligible_count = eligible.len();
        if eligible_count == 0 {
            return None;
        }
        let count = tail_tile_count(fraction, eligible_count);
        let cutoff_index = eligible_count - count;
        eligible.select_nth_unstable_by(cutoff_index, f64::total_cmp);
        Some(GridSelection {
            worst,
            worst_index,
            cutoff: eligible[cutoff_index],
            tail_count: count,
            eligible_count,
            tiles,
        })
    }
}

// Nonnegative binary64 values share the resolution of their lowest set bit.
// If the total still fits at that resolution, every partial sum is exact
// and changing the partition order cannot affect the result.
fn tail_estimate(values: &[f64]) -> TailEstimate {
    let mut sum = 0.0;
    let mut unit = i32::MAX;
    for &value in values {
        if !value.is_finite() {
            return TailEstimate {
                value,
                error: f64::INFINITY,
            };
        }
        if value != 0.0 {
            let bits = value.to_bits();
            let exponent = ((bits >> 52) & 0x7ff) as i32;
            let significand = (bits & ((1 << 52) - 1)) | (u64::from(exponent > 0) << 52);
            unit = unit.min(exponent.max(1) - 52 + significand.trailing_zeros() as i32);
        }
        sum += value;
    }
    let sum_exponent = ((sum.to_bits() >> 52) & 0x7ff) as i32;
    let count = values.len() as f64;
    let value = sum / count;
    let error = if sum.is_finite() && (sum == 0.0 || sum_exponent - unit < 53) {
        0.0
    } else {
        // Both sums contain the same nonnegative values. In the worst
        // addition order each term sees at most count-1 rounded additions,
        // then one division. Bound the difference between both orders.
        let relative = 8.0 * count * f64::EPSILON;
        if sum.is_finite() && relative < 1.0 {
            (value * relative).max(f64::MIN_POSITIVE)
        } else {
            f64::INFINITY
        }
    };
    TailEstimate { value, error }
}

fn pool_grids(
    grids: &[Grid<'_>],
    integral: &[f64],
    stride: usize,
    damage_max: f64,
    fraction: f64,
    keep_tiles: bool,
) -> Option<Vec<PooledGrid>> {
    let mut scratch = Vec::new();
    let mut pooled = grids
        .iter()
        .map(|grid| {
            let selected = grid.select(
                integral,
                stride,
                damage_max,
                fraction,
                keep_tiles,
                &mut scratch,
            )?;
            Some(PooledGrid {
                stats: (
                    selected.worst,
                    selected.worst_index,
                    None,
                    selected.cutoff,
                    selected.tail_count,
                    selected.eligible_count,
                ),
                tail: tail_estimate(selected.top(&scratch)),
                tiles: selected.tiles,
                partition_input: None,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let best_lower_bound = pooled
        .iter()
        .map(|grid| grid.tail.lower_bound())
        .fold(0.0, f64::max);
    for (grid, result) in grids.iter().zip(&mut pooled) {
        if result.tail.needs_partition(best_lower_bound) {
            if !keep_tiles {
                result.partition_input = Some(
                    grid.values(integral, stride, damage_max)
                        .enumerate()
                        .filter_map(|(index, value)| grid.contains(index).then_some(value))
                        .collect(),
                );
            }
        } else {
            // A grid whose upper bound is below the best lower bound
            // cannot supply the reported tail, so its estimate suffices.
            result.stats.2 = Some(result.tail.value);
        }
    }
    Some(pooled)
}

#[pyfunction]
#[pyo3(signature = (image, support, edges, grids, settings))]
fn pool_tiles<'py>(
    py: Python<'py>,
    image: PyReadonlyArray2<'py, f32>,
    support: Option<PyReadonlyArray2<'py, u8>>,
    edges: Edges<'py>,
    grids: Vec<GridInput<'py>>,
    settings: Settings,
) -> PyResult<(f64, usize, Vec<GridOutput<'py>>)> {
    let (threshold, damage_max, fraction, keep_tiles, buffer_size) = settings;
    if !(damage_max.is_finite()
        && damage_max >= 0.0
        && fraction > 0.0
        && fraction <= 1.0
        && buffer_size > 0)
    {
        return Err(PyValueError::new_err("Invalid damage pooling settings"));
    }
    let (y, x) = checked_edges(&edges, image.shape()[0], image.shape()[1])?;
    let support = image_support(support.as_ref(), image.shape())?;
    let image = c_slice(&image)?;
    let grids = grids
        .iter()
        .map(|(edges, coverage, nominal_pixels)| {
            let (y, x) = checked_edges(edges, y.len() - 1, x.len() - 1)?;
            if *nominal_pixels == 0
                || coverage
                    .as_ref()
                    .is_some_and(|values| values.shape() != [y.len() - 1, x.len() - 1])
            {
                return Err(PyValueError::new_err("Tile coverage must match the grid"));
            }
            Ok(Grid {
                y,
                x,
                coverage: coverage.as_ref().map(c_slice).transpose()?,
                nominal_pixels: *nominal_pixels,
            })
        })
        .collect::<PyResult<Vec<_>>>()?;

    let (sum, affected, pooled) = py.detach(|| {
        let (integral, affected) = damage_prefix(image, support, y, x, threshold);
        let sum = damage_sum(image, support, buffer_size);
        let pooled = pool_grids(&grids, &integral, x.len(), damage_max, fraction, keep_tiles);
        (sum, affected, pooled)
    });
    let pooled = pooled.ok_or_else(|| PyValueError::new_err("Grid must contain a covered tile"))?;
    Ok((
        sum,
        affected,
        pooled
            .into_iter()
            .map(|grid| {
                (
                    grid.stats,
                    grid.tiles.map(|values| values.into_pyarray(py)),
                    grid.partition_input.map(|values| values.into_pyarray(py)),
                )
            })
            .collect(),
    ))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(damage_tile_prefix, module)?)?;
    module.add_function(wrap_pyfunction!(coverage_tile_prefix, module)?)?;
    module.add_function(wrap_pyfunction!(pool_tiles, module)?)?;
    Ok(())
}
