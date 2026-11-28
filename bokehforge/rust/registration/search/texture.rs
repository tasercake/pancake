use std::ops::Range;

use rayon::prelude::*;

use super::ImageView;

const LUMA: [f32; 3] = [0.2126, 0.7152, 0.0722];

pub(super) struct Texture {
    pub shape: [usize; 2],
    pub step: [f64; 2],
    pub pixels: Vec<[f32; 3]>,
    pub contrast: f64,
}

impl Texture {
    pub fn new(image: ImageView<'_>, side: usize) -> Self {
        let dimensions = image.shape();
        let decimation = (dimensions[0].max(dimensions[1]) as f64 / side as f64).max(1.0);
        let shape = dimensions.map(|n| ((n as f64 / decimation).round_ties_even() as usize).max(1));
        let step = std::array::from_fn(|axis| dimensions[axis] as f64 / shape[axis] as f64);
        let mut colors = resize(image, shape);
        let color = |rgb: &mut [f32; 3]| {
            let [r, g, b] = *rgb;
            *rgb = [
                r * LUMA[0] + g * LUMA[1] + b * LUMA[2],
                (r - g) * 0.55,
                (b - g) * 0.55,
            ];
        };
        colors.par_iter_mut().for_each(color);
        let contrast = deviation(&colors).into_iter().fold(0.0, f32::max);
        let (mut narrow, broad) = rayon::join(
            || gaussian(&colors, shape, 0.65, 3),
            || gaussian(&colors, shape, 4.0, 16),
        );
        for (a, b) in narrow.iter_mut().zip(broad) {
            for channel in 0..3 {
                a[channel] -= b[channel];
            }
        }
        Self {
            shape,
            step,
            pixels: narrow,
            contrast: f64::from(contrast),
        }
    }

    #[inline]
    pub fn sample(&self, point: [f64; 2]) -> [f32; 3] {
        let y = (point[0] + 0.5) / self.step[0] - 0.5;
        let x = (point[1] + 0.5) / self.step[1] - 0.5;
        let top = y.floor();
        let left = x.floor();
        let y0 = (top as usize).min(self.shape[0] - 1);
        let x0 = (left as usize).min(self.shape[1] - 1);
        let y1 = ((top + 1.0) as usize).min(self.shape[0] - 1);
        let x1 = ((left + 1.0) as usize).min(self.shape[1] - 1);
        let upper_left = self.pixels[y0 * self.shape[1] + x0];
        let upper_right = self.pixels[y0 * self.shape[1] + x1];
        let lower_left = self.pixels[y1 * self.shape[1] + x0];
        let lower_right = self.pixels[y1 * self.shape[1] + x1];
        let wy0 = 1.0 - (y - top);
        let wx0 = 1.0 - (x - left);
        let wy1 = 1.0 - wy0;
        let wx1 = 1.0 - wx0;
        std::array::from_fn(|c| {
            let mut total = 0.0;
            total += (f64::from(upper_left[c]) * wy0) * wx0;
            total += (f64::from(upper_right[c]) * wy0) * wx1;
            total += (f64::from(lower_left[c]) * wy1) * wx0;
            total += (f64::from(lower_right[c]) * wy1) * wx1;
            total as f32
        })
    }
}

#[inline]
pub(super) fn native_sample(image: ImageView<'_>, [y, x]: [f64; 2]) -> [f32; 3] {
    let sample = |y, x| image.sample(y, x).map(|v| f64::from(v.clamp(0.0, 1.0)));
    let center = sample(y, x);
    let above = sample(y - 3.0, x);
    let below = sample(y + 3.0, x);
    let left = sample(y, x - 3.0);
    let right = sample(y, x + 3.0);
    let [r, g, b] =
        std::array::from_fn(|c| center[c] - (above[c] + below[c] + left[c] + right[c]) * 0.25);
    let [lr, lg, lb] = LUMA.map(f64::from);
    [
        (r * lr + g * lg + b * lb) as f32,
        ((r - g) * 0.55) as f32,
        ((b - g) * 0.55) as f32,
    ]
}

/// Pillow's BOX downsample selects source pixel centers in a half-open footprint.
fn box_footprints(input: usize, output: usize) -> Vec<(Range<usize>, f64)> {
    let step = input as f64 / output as f64;
    let scale = step.max(1.0);
    let half = 0.5 * scale;
    let inverse = 1.0 / scale;
    (0..output)
        .map(|index| {
            let center = (index as f64 + 0.5) * step;
            let start = (center - half + 0.5) as usize;
            let end = ((center + half + 0.5) as usize).min(input);
            let included = |i: usize| {
                let distance = (i as f64 - center + 0.5) * inverse;
                distance > -0.5 && distance <= 0.5
            };
            let mut support = (start..end).filter(|&i| included(i));
            let Some(first) = support.next() else {
                return (0..0, 0.0);
            };
            let last = support.next_back().unwrap_or(first);
            (first..last + 1, 1.0 / (last - first + 1) as f64)
        })
        .collect()
}

pub(super) fn resize(image: ImageView<'_>, [height, width]: [usize; 2]) -> Vec<[f32; 3]> {
    let [source_h, source_w] = image.shape();
    let horizontal = box_footprints(source_w, width);
    let mut rows = vec![[0.0f32; 3]; source_h * width];
    rows.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
        let source = image.row(y);
        if width == source_w {
            for (out, rgb) in row.iter_mut().zip(source.as_chunks::<3>().0) {
                *out = rgb.map(|v| v.clamp(0.0, 1.0));
            }
            return;
        }
        for (out, (range, weight)) in row.iter_mut().zip(&horizontal) {
            let mut total = [0.0; 3];
            for pixel in source[range.start * 3..range.end * 3].as_chunks::<3>().0 {
                for c in 0..3 {
                    total[c] += f64::from(pixel[c].clamp(0.0, 1.0)) * weight;
                }
            }
            *out = total.map(|v| v as f32);
        }
    });
    if height == source_h {
        return rows;
    }
    let vertical = box_footprints(source_h, height);
    let mut out = vec![[0.0; 3]; height * width];
    out.par_chunks_mut(width)
        .zip(vertical.into_par_iter())
        .for_each_init(
            || vec![[0.0; 3]; width],
            |total, (row, (range, weight))| {
                total.fill([0.0; 3]);
                for y in range {
                    for (acc, pixel) in total.iter_mut().zip(&rows[y * width..(y + 1) * width]) {
                        for c in 0..3 {
                            acc[c] += f64::from(pixel[c]) * weight;
                        }
                    }
                }
                for (out, total) in row.iter_mut().zip(total.iter()) {
                    *out = total.map(|v| v as f32);
                }
            },
        );
    out
}

#[inline]
fn reflection(index: isize, length: usize) -> usize {
    let length = length as isize;
    let wrapped = index.rem_euclid(2 * length);
    wrapped.min(2 * length - wrapped - 1) as usize
}

fn gaussian(
    pixels: &[[f32; 3]],
    [height, width]: [usize; 2],
    sigma: f64,
    radius: usize,
) -> Vec<[f32; 3]> {
    let weights: Vec<_> = (0..=radius)
        .map(|i| (-0.5 / (sigma * sigma) * (i * i) as f64).exp())
        .collect();
    let sum = weights[0] + 2.0 * weights[1..].iter().sum::<f64>();
    let weights: Vec<_> = weights.into_iter().map(|weight| weight / sum).collect();
    let mut vertical = vec![[0.0; 3]; pixels.len()];
    vertical.par_chunks_mut(width).enumerate().for_each_init(
        || vec![[0.0; 3]; width],
        |totals, (y, row)| {
            for (total, pixel) in totals.iter_mut().zip(&pixels[y * width..(y + 1) * width]) {
                *total = pixel.map(|v| f64::from(v) * weights[0]);
            }
            for offset in (1..=radius).rev() {
                let up = reflection(y as isize - offset as isize, height) * width;
                let down = reflection(y as isize + offset as isize, height) * width;
                for ((acc, a), b) in totals
                    .iter_mut()
                    .zip(&pixels[up..up + width])
                    .zip(&pixels[down..down + width])
                {
                    for c in 0..3 {
                        acc[c] += (f64::from(a[c]) + f64::from(b[c])) * weights[offset];
                    }
                }
            }
            for (out, total) in row.iter_mut().zip(totals.iter()) {
                *out = total.map(|v| v as f32);
            }
        },
    );
    let mut out = vec![[0.0; 3]; pixels.len()];
    out.par_chunks_mut(width)
        .zip(vertical.par_chunks(width))
        .for_each_init(
            || vec![[0.0; 3]; width + 2 * radius],
            |extended, (out, row)| {
                for (x, value) in extended.iter_mut().enumerate() {
                    *value = row[reflection(x as isize - radius as isize, width)].map(f64::from);
                }
                for (x, output) in out.iter_mut().enumerate() {
                    let middle = x + radius;
                    let mut total = extended[middle].map(|v| v * weights[0]);
                    for offset in (1..=radius).rev() {
                        for (c, acc) in total.iter_mut().enumerate() {
                            *acc += (extended[middle - offset][c] + extended[middle + offset][c])
                                * weights[offset];
                        }
                    }
                    *output = total.map(|v| v as f32);
                }
            },
        );
    out
}

/// NumPy uses blocks of at most 128 values with eight independent accumulators
/// for reductions over a contiguous channel plane.
pub(super) fn contiguous_sum<T>(values: &[T], value: impl Fn(&T) -> f32 + Copy) -> f32 {
    let len = values.len();
    if len < 8 {
        return values.iter().fold(-0.0, |sum, item| sum + value(item));
    }
    if len <= 128 {
        let mut lanes: [f32; 8] = std::array::from_fn(|i| value(&values[i]));
        let (blocks, rest) = values[8..].as_chunks::<8>();
        for block in blocks {
            for (lane, item) in lanes.iter_mut().zip(block) {
                *lane += value(item);
            }
        }
        let sum = ((lanes[0] + lanes[1]) + (lanes[2] + lanes[3]))
            + ((lanes[4] + lanes[5]) + (lanes[6] + lanes[7]));
        return rest.iter().fold(sum, |sum, item| sum + value(item));
    }
    let split = (len / 2) & !7;
    contiguous_sum(&values[..split], value) + contiguous_sum(&values[split..], value)
}

pub(super) fn deviation(pixels: &[[f32; 3]]) -> [f32; 3] {
    let count = pixels.len() as f32;
    std::array::from_fn(|c| {
        let mean = contiguous_sum(pixels, |rgb| rgb[c]) / count;
        let energy = contiguous_sum(pixels, |rgb| {
            let centered = rgb[c] - mean;
            centered * centered
        });
        (energy / count).sqrt()
    })
}
