use serde_json::{Value, json};

use super::texture::{Texture, contiguous_sum, native_sample};
use super::{ImageView, MAX_EVALUATIONS, MAX_ROTATION, MIN_SUPPORT, Pose, linspace};

#[derive(Clone, Copy)]
pub(super) enum Source<'a> {
    Texture(&'a Texture),
    Native(ImageView<'a>),
}

impl Source<'_> {
    fn step(self) -> [f64; 2] {
        match self {
            Self::Texture(texture) => texture.step,
            Self::Native(_) => [1.0; 2],
        }
    }

    #[inline]
    fn sample(self, point: [f64; 2]) -> [f32; 3] {
        match self {
            Self::Texture(texture) => texture.sample(point),
            Self::Native(image) => native_sample(image, point),
        }
    }
}

struct Delivery {
    point: [f64; 2],
    texture: [f32; 3],
    region: usize,
}

#[derive(Clone, Copy)]
struct Observation {
    delivery: [f32; 3],
    reference: [f32; 3],
    region: usize,
}

#[derive(Default)]
pub(super) struct Evidence {
    pub agreement: f64,
    pub support: f64,
    regions: Vec<(usize, usize, f64, f64)>,
}

impl Evidence {
    pub fn objective(&self) -> f64 {
        if self.support < MIN_SUPPORT {
            return -1.0 + self.support;
        }
        self.agreement - 0.16 * (1.0 - self.support)
    }

    pub fn describe(&self) -> Value {
        json!({"agreement": self.agreement, "support_fraction": self.support, "regions": self.regions})
    }

    pub fn strength(&self) -> f64 {
        if self.regions.is_empty() {
            return self.agreement.max(0.0);
        }
        let strongest = self
            .regions
            .iter()
            .map(|&(_, _, _, energy)| energy)
            .fold(0.0, f64::max);
        let mut local: Vec<_> = self
            .regions
            .iter()
            .filter_map(|&(_, _, agreement, energy)| {
                (energy >= 0.09 * strongest).then_some(agreement)
            })
            .collect();
        local.sort_by(f64::total_cmp);
        let midpoint = local.len() / 2;
        let median = if local.len().is_multiple_of(2) {
            (local[midpoint - 1] + local[midpoint]) * 0.5
        } else {
            local[midpoint]
        };
        0.76 * self.agreement + 0.24 * median
    }
}

fn mean(
    samples: &[Observation],
    value: impl Fn(&Observation) -> f32 + Copy,
    contiguous: bool,
) -> f32 {
    let sum = if contiguous {
        contiguous_sum(samples, value)
    } else {
        samples.iter().map(value).sum()
    };
    sum / samples.len() as f32
}

fn agreement(samples: &[Observation], reference_contiguous: bool) -> (f64, f32, f32) {
    let n = samples.len();
    if n < 4 {
        return (0.0, 0.0, 0.0);
    }
    let delivery_mean = [0, 1, 2].map(|c| mean(samples, |s| s.delivery[c], false));
    let reference_mean = [0, 1, 2].map(|c| mean(samples, |s| s.reference[c], reference_contiguous));
    let mut sum_products = 0.0;
    let mut norm_delivery = 0.0;
    let mut norm_reference = 0.0;
    let mut variances = [[0.0f32; 3]; 2];
    for sample in samples {
        for c in 0..3 {
            let a = sample.delivery[c] - delivery_mean[c];
            let b = sample.reference[c] - reference_mean[c];
            let aa = a * a;
            let bb = b * b;
            sum_products += f64::from(a * b);
            norm_delivery += f64::from(aa);
            norm_reference += f64::from(bb);
            variances[0][c] += aa;
            variances[1][c] += bb;
        }
    }
    let denom = (norm_delivery * norm_reference).sqrt();
    let rho = if denom <= n as f64 * 1e-13 {
        0.0
    } else {
        (sum_products / denom).clamp(-1.0, 1.0)
    };
    let sigma = variances.map(|values| (values.into_iter().fold(0.0, f32::max) / n as f32).sqrt());
    (rho, sigma[0], sigma[1])
}

pub(super) struct Sampler<'a> {
    reference: Source<'a>,
    last: [f64; 2],
    delivery: [Vec<Delivery>; 2],
}

impl<'a> Sampler<'a> {
    pub fn new(
        crop: Source<'a>,
        reference: Source<'a>,
        shape: [usize; 2],
        reference_shape: [usize; 2],
        count: usize,
    ) -> Self {
        let last = reference_shape.map(|n| (n - 1) as f64);
        let crop_last = shape.map(|n| (n - 1) as f64);
        let step = crop.step();
        let margin: [f64; 2] =
            std::array::from_fn(|axis| (crop_last[axis] / 5.0).min(step[axis] * 3.0));
        let coord = |axis: usize| {
            linspace(
                margin[axis],
                crop_last[axis] - margin[axis],
                shape[axis].min(count),
            )
            .map(|v| {
                if matches!(crop, Source::Native(_)) {
                    v.round_ties_even()
                } else {
                    v
                }
            })
        };
        let x_positions: Vec<_> = coord(1).collect();
        let mut delivery: [Vec<Delivery>; 2] =
            std::array::from_fn(|_| Vec::with_capacity(count * count / 2));
        for y in coord(0) {
            let ry = ((y * 4.0 / shape[0] as f64) as usize).min(3);
            for &x in &x_positions {
                let rx = ((x * 4.0 / shape[1] as f64) as usize).min(3);
                let point = [y, x];
                delivery[(ry + rx) % 2].push(Delivery {
                    point,
                    texture: crop.sample(point),
                    region: 4 * ry + rx,
                });
            }
        }
        Self {
            reference,
            last,
            delivery,
        }
    }

    fn sample(&self, pose: Pose, subset: usize, scratch: &mut Vec<Observation>) -> f64 {
        let rows = &self.delivery[subset];
        let matrix = pose.matrix();
        scratch.clear();
        for sample in rows {
            let [y, x] = sample.point;
            let y_ref = matrix[0][0] * y + matrix[0][1] * x + pose.y;
            let x_ref = matrix[1][0] * y + matrix[1][1] * x + pose.x;
            if 0.0 <= y_ref && y_ref <= self.last[0] && 0.0 <= x_ref && x_ref <= self.last[1] {
                scratch.push(Observation {
                    delivery: sample.texture,
                    reference: self.reference.sample([y_ref, x_ref]),
                    region: sample.region,
                });
            }
        }
        scratch.len() as f64 / rows.len().max(1) as f64
    }

    pub fn evaluate(&self, pose: Pose, held_out: bool) -> Evidence {
        let mut samples = Vec::new();
        let support = self.sample(pose, usize::from(held_out), &mut samples);
        let global = agreement(&samples, true).0;
        let mut regions = Vec::new();
        if samples.len() >= 4 {
            let mut cells: [Vec<_>; 16] = std::array::from_fn(|_| Vec::new());
            for sample in samples {
                cells[sample.region].push(sample);
            }
            for (region, samples) in cells.iter().enumerate() {
                let (rho, sigma_a, sigma_b) = agreement(samples, false);
                let energy = f64::from((sigma_a * sigma_b).sqrt());
                if energy > 1e-6 {
                    regions.push((region, samples.len(), rho, energy));
                }
            }
        }
        Evidence {
            agreement: global,
            support,
            regions,
        }
    }

    pub fn objective(&self, pose: Pose) -> f64 {
        self.objective_with_scratch(pose, &mut Vec::new())
    }

    fn objective_with_scratch(&self, pose: Pose, scratch: &mut Vec<Observation>) -> f64 {
        let support = self.sample(pose, 0, scratch);
        Evidence {
            agreement: agreement(scratch, true).0,
            support,
            regions: Vec::new(),
        }
        .objective()
    }
}

type Params = [f64; 4];
type Bounds = [(f64, f64); 4];

fn add_scaled(center: Params, direction: Params, factor: f64) -> Params {
    std::array::from_fn(|i| center[i] + factor * direction[i])
}

fn line_bounds(center: Params, direction: Params, bounds: Bounds) -> (f64, f64) {
    let mut start = f64::NEG_INFINITY;
    let mut end = f64::INFINITY;
    for i in 0..4 {
        if direction[i] == 0.0 {
            continue;
        }
        let a = (bounds[i].0 - center[i]) / direction[i];
        let b = (bounds[i].1 - center[i]) / direction[i];
        start = start.max(a.min(b));
        end = end.min(a.max(b));
    }
    if end >= start {
        (start, end)
    } else {
        (0.0, 0.0)
    }
}

struct Budget<F> {
    remaining: usize,
    loss: F,
}

impl<F: FnMut(Params) -> f64> Budget<F> {
    fn evaluate(&mut self, location: Params) -> Option<f64> {
        self.remaining = self.remaining.checked_sub(1)?;
        Some((self.loss)(location))
    }
}

fn signed_step(step: f64) -> f64 {
    if step < 0.0 { -1.0 } else { 1.0 }
}

/// Bounded scalar Brent search for each Powell direction.
fn minimize_on_line(
    start: Params,
    direction: Params,
    bounds: Bounds,
    loss: &mut Budget<impl FnMut(Params) -> f64>,
    prior: f64,
) -> Option<(f64, Params, Params)> {
    if direction == [0.0; 4] {
        return Some((prior, start, direction));
    }
    let (mut lower, mut upper) = line_bounds(start, direction, bounds);
    let golden_ratio = 0.5 * (3.0 - 5.0_f64.sqrt());
    let mut best = lower + golden_ratio * (upper - lower);
    let mut best_value = loss.evaluate(add_scaled(start, direction, best))?;
    let mut earlier = (best, best_value);
    let mut oldest = earlier;
    let mut movement = 0.0_f64;
    let mut previous = 0.0_f64;
    loop {
        let middle = 0.5 * (lower + upper);
        let tolerance = 2.2e-16_f64.sqrt() * best.abs() + 0.006 / 3.0;
        if (best - middle).abs() <= 2.0 * tolerance - 0.5 * (upper - lower) {
            let change = direction.map(|value| best * value);
            return Some((best_value, add_scaled(start, direction, best), change));
        }
        let use_parabola = if previous.abs() > tolerance {
            // Fit through the best iterate and the two retained alternatives.
            let r = (best - earlier.0) * (best_value - oldest.1);
            let q = (best - oldest.0) * (best_value - earlier.1);
            let mut numerator = (best - oldest.0) * q - (best - earlier.0) * r;
            let denominator = 2.0 * (q - r);
            if denominator > 0.0 {
                numerator = -numerator;
            }
            let denominator = denominator.abs();
            let limit = previous;
            previous = movement;
            if numerator.abs() < (0.5 * denominator * limit).abs()
                && numerator > denominator * (lower - best)
                && numerator < denominator * (upper - best)
            {
                movement = (numerator + 0.0) / denominator;
                let point = best + movement;
                if point - lower < 2.0 * tolerance || upper - point < 2.0 * tolerance {
                    movement = tolerance * signed_step(middle - best);
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        if !use_parabola {
            previous = if best >= middle {
                lower - best
            } else {
                upper - best
            };
            movement = golden_ratio * previous;
        }
        let point = best + signed_step(movement) * movement.abs().max(tolerance);
        let value = loss.evaluate(add_scaled(start, direction, point))?;
        if value <= best_value {
            if point >= best {
                lower = best;
            } else {
                upper = best;
            }
            oldest = earlier;
            earlier = (best, best_value);
            best = point;
            best_value = value;
        } else {
            if point < best {
                lower = point;
            } else {
                upper = point;
            }
            if value <= earlier.1 || earlier.0 == best {
                oldest = earlier;
                earlier = (point, value);
            } else if value <= oldest.1 || oldest.0 == best || oldest.0 == earlier.0 {
                oldest = (point, value);
            }
        }
    }
}

fn minimize(initial: Params, bounds: Bounds, loss: impl FnMut(Params) -> f64) -> Params {
    let mut loss = Budget {
        remaining: MAX_EVALUATIONS,
        loss,
    };
    let mut direction: [Params; 4] =
        std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1.0 } else { 0.0 }));
    let mut point = initial;
    let mut previous_point = initial;
    let Some(mut value) = loss.evaluate(initial) else {
        return initial;
    };
    loop {
        let start_value = value;
        let mut max_delta = 0.0;
        let mut biggest_change = 0;
        for (i, &line) in direction.iter().enumerate() {
            let before = value;
            let Some((next_value, next_point, _)) =
                minimize_on_line(point, line, bounds, &mut loss, value)
            else {
                return point;
            };
            (value, point) = (next_value, next_point);
            if before - value > max_delta {
                max_delta = before - value;
                biggest_change = i;
            }
        }
        if 2.0 * (start_value - value) <= 0.00015 * (start_value.abs() + value.abs()) + 1e-20 {
            return point;
        }
        let movement = std::array::from_fn(|i| point[i] - previous_point[i]);
        previous_point = point;
        let upper = line_bounds(point, movement, bounds).1;
        let extrapolated = add_scaled(point, movement, upper.min(1.0));
        let Some(new_value) = loss.evaluate(extrapolated) else {
            return point;
        };
        if start_value > new_value {
            let bend = 2.0 * (start_value + new_value - 2.0 * value);
            let remainder = start_value - value - max_delta;
            let extrapolated_change = start_value - new_value;
            if bend * (remainder * remainder)
                - max_delta * extrapolated_change * extrapolated_change
                < 0.0
            {
                let Some((next_value, next_point, new_direction)) =
                    minimize_on_line(point, movement, bounds, &mut loss, value)
                else {
                    return point;
                };
                (value, point) = (next_value, next_point);
                if new_direction != [0.0; 4] {
                    direction[biggest_change] = direction[3];
                    direction[3] = new_direction;
                }
            }
        }
    }
}

pub(super) fn refine(
    seed: Pose,
    sampler: &Sampler<'_>,
    shape: [usize; 2],
    step: f64,
    scale: (f64, f64),
    wide: bool,
) -> Pose {
    // Center the parameters so that roll and scale need no compensating shift.
    let center = shape.map(|n| (n - 1) as f64 / 2.0);
    let mapped = seed.transform(center);
    let initial = [0.0, 0.0, seed.angle, seed.scale.ln()];
    let movement = (step * if wide { 4.8 } else { 3.0 }).max(1.3);
    let relative = (movement / (shape[0].max(shape[1]) as f64 * seed.scale)).max(0.001);
    let (roll, size) = if wide {
        (2.6, 0.13)
    } else {
        ((relative * 110.0).min(0.9), (relative * 2.0).min(0.035))
    };
    let bounds = [
        (-movement, movement),
        (-movement, movement),
        (
            (seed.angle - roll).max(-MAX_ROTATION),
            (seed.angle + roll).min(MAX_ROTATION),
        ),
        (
            (initial[3] - size).max(scale.0.ln()),
            (initial[3] + size).min(scale.1.ln()),
        ),
    ];
    let transform = |p: Params| {
        let pose = Pose {
            angle: p[2],
            scale: p[3].exp(),
            ..Pose::new(0.0, 0.0)
        };
        let transformed_center = pose.transform(center);
        Pose {
            y: mapped[0] + p[0] - transformed_center[0],
            x: mapped[1] + p[1] - transformed_center[1],
            ..pose
        }
    };
    let mut scratch = Vec::new();
    let continuous = transform(minimize(initial, bounds, |p| {
        -sampler.objective_with_scratch(transform(p), &mut scratch)
    }));
    let integer = Pose::new(
        continuous.y.round_ties_even(),
        continuous.x.round_ties_even(),
    );
    let centered = Pose::new(
        (mapped[0] - center[0]).round_ties_even(),
        (mapped[1] - center[1]).round_ties_even(),
    );
    let mut best = seed;
    let mut best_value = sampler.objective_with_scratch(seed, &mut scratch);
    for pose in [continuous, integer, centered] {
        let objective = sampler.objective_with_scratch(pose, &mut scratch);
        if objective > best_value {
            best = pose;
            best_value = objective;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::minimize;

    #[test]
    fn bounded_directions_find_the_interior_optimum() {
        let target = [0.75, -0.25, -0.125, 0.6];
        let fit = minimize([0.0; 4], [(-1.0, 1.0); 4], |point| {
            point
                .iter()
                .zip(target)
                .map(|(&value, want)| (value - want).powi(2))
                .sum()
        });
        assert!(
            fit.iter()
                .zip(target)
                .all(|(&value, want)| (value - want).abs() < 0.003)
        );
    }
}
