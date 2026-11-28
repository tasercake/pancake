use std::collections::{HashMap, HashSet};

use numpy::{
    IntoPyArray, PyArray2, PyArrayDescrMethods, PyArrayMethods, PyReadonlyArray2,
    PyUntypedArrayMethods, ndarray::Array2,
};
use pyo3::{exceptions::PyValueError, prelude::*};
use rayon::prelude::*;

pub(crate) fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0_f64;
    let mut residual = 0.0;
    for value in values {
        let next = sum + value;
        residual += if sum.abs() >= value.abs() {
            (sum - next) + value
        } else {
            (value - next) + sum
        };
        sum = next;
    }
    if residual.is_finite() && residual != 0.0 {
        sum + residual
    } else {
        sum
    }
}

#[pyfunction]
fn weighted_damage(terms: Vec<(f64, f64)>) -> f64 {
    compensated_sum(terms.into_iter().map(|(weight, burden)| weight * burden))
}

#[pyfunction]
pub(crate) fn placement_score(
    geometry: f64,
    resolution: f64,
    weight: f64,
    quality: f64,
) -> (f64, f64) {
    let coverage = geometry.min(resolution);
    (coverage, weight * coverage * quality)
}

#[pyfunction]
fn placement_total(contributions: Vec<f64>) -> f64 {
    compensated_sum(contributions)
}

#[pyfunction]
pub(crate) fn inspection_gain(
    mean: f64,
    peak: f64,
    max_mix: f64,
    max_multiplier: f64,
) -> Option<(f64, f64)> {
    if mean == 0.0 || peak == mean {
        return None;
    }
    // Ease toward the upper bound with matching slopes at the shoulder.
    let headroom = max_multiplier - 1.0;
    let shoulder = headroom / 2.0;
    let mut peak_boost = max_mix * (peak - mean) / mean;
    if peak_boost > shoulder {
        let excess = peak_boost - shoulder;
        let remaining = headroom - shoulder;
        peak_boost = shoulder + remaining * excess / (remaining + excess);
    }
    let gain = peak_boost / (peak - mean);
    Some((gain, gain * mean))
}

#[pyfunction]
fn inspection_multipliers<'py>(
    py: Python<'py>,
    heatmap: PyReadonlyArray2<'py, f32>,
    mean: f64,
    gain: f64,
) -> PyResult<Bound<'py, PyArray2<f32>>> {
    let sample_bytes = size_of::<f32>() as isize;
    if heatmap.is_empty()
        || !heatmap.data().is_aligned()
        || heatmap.dtype().is_native_byteorder() == Some(false)
        || heatmap
            .shape()
            .iter()
            .zip(heatmap.strides())
            .any(|(&len, &stride)| len > 1 && stride % sample_bytes != 0)
    {
        return Err(PyValueError::new_err(
            "Expected a nonempty, aligned, native float32 heatmap",
        ));
    }
    let heatmap = heatmap.as_array();
    let multiplier = |value| (1.0 + (f64::from(value) - mean) * gain) as f32;
    let weights = py.detach(|| {
        let Some(values) = heatmap
            .as_slice()
            .filter(|values| values.len() >= 1_000_000)
        else {
            return heatmap.mapv(multiplier);
        };
        let weights = crate::parallel(|| values.par_iter().copied().map(multiplier).collect());
        Array2::from_shape_vec(heatmap.raw_dim(), weights).expect("one weight per heatmap pixel")
    });
    Ok(weights.into_pyarray(py))
}

#[derive(Default)]
struct Dataset<'a> {
    id: &'a str,
    groups: Vec<Group<'a>>,
    group_indices: HashMap<&'a str, usize>,
}

struct Group<'a> {
    id: &'a str,
    scores: Vec<f64>,
}

type GroupDetail = (String, f64, f64, usize);
type DatasetDetail = (String, f64, f64, Vec<GroupDetail>);
type CohortScore = (Option<f64>, usize, Vec<DatasetDetail>);

#[pyfunction]
pub(crate) fn cohort_score(
    entries: Vec<(String, String, f64)>,
    unavailable: bool,
    explain: bool,
) -> CohortScore {
    // Keep first-seen order for the report while indexing IDs for aggregation.
    let mut datasets = Vec::<Dataset<'_>>::new();
    let mut dataset_indices = HashMap::new();
    let mut unique_group_ids = HashSet::new();
    for (dataset, group, score) in &entries {
        let dataset_index = *dataset_indices.entry(dataset.as_str()).or_insert_with(|| {
            datasets.push(Dataset {
                id: dataset,
                ..Dataset::default()
            });
            datasets.len() - 1
        });
        let dataset = &mut datasets[dataset_index];
        let group_index = *dataset
            .group_indices
            .entry(group.as_str())
            .or_insert_with(|| {
                dataset.groups.push(Group {
                    id: group,
                    scores: Vec::new(),
                });
                dataset.groups.len() - 1
            });
        dataset.groups[group_index].scores.push(*score);
        unique_group_ids.insert(group.as_str());
    }

    let dataset_count = datasets.len();
    let mut dataset_means = Vec::with_capacity(dataset_count);
    let mut details = Vec::new();
    for dataset in datasets {
        let group_count = dataset.groups.len();
        let group_means: Vec<_> = dataset
            .groups
            .into_iter()
            .map(|group| {
                let count = group.scores.len();
                (
                    group.id,
                    compensated_sum(group.scores) / count as f64,
                    count,
                )
            })
            .collect();
        let mean =
            compensated_sum(group_means.iter().map(|&(_, mean, _)| mean)) / group_count as f64;
        dataset_means.push(mean);
        if explain {
            let groups = group_means
                .into_iter()
                .map(|(id, score, count)| (id.to_owned(), score, 1.0 / group_count as f64, count))
                .collect();
            details.push((
                dataset.id.to_owned(),
                mean,
                1.0 / dataset_count as f64,
                groups,
            ));
        }
    }
    let score = (!unavailable && dataset_count != 0)
        .then(|| compensated_sum(dataset_means) / dataset_count as f64);
    (score, unique_group_ids.len(), details)
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(weighted_damage, module)?)?;
    module.add_function(wrap_pyfunction!(placement_score, module)?)?;
    module.add_function(wrap_pyfunction!(placement_total, module)?)?;
    module.add_function(wrap_pyfunction!(inspection_gain, module)?)?;
    module.add_function(wrap_pyfunction!(inspection_multipliers, module)?)?;
    module.add_function(wrap_pyfunction!(cohort_score, module)?)?;
    Ok(())
}
