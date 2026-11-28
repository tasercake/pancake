use rayon::prelude::*;

use super::Rgb8;
use crate::parallel;

pub(crate) fn detail_light(image: &Rgb8) -> Vec<f32> {
    let (width, height) = (image.width, image.height);
    assert!(width != 0 && height != 0 && image.pixels.len() == 3 * width * height);
    let mut luma = Vec::with_capacity(width * height);
    let mut saturation = Vec::with_capacity(width * height);
    for row in image.pixels.chunks_exact(3 * width) {
        for (x, &[r, g, b]) in row.as_chunks::<3>().0.iter().enumerate() {
            let [r, g, b] = [r, g, b].map(f32::from);
            luma.push(luminance(r, g, b, x, width) / 255.0);
            let [r, g, b] = [r / 255.0, g / 255.0, b / 255.0];
            saturation.push(r.max(g).max(b) - r.min(g).min(b));
        }
    }

    let scale = width.max(height) as f64 / 640.0;
    let mut residual = gaussian(&luma, width, height, 1.8 * scale);
    subtract_abs(&mut residual, &luma);
    let mut fine = gaussian(&residual, width, height, 6.0 * scale);
    let mut shading = gaussian(&luma, width, height, 8.0 * scale);
    subtract_abs(&mut shading, &gaussian(&luma, width, height, 30.0 * scale));
    let mut contrast = gaussian(&saturation, width, height, 9.0 * scale);
    subtract_abs(
        &mut contrast,
        &gaussian(&saturation, width, height, 36.0 * scale),
    );
    scale_percentiles(&mut fine);
    scale_percentiles(&mut shading);
    scale_percentiles(&mut contrast);
    for ((detail, shade), color) in fine.iter_mut().zip(shading).zip(contrast) {
        *detail = (0.65 * *detail + 0.23 * shade) + 0.12 * color;
    }
    fine
}

#[inline]
fn luminance(r: f32, g: f32, b: f32, x: usize, width: usize) -> f32 {
    const RED: f32 = 0.2126;
    const GREEN: f32 = 0.7152;
    const BLUE: f32 = 0.0722;
    // Match the pinned x86-64 reference float32 matrix-vector product.
    // Its vector, tail, and single-row dot paths round in different orders.
    let vector_end = width & !7;
    let pair_start = width & !3;
    let pair_end = pair_start + (width & 2);
    if x < vector_end {
        b.mul_add(BLUE, g.mul_add(GREEN, r * RED))
    } else if width == 1 {
        (f64::from(r * RED) + f64::from(g * GREEN) + f64::from(b * BLUE)) as f32
    } else if (pair_start..pair_end).contains(&x) {
        (r * RED + g * GREEN) + b * BLUE
    } else {
        b.mul_add(BLUE, r.mul_add(RED, g * GREEN))
    }
}

fn subtract_abs(left: &mut [f32], right: &[f32]) {
    for (value, &other) in left.iter_mut().zip(right) {
        *value = (*value - other).abs();
    }
}

fn gaussian(input: &[f32], width: usize, height: usize, sigma: f64) -> Vec<f32> {
    let radius = (4.0 * sigma + 0.5) as usize;
    if radius == 0 {
        return input.to_vec();
    }
    let multiplier = -0.5 / (sigma * sigma);
    let mut weights = (-(radius as isize)..=radius as isize)
        .map(|x| (multiplier * (x * x) as f64).exp())
        .collect::<Vec<_>>();
    let divisor: f64 = weights.iter().sum();
    weights.truncate(radius + 1);
    for weight in &mut weights {
        *weight /= divisor;
    }

    // SciPy filters the vertical axis first and rounds each pass to float32.
    // The intermediate is transposed so each worker can write a full column.
    let mut vertical = vec![0.0; input.len()];
    let mut output = vec![0.0; input.len()];
    parallel(|| {
        vertical
            .par_chunks_exact_mut(height)
            .enumerate()
            .for_each_init(
                || vec![0.0; height + 2 * radius],
                |line, (x, column)| {
                    fill_reflected(line, height, radius, |y| input[y * width + x]);
                    correlate(line, column, &weights);
                },
            );
        output
            .par_chunks_exact_mut(width)
            .enumerate()
            .for_each_init(
                || vec![0.0; width + 2 * radius],
                |line, (y, row)| {
                    fill_reflected(line, width, radius, |x| vertical[x * height + y]);
                    correlate(line, row, &weights);
                },
            );
    });
    output
}

fn fill_reflected(line: &mut [f64], len: usize, radius: usize, get: impl Fn(usize) -> f32) {
    for (i, slot) in line.iter_mut().enumerate() {
        let coordinate = (i as isize - radius as isize).rem_euclid(2 * len as isize) as usize;
        let reflected = if coordinate < len {
            coordinate
        } else {
            2 * len - 1 - coordinate
        };
        *slot = f64::from(get(reflected));
    }
}

fn correlate(line: &[f64], output: &mut [f32], weights: &[f64]) {
    let radius = weights.len() - 1;
    let (pairs, center) = weights.split_at(radius);
    for (window, out) in line.windows(2 * radius + 1).zip(output) {
        let inner = window[radius] * center[0];
        // Accumulate symmetric pairs from the outermost to the center tap.
        *out = window[..radius]
            .iter()
            .zip(window[radius + 1..].iter().rev())
            .zip(pairs)
            .fold(inner, |sum, ((left, right), weight)| {
                sum + (left + right) * weight
            }) as f32;
    }
}

fn quantile_neighbors(values: &mut [f32], fraction: f64) -> (f32, f32, f64) {
    let position = (values.len() - 1) as f64 * fraction;
    let rank = position.floor() as usize;
    let (_, low, higher) = values.select_nth_unstable_by(rank, f32::total_cmp);
    let mix = position - rank as f64;
    let high = if mix == 0.0 {
        *low
    } else {
        higher.iter().copied().reduce(f32::min).unwrap()
    };
    (*low, high, mix)
}

fn quantile(values: &mut [f32], fraction: f64) -> f64 {
    let (low, high, mix) = quantile_neighbors(values, fraction);
    // NumPy differences inherit the input dtype. Array-valued percentiles
    // promote the subsequent interpolation to float64.
    let range = f64::from(high - low);
    if mix < 0.5 {
        f64::from(low) + range * mix
    } else {
        f64::from(high) - range * (1.0 - mix)
    }
}

pub(super) fn mask_quantile(values: &mut [f32]) -> f32 {
    let (low, high, mix) = quantile_neighbors(values, 0.75);
    let mix = mix as f32;
    let range = high - low;
    // A scalar quartile keeps the interpolation in float32.
    if mix < 0.5 {
        low + range * mix
    } else {
        high - range * (1.0 - mix)
    }
}

fn scale_percentiles(values: &mut [f32]) {
    let mut ranked = values.to_vec();
    let low = quantile(&mut ranked, 0.05);
    let high = quantile(&mut ranked, 0.98);
    let range = (high - low).max(1e-6);
    for value in values {
        *value = ((f64::from(*value) - low) / range).clamp(0.0, 1.0) as f32;
    }
}
