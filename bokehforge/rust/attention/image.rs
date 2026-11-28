use std::f64::consts::PI;
use std::ops::Range;

use rayon::prelude::*;

use crate::parallel;
use crate::srgb::CANONICAL_SRGB;

mod detail;

pub(crate) use detail::detail_light;

const PREVIEW_LONG_EDGE: usize = 640;
const WEIGHT_BITS: u32 = 22;
const WEIGHT_SCALE: i32 = 1 << WEIGHT_BITS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Rgb8 {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
enum Filter {
    Bilinear,
    Lanczos,
}

impl Filter {
    fn support(self) -> f64 {
        match self {
            Self::Bilinear => 1.0,
            Self::Lanczos => 3.0,
        }
    }

    fn evaluate(self, x: f64) -> f64 {
        match self {
            Self::Bilinear => (1.0 - x.abs()).max(0.0),
            Self::Lanczos if (-3.0..3.0).contains(&x) => sinc(x) * sinc(x / 3.0),
            Self::Lanczos => 0.0,
        }
    }
}

fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let x = x * PI;
        x.sin() / x
    }
}

struct Taps<T> {
    start: usize,
    weights: Vec<T>,
}

fn filter_axis<T>(
    source: usize,
    dest: usize,
    filter: Filter,
    convert: impl Fn(f64) -> T,
) -> Vec<Taps<T>> {
    // The resize API uses a float32 source box, then computes tap positions and
    // weights in float64. The filter expands when the axis is downsampled.
    let scale = f64::from(source as f32) / dest as f64;
    let support = filter.support() * scale.max(1.0);
    let filter_step = 1.0 / scale.max(1.0);
    (0..dest)
        .map(|i| {
            let center = (i as f64 + 0.5) * scale;
            let start = (center - support + 0.5) as usize;
            let end = ((center + support + 0.5) as usize).min(source);
            let weights = (start..end)
                .map(|p| filter.evaluate((p as f64 - center + 0.5) * filter_step))
                .collect::<Vec<_>>();
            let total: f64 = weights.iter().sum();
            let divisor = if total != 0.0 { total } else { 1.0 };
            Taps {
                start,
                weights: weights.into_iter().map(|w| convert(w / divisor)).collect(),
            }
        })
        .collect()
}

fn byte_taps(source: usize, dest: usize, filter: Filter) -> Vec<Taps<i32>> {
    filter_axis(source, dest, filter, |w| {
        (w * f64::from(WEIGHT_SCALE)).round() as i32
    })
}

fn round_rgb_sample(sum: i32) -> u8 {
    // Pillow rounds each pass to uint8 before filtering the next axis.
    (sum >> WEIGHT_BITS).clamp(0, 255) as u8
}

fn horizontal_rgb(source: &[u8], dest: &mut [u8], cols: &[Taps<i32>]) {
    let source = source.as_chunks::<3>().0;
    for (pixel, taps) in dest.as_chunks_mut::<3>().0.iter_mut().zip(cols) {
        let mut sums = [WEIGHT_SCALE / 2; 3];
        let samples = &source[taps.start..taps.start + taps.weights.len()];
        for (&[r, g, b], &weight) in samples.iter().zip(&taps.weights) {
            sums[0] += i32::from(r) * weight;
            sums[1] += i32::from(g) * weight;
            sums[2] += i32::from(b) * weight;
        }
        *pixel = sums.map(round_rgb_sample);
    }
}

fn vertical_rgb(image: Rgb8, height: usize, filter: Filter) -> Rgb8 {
    let width = image.width;
    if image.height == height {
        return image;
    }
    let rows = byte_taps(image.height, height, filter);
    let stride = 3 * width;
    let mut pixels = vec![0; stride * height];
    parallel(|| {
        pixels.par_chunks_exact_mut(stride).zip(rows).for_each_init(
            || vec![0; stride],
            |sums, (out, taps)| {
                sums.fill(WEIGHT_SCALE / 2);
                let source = &image.pixels[stride * taps.start..];
                // Traverse each source row contiguously and keep the same
                // tap order for every destination channel.
                for (row, weight) in source.chunks_exact(stride).zip(taps.weights) {
                    for (sum, &value) in sums.iter_mut().zip(row) {
                        *sum += i32::from(value) * weight;
                    }
                }
                for (value, &sum) in out.iter_mut().zip(sums.iter()) {
                    *value = round_rgb_sample(sum);
                }
            },
        );
    });
    Rgb8 {
        width,
        height,
        pixels,
    }
}

pub(super) fn reference_dimensions(width: usize, height: usize) -> [usize; 2] {
    let scale = (PREVIEW_LONG_EDGE as f64 / width.max(height) as f64).min(1.0);
    let out_width = ((width as f64 * scale).round_ties_even() as usize).max(1);
    let out_height = ((height as f64 * scale).round_ties_even() as usize).max(1);
    [out_width, out_height]
}

pub(crate) fn reference_preview(reference: &[f32], width: usize, height: usize) -> Rgb8 {
    assert!(width != 0 && height != 0 && reference.len() == 3 * width * height);
    let [out_width, out_height] = reference_dimensions(width, height);
    let mut pixels = vec![0; 3 * out_width * height];
    let encoder = &*CANONICAL_SRGB;
    parallel(|| {
        let rows = reference
            .par_chunks_exact(3 * width)
            .zip(pixels.par_chunks_exact_mut(3 * out_width));
        if width == out_width {
            rows.for_each(|(input, out)| {
                for (&value, byte) in input.iter().zip(out) {
                    *byte = encoder.encode(value);
                }
            });
        } else {
            let cols = byte_taps(width, out_width, Filter::Lanczos);
            rows.for_each_init(
                || vec![0; 3 * width],
                |encoded, (input, out)| {
                    for (&value, byte) in input.iter().zip(encoded.iter_mut()) {
                        *byte = encoder.encode(value);
                    }
                    horizontal_rgb(encoded, out, &cols);
                },
            );
        }
    });
    vertical_rgb(
        Rgb8 {
            width: out_width,
            height,
            pixels,
        },
        out_height,
        Filter::Lanczos,
    )
}

pub(crate) fn resize_rgb_bilinear(image: &Rgb8, width: usize, height: usize) -> Rgb8 {
    assert!(width != 0 && height != 0);
    let horizontal = if width == image.width {
        image.clone()
    } else {
        let cols = byte_taps(image.width, width, Filter::Bilinear);
        let mut pixels = vec![0; 3 * width * image.height];
        parallel(|| {
            pixels
                .par_chunks_exact_mut(3 * width)
                .zip(image.pixels.par_chunks_exact(3 * image.width))
                .for_each(|(out, source)| horizontal_rgb(source, out, &cols));
        });
        Rgb8 {
            width,
            height: image.height,
            pixels,
        }
    };
    vertical_rgb(horizontal, height, Filter::Bilinear)
}

fn sample_map(pixels: &[f32], stride: usize, weights: &[f64]) -> f32 {
    pixels
        .iter()
        .step_by(stride)
        .zip(weights)
        .fold(0.0, |sum, (&value, &w)| sum + f64::from(value) * w) as f32
}

pub(crate) fn resize_map_bilinear(
    pixels: &[f32],
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> Vec<f32> {
    assert!(width != 0 && height != 0 && out_width != 0 && out_height != 0);
    assert_eq!(pixels.len(), width * height);
    let horizontal = if width == out_width {
        None
    } else {
        let cols = filter_axis(width, out_width, Filter::Bilinear, |w| w);
        let mut intermediate = vec![0.0; out_width * height];
        parallel(|| {
            intermediate
                .par_chunks_exact_mut(out_width)
                .zip(pixels.par_chunks_exact(width))
                .for_each(|(out, input)| {
                    for (value, taps) in out.iter_mut().zip(&cols) {
                        *value = sample_map(&input[taps.start..], 1, &taps.weights);
                    }
                });
        });
        Some(intermediate)
    };
    if height == out_height {
        return horizontal.unwrap_or_else(|| pixels.to_vec());
    }
    let source = horizontal.as_deref().unwrap_or(pixels);
    let rows = filter_axis(height, out_height, Filter::Bilinear, |w| w);
    let mut output = vec![0.0; out_width * out_height];
    parallel(|| {
        output
            .par_chunks_exact_mut(out_width)
            .zip(rows)
            .for_each(|(out, taps)| {
                let input = &source[out_width * taps.start..];
                for (x, value) in out.iter_mut().enumerate() {
                    *value = sample_map(&input[x..], out_width, &taps.weights);
                }
            });
    });
    output
}

pub(crate) fn blend_components([a, b, c, d]: [&[f32]; 4]) -> Vec<f32> {
    assert!(a.len() == b.len() && a.len() == c.len() && a.len() == d.len());
    (0..a.len())
        .map(|i| {
            let product = ((1.0 - 0.5 * a[i]) * (1.0 - 0.5 * b[i])) * (1.0 - 0.5 * c[i]);
            (1.0 - product * (1.0 - 0.5 * d[i])) / (1.0 - 0.5_f32.powi(4))
        })
        .collect()
}

pub(crate) fn mask_heat(masks: &[Vec<bool>], scores: &[f32], detail: &[f32]) -> Vec<f32> {
    assert_eq!(masks.len(), scores.len());
    parallel(|| {
        let mut heat = vec![0.0_f32; detail.len()];
        let group_size = rayon::current_num_threads();
        // Each mask's percentile is independent. Bound the temporary runs
        // to one group, then apply them to the shared heatmap in mask order.
        for (masks, scores) in masks.chunks(group_size).zip(scores.chunks(group_size)) {
            let prepared = masks
                .par_iter()
                .zip(scores)
                .filter_map(|(mask, &score)| prepare_mask_heat(mask, score, detail))
                .collect::<Vec<_>>();
            for (strength, ranges) in prepared {
                for range in ranges {
                    for (pixel, &relevance) in heat[range.clone()].iter_mut().zip(&detail[range]) {
                        *pixel = pixel.max(strength * (0.75 + 0.25 * relevance));
                    }
                }
            }
        }
        heat
    })
}

fn prepare_mask_heat(
    mask: &[bool],
    score: f32,
    detail: &[f32],
) -> Option<(f32, Vec<Range<usize>>)> {
    assert_eq!(mask.len(), detail.len());
    let area = mask.iter().filter(|&&member| member).count();
    let fraction = area as f64 / mask.len() as f64;
    if !(0.001..=0.65).contains(&fraction) {
        return None;
    }
    let mut values = Vec::with_capacity(area);
    let mut ranges = Vec::new();
    let mut start = 0;
    for run in mask.chunk_by(|&a, &b| a == b) {
        let end = start + run.len();
        if run[0] {
            values.extend_from_slice(&detail[start..end]);
            ranges.push(start..end);
        }
        start = end;
    }
    let quartile = f64::from(detail::mask_quantile(&mut values));
    let strength = (f64::from(score).min(1.0) * (0.25 + 0.75 * quartile)) as f32;
    Some((strength, ranges))
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    fn float_digest(pixels: &[f32]) -> String {
        let mut hash = Sha256::new();
        for pixel in pixels {
            hash.update(pixel.to_le_bytes());
        }
        format!("{:x}", hash.finalize())
    }

    #[test]
    fn portrait_preview_and_detail_match_the_reference_profile() {
        let (width, height) = (451, 1097);
        let input = (0..width * height * 3)
            .map(|i| ((i * 23 + i / 3 * 17) % 509) as f32 / 255.0 - 0.2)
            .collect::<Vec<_>>();
        let view = reference_preview(&input, width, height);
        assert_eq!((view.width, view.height), (263, 640));
        assert_eq!(
            format!("{:x}", Sha256::digest(&view.pixels)),
            "1aac71d0dc2c8112843ad40560e2ca2aaf8fd80d67b88473b486ddfe57de2a92"
        );
        assert_eq!(
            float_digest(&detail_light(&view)),
            "6e8795c07fd18aeaf4d6fe0e1e239e1aec47a4089a0c3823b318290ad884943f"
        );
    }

    #[test]
    fn bilinear_matches_pillow_when_axes_scale_in_opposite_directions() {
        let (width, height) = (11, 7);
        let rgb = Rgb8 {
            width,
            height,
            pixels: (0..width * height * 3)
                .map(|i| (i * 73 % 251) as u8)
                .collect(),
        };
        let map = (0..width * height)
            .map(|i| i as f32 / (width * height - 1) as f32)
            .collect::<Vec<_>>();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(resize_rgb_bilinear(&rgb, 23, 3).pixels)
            ),
            "2f225d5883792b0b2b6b278745bf304a999e2302259bb61669e478e8ca2740f9"
        );
        assert_eq!(
            float_digest(&resize_map_bilinear(&map, width, height, 23, 3)),
            "56dd712ed94c65bc61f26eeeeab6f3da942f87cf46a1a3a8b6b50ce8e027ea5a"
        );
    }

    #[test]
    fn mask_and_component_blend_keep_the_reference_float32_order() {
        let count = 77;
        let detail = (0..count)
            .map(|i| i as f32 / (count - 1) as f32)
            .collect::<Vec<_>>();
        let masks = [(2, 4), (3, 5), (5, 1), (7, 16)].map(|(factor, limit)| {
            (0..count)
                .map(|i| i * factor % 17 < limit)
                .collect::<Vec<_>>()
        });
        let heat = mask_heat(&masks, &[0.2, 0.4, 0.7, 0.9], &detail);
        assert_eq!(
            float_digest(&heat),
            "a321ee234f810baa60054281e66571780dad1567ac88de033501a5b0f0968e0a"
        );
        let square = detail.iter().map(|value| value * value).collect::<Vec<_>>();
        let temporal = detail
            .iter()
            .map(|value| (0.19 + value) / 2.0)
            .collect::<Vec<_>>();
        assert_eq!(
            float_digest(&blend_components([&detail, &square, &heat, &temporal])),
            "257fd2e48b93297a0ca2051aab53aabbaa8b44b9cfa56ef57adb3b093e06b0ae"
        );
    }

    #[test]
    fn mask_heat_preserves_coverage_limits_and_group_maxima() {
        let detail = vec![0.5_f32; 1000];
        for population in [0, 1, 650, 651] {
            let mask = (0..detail.len())
                .map(|i| i < population)
                .collect::<Vec<_>>();
            let heat = mask_heat(&[mask], &[0.5], &detail);
            let included = (1..=650).contains(&population);
            assert!(heat.iter().enumerate().all(|(i, &value)| {
                value
                    == if i < population && included {
                        0.2734375
                    } else {
                        0.0
                    }
            }));
        }

        let masks = (0..24)
            .map(|start| {
                (0..detail.len())
                    .map(|i| ((i + start) % 17) < 5)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let scores = (1..=24).map(|i| i as f32 / 24.0).collect::<Vec<_>>();
        let mut expected = vec![0.0_f32; detail.len()];
        for (mask, &score) in masks.iter().zip(&scores) {
            for (value, &included) in expected.iter_mut().zip(mask) {
                if included {
                    *value = value.max((f64::from(score) * 0.625) as f32 * 0.875);
                }
            }
        }
        assert_eq!(mask_heat(&masks, &scores, &detail), expected);
        let reversed_masks = masks.into_iter().rev().collect::<Vec<_>>();
        let reversed_scores = scores.into_iter().rev().collect::<Vec<_>>();
        assert_eq!(
            mask_heat(&reversed_masks, &reversed_scores, &detail),
            expected
        );
    }
}
