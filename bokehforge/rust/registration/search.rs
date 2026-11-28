use rayon::prelude::*;
use serde_json::{Map, Value, json};

use super::{ImageView, MAX_PLACEMENTS, Pose, append_stage, displacement, linspace, same_uniform};

mod correlation;
mod fit;
mod texture;

use fit::{Evidence, Sampler, Source, refine};
use texture::{Texture, contiguous_sum, resize};

const COARSE_SIDE: usize = 160;
const FINE_SIDE: usize = 960;
const BROAD_SIDE: usize = 96;
const MAX_ROTATION: f64 = 5.0;
const MAX_SEEDS: usize = 8;
const MAX_EVALUATIONS: usize = 155;
const MIN_SUPPORT: f64 = 0.32;
const AMBIGUITY_MARGIN: f64 = 0.037;
const ANGLES: [f64; 5] = [-MAX_ROTATION, -2.5, 0.0, 2.5, MAX_ROTATION];

pub(super) struct SearchResult {
    pub hypotheses: Vec<(Pose, f64, Value)>,
    pub confidence: f64,
    pub reason: &'static str,
}

impl SearchResult {
    fn inferred(pose: Pose, confidence: f64, reason: &'static str) -> Self {
        Self {
            hypotheses: vec![(pose, 1.0, json!({"source": reason}))],
            confidence,
            reason,
        }
    }
}

fn largest(values: [f64; 2]) -> f64 {
    values[0].max(values[1])
}

fn distinct(
    seeds: impl IntoIterator<Item = Pose>,
    shape: [usize; 2],
    distance: f64,
    limit: usize,
) -> Vec<Pose> {
    let mut result = Vec::new();
    for seed in seeds {
        if result
            .iter()
            .all(|&other| displacement(seed, other, shape) > distance)
        {
            result.push(seed);
            if result.len() == limit {
                break;
            }
        }
    }
    result
}

fn rank_by_fit(seeds: impl IntoIterator<Item = Pose>, sampler: &Sampler<'_>) -> Vec<Pose> {
    let mut scored: Vec<_> = seeds
        .into_iter()
        .map(|pose| (sampler.objective(pose), pose))
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    scored.into_iter().map(|(_, pose)| pose).collect()
}

struct SearchScales {
    hypotheses: Vec<f64>,
    limits: (f64, f64),
}

fn scales(ratio: [f64; 2]) -> SearchScales {
    let smaller = ratio[0].min(ratio[1]);
    let lower = (smaller * 0.75).clamp(0.2, 0.35);
    let upper = (smaller * 1.08).clamp(2.5, 8.0);
    let count = 1 + ((upper / lower).ln() / 0.095).ceil() as usize;
    let mut values = vec![
        lower,
        upper,
        1.0,
        ratio[0].clamp(lower, upper),
        ratio[1].clamp(lower, upper),
    ];
    values.extend(
        linspace(lower.log10(), upper.log10(), count)
            .skip(1)
            .take(count - 2)
            .map(|exponent| 10.0_f64.powf(exponent)),
    );
    values.sort_by(f64::total_cmp);
    values.dedup();
    SearchScales {
        hypotheses: values,
        limits: (lower, upper),
    }
}

struct Comparison {
    pose: Pose,
    rank: f64,
    strength: f64,
    fit: Evidence,
    held_out: Evidence,
    broad_fit: Evidence,
    broad_held_out: Evidence,
}

fn rank(pose: Pose, sampler: &Sampler<'_>, broad: &Sampler<'_>, shape: [usize; 2]) -> Comparison {
    let fit = sampler.evaluate(pose, false);
    let held_out = sampler.evaluate(pose, true);
    let broad_fit = broad.evaluate(pose, false);
    let broad_held_out = broad.evaluate(pose, true);
    let fine = fit.strength().min(held_out.strength());
    let broad = broad_fit.strength().min(broad_held_out.strength());
    let strength = fine.max(broad);
    let support = fit
        .support
        .min(held_out.support)
        .min(broad_fit.support)
        .min(broad_held_out.support);
    let native = Pose::new(pose.y.round_ties_even(), pose.x.round_ties_even());
    let complexity = (displacement(pose, native, shape) / 1.8).min(1.0);
    let joint = 0.60 * strength + 0.40 * fine.min(broad);
    let rank = if support >= MIN_SUPPORT {
        joint - 0.11 * (1.0 - support) - 0.035 * complexity
    } else {
        -1.0
    };
    Comparison {
        pose,
        rank,
        strength,
        fit,
        held_out,
        broad_fit,
        broad_held_out,
    }
}

fn compare(
    tried: Vec<Pose>,
    sampler: &Sampler<'_>,
    broad: &Sampler<'_>,
    shape: [usize; 2],
    reference_shape: [usize; 2],
    origin: [f64; 2],
    trace: &mut Map<String, Value>,
) -> SearchResult {
    let mut unique = Vec::with_capacity(tried.len());
    for pose in tried {
        if !unique.contains(&pose) {
            unique.push(pose);
        }
    }
    let mut comparisons: Vec<_> = unique
        .into_par_iter()
        .map(|pose| rank(pose, sampler, broad, shape))
        .collect();
    comparisons.sort_by(|a, b| b.rank.total_cmp(&a.rank));
    let (best_rank, strength) = comparisons
        .first()
        .map_or((-1.0, 0.0), |v| (v.rank, v.strength));
    let point = ((strength - 0.23) / (0.71 - 0.23)).clamp(0.0, 1.0);
    let confidence = if best_rank > -1.0 {
        point * point * (3.0 - 2.0 * point)
    } else {
        0.0
    };
    let cluster_distance = (reference_shape[0].max(reference_shape[1]) as f64 * 0.004).max(2.25);
    let mut hypotheses = Vec::new();
    let mut candidates = Vec::new();
    for row in comparisons {
        let decision = if confidence == 0.0 {
            "insufficient_distributed_agreement"
        } else if best_rank - row.rank >= AMBIGUITY_MARGIN {
            "weaker_alignment"
        } else if hypotheses
            .iter()
            .any(|&(pose, _, _)| displacement(pose, row.pose, shape) <= cluster_distance)
        {
            "same_placement"
        } else if hypotheses.len() == MAX_PLACEMENTS {
            "placement_limit"
        } else {
            let weight = (1.0 - (best_rank - row.rank) / AMBIGUITY_MARGIN).powi(2);
            hypotheses.push((
                row.pose,
                weight,
                json!({
                    "source": "rgb_similarity_search", "candidate_index": candidates.len()
                }),
            ));
            "selected"
        };
        candidates.push(json!({
            "transform": row.pose.move_origin(origin.map(|v| -v)).describe(),
            "fit": row.fit.describe(), "held_out": row.held_out.describe(),
            "broad_fit": row.broad_fit.describe(), "broad_held_out": row.broad_held_out.describe(),
            "strength": row.strength, "rank": row.rank, "decision": decision,
        }));
    }
    trace.insert("candidates".into(), Value::Array(candidates));
    let reason = if hypotheses.is_empty() {
        "weak_image_agreement"
    } else {
        "distributed_rgb_agreement"
    };
    SearchResult {
        hypotheses,
        confidence,
        reason,
    }
}

struct CoarseFit {
    seeds: Vec<Pose>,
    step: f64,
}

fn coarse_fit(
    [crop, target]: [&Texture; 2],
    shapes: [[usize; 2]; 2],
    native: Pose,
    centered: Pose,
    origin: [f64; 2],
    trace: &mut Map<String, Value>,
    scales: &SearchScales,
) -> CoarseFit {
    let [shape, reference_shape] = shapes;
    let correlation::CoarseSearch {
        mut peaks,
        attempted,
        anchors,
    } = correlation::candidates(crop, target, shape, &scales.hypotheses, &ANGLES);
    let sampler = Sampler::new(
        Source::Texture(crop),
        Source::Texture(target),
        shape,
        reference_shape,
        28,
    );
    let ratio = std::array::from_fn::<_, 2, _>(|i| reference_shape[i] as f64 / shape[i] as f64);
    let mut starts = vec![native, centered, Pose::new(0.0, 0.0)];
    for scale in [ratio[0].min(ratio[1]), ratio[0].max(ratio[1])] {
        if scales.limits.0 <= scale && scale <= scales.limits.1 {
            let offset = |axis: usize| {
                ((reference_shape[axis] - 1) as f64 - scale * (shape[axis] - 1) as f64) / 2.0
            };
            starts.push(Pose {
                scale,
                ..Pose::new(offset(0), offset(1))
            });
        }
    }
    let peak_count = peaks.len();
    peaks.sort_by(|a, b| b.0.total_cmp(&a.0));
    starts.extend(peaks.into_iter().take(96).map(|(_, pose)| pose));
    let step = largest(target.step);
    let starts = distinct(starts, shape, (step * 1.2).max(2.0), 76);
    let mut seeds = distinct(
        rank_by_fit(starts.iter().copied(), &sampler),
        shape,
        (step * 2.5).max(3.0),
        MAX_SEEDS,
    );
    if seeds
        .iter()
        .all(|&pose| displacement(centered, pose, shape) > step)
    {
        seeds.pop();
        seeds.push(centered);
    }
    let transforms: Vec<_> = seeds
        .iter()
        .map(|pose| pose.move_origin(origin.map(|v| -v)).describe())
        .collect();
    let anchors: Vec<_> = anchors
        .into_iter()
        .map(|p| [p[0] + origin[0], p[1] + origin[1]])
        .collect();
    append_stage(
        trace,
        json!({
            "method": "rgb_patch_correlation", "reference_size_wh": [target.shape[1], target.shape[0]],
            "scale_hypotheses": scales.hypotheses, "angles_degrees": ANGLES, "anchor_yx": anchors,
            "templates": attempted, "peaks": peak_count, "ranked_starts": starts.len(),
            "refined_starts": seeds.len(), "seed_transforms": transforms,
        }),
    );
    let fitted: Vec<_> = seeds
        .into_par_iter()
        .map(|pose| refine(pose, &sampler, shape, step, scales.limits, true))
        .collect();
    let seeds = distinct(rank_by_fit(fitted, &sampler), shape, step.max(1.8), 5);
    CoarseFit { seeds, step }
}

fn best_integer(seed: Pose, sampler: &Sampler<'_>, reach: i32) -> Pose {
    let [y, x] = [seed.y.round_ties_even(), seed.x.round_ties_even()];
    let mut best = (f64::NEG_INFINITY, seed);
    for dy in -reach..=reach {
        for dx in -reach..=reach {
            let pose = Pose::new(y + f64::from(dy), x + f64::from(dx));
            let merit = sampler.objective(pose);
            if merit > best.0 {
                best = (merit, pose);
            }
        }
    }
    best.1
}

fn appearance_delta(crop: ImageView<'_>, target: ImageView<'_>) -> f64 {
    let side = crop.h.min(crop.w).min(target.h).min(target.w).min(12);
    let (a, b) = rayon::join(|| resize(crop, [side; 2]), || resize(target, [side; 2]));
    let mut delta = Vec::with_capacity(a.len() * 3);
    for c in 0..3 {
        delta.extend(a.iter().zip(&b).map(|(p, q)| (p[c] - q[c]).abs()));
    }
    f64::from(contiguous_sum(&delta, |v| *v) / delta.len() as f32)
}

pub(super) fn approximate(
    crop: ImageView<'_>,
    target: ImageView<'_>,
    trace: &mut Map<String, Value>,
    native: Pose,
    origin: [f64; 2],
) -> SearchResult {
    crate::parallel(|| approximate_in_pool(crop, target, trace, native, origin))
}

fn approximate_in_pool(
    crop: ImageView<'_>,
    target: ImageView<'_>,
    trace: &mut Map<String, Value>,
    native: Pose,
    origin: [f64; 2],
) -> SearchResult {
    let shape = crop.shape();
    let reference_shape = target.shape();
    let centered = Pose::new(
        ((target.h as f64 - crop.h as f64) * 0.5).floor(),
        ((target.w as f64 - crop.w as f64) * 0.5).floor(),
    );
    let pair = |side| rayon::join(|| Texture::new(crop, side), || Texture::new(target, side));
    let (coarse_crop, coarse_target) = pair(COARSE_SIDE);
    let contrast = coarse_crop.contrast.max(coarse_target.contrast);
    trace.insert(
        "coarse_rgb_contrast".into(),
        json!({"delivery": coarse_crop.contrast, "reference": coarse_target.contrast}),
    );
    if contrast < 0.006
        || crop.h.min(crop.w).min(target.h).min(target.w) < 3
        || (coarse_target.contrast < 1e-6 && same_uniform(target))
    {
        append_stage(
            trace,
            json!({"method": "rgb_similarity_search", "result": "insufficient_spatial_structure"}),
        );
        return SearchResult::inferred(native, 1.0, "low_information_native_fallback");
    }
    let scales = scales(std::array::from_fn(|i| {
        reference_shape[i] as f64 / shape[i] as f64
    }));
    let (coarse, ((fine_crop, fine_target), (broad_crop, broad_target))) = rayon::join(
        || {
            coarse_fit(
                [&coarse_crop, &coarse_target],
                [shape, reference_shape],
                native,
                centered,
                origin,
                trace,
                &scales,
            )
        },
        || rayon::join(|| pair(FINE_SIDE), || pair(BROAD_SIDE)),
    );
    let fine = Sampler::new(
        Source::Texture(&fine_crop),
        Source::Texture(&fine_target),
        shape,
        reference_shape,
        44,
    );
    let broad = Sampler::new(
        Source::Texture(&broad_crop),
        Source::Texture(&broad_target),
        shape,
        reference_shape,
        28,
    );
    let step = largest(fine_target.step);
    let fits: Vec<_> = coarse
        .seeds
        .par_iter()
        .map(|&pose| {
            refine(
                pose,
                &fine,
                shape,
                step.max(coarse.step * 0.55),
                scales.limits,
                false,
            )
        })
        .collect();
    append_stage(
        trace,
        json!({
            "method": "coarse_to_fine_fit",
            "fine_reference_size_wh": [fine_target.shape[1], fine_target.shape[0]],
            "broad_reference_size_wh": [broad_target.shape[1], broad_target.shape[0]],
            "fine_starts": fits.len(), "max_objective_evaluations_per_start": MAX_EVALUATIONS,
            "fit_regions": "even checkerboard cells of a 4 by 4 delivery grid",
            "held_out_regions": "odd checkerboard cells of the same grid",
            "regional_columns": ["cell_index", "supporting_samples", "agreement", "shared_texture_energy"],
            "feature_planes": ["local_luma", "local_red_minus_green", "local_blue_minus_green"],
        }),
    );
    let mut tried = vec![native, centered];
    for &pose in &fits {
        tried.push(Pose::new(
            pose.y.round_ties_even(),
            pose.x.round_ties_even(),
        ));
        tried.push(pose);
    }
    let result = if step.max(largest(fine_crop.step)) > 1.05 {
        let native = Sampler::new(
            Source::Native(crop),
            Source::Native(target),
            shape,
            reference_shape,
            42,
        );
        let strongest = rank_by_fit(fits, &fine);
        let strongest = &strongest[..strongest.len().min(3)];
        let reach = ((step / 2.0).ceil() as i32).clamp(2, 5);
        let alternatives: Vec<_> = strongest
            .par_iter()
            .map(|&seed| {
                [
                    best_integer(seed, &native, reach),
                    refine(seed, &native, shape, step / 2.0, scales.limits, false),
                ]
            })
            .collect();
        tried.extend(alternatives.into_iter().flatten());
        append_stage(
            trace,
            json!({
                "method": "native_sample_refinement", "refined_starts": strongest.len(),
                "integer_search_radius": reach, "samples_per_axis_limit": 42,
                "max_objective_evaluations_per_start": MAX_EVALUATIONS,
            }),
        );
        compare(
            tried,
            &native,
            &broad,
            shape,
            reference_shape,
            origin,
            trace,
        )
    } else {
        compare(tried, &fine, &broad, shape, reference_shape, origin, trace)
    };
    if !result.hypotheses.is_empty() {
        return result;
    }
    // A near-full field with weak texture can still support a centered placement.
    let near = (0..2).all(|i| {
        shape[i].abs_diff(reference_shape[i]) as f64 <= (reference_shape[i] as f64 * 0.05).max(3.0)
    });
    if near && contrast < 0.045 {
        let difference = appearance_delta(crop, target);
        trace.insert("low_information_appearance_delta".into(), json!(difference));
        let fallback = (1.0 - difference / 0.05).clamp(0.0, 1.0);
        if fallback > 0.0 {
            return SearchResult::inferred(
                centered,
                fallback,
                "near_native_low_information_fallback",
            );
        }
    }
    result
}
