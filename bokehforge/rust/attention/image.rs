use std::f64::consts::PI;

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

fn sample_rgb(pixels: &[u8], stride: usize, weights: &[i32]) -> [u8; 3] {
    // Pillow rounds each pass to uint8 before filtering the next axis.
    let mut accum = [WEIGHT_SCALE / 2; 3];
    for (pixel, &weight) in pixels.chunks(stride).zip(weights) {
        for c in 0..3 {
            accum[c] += i32::from(pixel[c]) * weight;
        }
    }
    accum.map(|v| ((v >> WEIGHT_BITS).clamp(0, 255)) as u8)
}

fn horizontal_rgb(source: &[u8], dest: &mut [u8], cols: &[Taps<i32>]) {
    for (pixel, taps) in dest.chunks_exact_mut(3).zip(cols) {
        pixel.copy_from_slice(&sample_rgb(&source[3 * taps.start..], 3, &taps.weights));
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
        pixels
            .par_chunks_exact_mut(stride)
            .zip(rows)
            .for_each(|(out, taps)| {
                let source = &image.pixels[stride * taps.start..];
                for (x, pixel) in out.chunks_exact_mut(3).enumerate() {
                    pixel.copy_from_slice(&sample_rgb(&source[3 * x..], stride, &taps.weights));
                }
            });
    });
    Rgb8 {
        width,
        height,
        pixels,
    }
}

pub(crate) fn reference_preview(reference: &[f32], width: usize, height: usize) -> Rgb8 {
    assert!(width != 0 && height != 0 && reference.len() == 3 * width * height);
    let scale = (PREVIEW_LONG_EDGE as f64 / width.max(height) as f64).min(1.0);
    let out_width = ((width as f64 * scale).round_ties_even() as usize).max(1);
    let out_height = ((height as f64 * scale).round_ties_even() as usize).max(1);
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

pub(crate) fn accumulate_mask_heat(heat: &mut [f32], mask: &[bool], score: f32, detail: &[f32]) {
    assert!(heat.len() == mask.len() && heat.len() == detail.len());
    let area = mask.iter().filter(|&&member| member).count();
    let fraction = area as f64 / mask.len() as f64;
    if !(0.001..=0.65).contains(&fraction) {
        return;
    }
    let mut values = detail
        .iter()
        .zip(mask)
        .filter_map(|(&value, &member)| member.then_some(value))
        .collect::<Vec<_>>();
    let quartile = f64::from(detail::mask_quantile(&mut values));
    let strength = (f64::from(score).min(1.0) * (0.25 + 0.75 * quartile)) as f32;
    for (pixel, (&relevance, &member)) in heat.iter_mut().zip(detail.iter().zip(mask)) {
        if member {
            *pixel = pixel.max(strength * (0.75 + 0.25 * relevance));
        }
    }
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
        let mut heat = vec![0.0; count];
        for (factor, limit, score) in [(2, 4, 0.2), (3, 5, 0.4), (5, 1, 0.7), (7, 16, 0.9)] {
            let mask = (0..count)
                .map(|i| i * factor % 17 < limit)
                .collect::<Vec<_>>();
            accumulate_mask_heat(&mut heat, &mask, score, &detail);
        }
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
}
