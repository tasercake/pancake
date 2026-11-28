use std::path::Path;

use ort::{session::Session, value::Tensor};
use rayon::prelude::*;

use super::{Backend, image::Rgb8, load_session};

const ENCODER_SIDE: usize = 1024;
const MASK_SIDE: usize = 256;
const POINTS_PER_SIDE: usize = 16;
const POINTS_PER_BATCH: usize = 32;
const MASKS_PER_POINT: usize = 3;

pub(super) struct Sam2 {
    encoder: Session,
    decoder: Session,
}

#[derive(Clone, Copy)]
struct AxisSample {
    low: usize,
    high: usize,
    fraction: f32,
}

fn sample_axis(source: usize, output: usize) -> Vec<AxisSample> {
    let scale = source as f32 / output as f32;
    (0..output)
        .map(|pixel| {
            let center = scale.mul_add(pixel as f32 + 0.5, -0.5).max(0.0);
            let low = center as usize;
            AxisSample {
                low,
                high: (low + 1).min(source - 1),
                fraction: center - low as f32,
            }
        })
        .collect()
}

fn interpolate(sample: AxisSample, low: f32, high: f32) -> f32 {
    (1.0 - sample.fraction) * low + sample.fraction * high
}

fn preprocess(image: &Rgb8) -> Vec<f32> {
    let horizontal = sample_axis(image.width, ENCODER_SIDE);
    let vertical = sample_axis(image.height, ENCODER_SIDE);
    let mut pixels = vec![0.0; 3 * ENCODER_SIDE * ENCODER_SIDE];
    let mean = [0.485_f32, 0.456, 0.406];
    let std = [0.229_f32, 0.224, 0.225];

    // The preview is at most 640 pixels on either side, so the encoder's
    // antialiased bilinear resize always enlarges it. Torchvision rounds the
    // interpolated values back to uint8 before applying fused normalization.
    crate::parallel(|| {
        pixels
            .par_chunks_exact_mut(ENCODER_SIDE * ENCODER_SIDE)
            .enumerate()
            .for_each(|(channel, out)| {
                let mut rows = vec![0.0; image.height * ENCODER_SIDE];
                for (row, dest) in rows
                    .as_chunks_mut::<ENCODER_SIDE>()
                    .0
                    .iter_mut()
                    .enumerate()
                {
                    let base = row * image.width * 3 + channel;
                    for (dest, &x) in dest.iter_mut().zip(&horizontal) {
                        *dest = interpolate(
                            x,
                            f32::from(image.pixels[base + x.low * 3]),
                            f32::from(image.pixels[base + x.high * 3]),
                        );
                    }
                }
                let mean = mean[channel] * 255.0;
                let std = std[channel] * 255.0;
                for (row, &y) in out
                    .as_chunks_mut::<ENCODER_SIDE>()
                    .0
                    .iter_mut()
                    .zip(&vertical)
                {
                    for (x, dest) in row.iter_mut().enumerate() {
                        let value = interpolate(
                            y,
                            rows[y.low * ENCODER_SIDE + x],
                            rows[y.high * ENCODER_SIDE + x],
                        );
                        *dest = (value.round_ties_even() - mean) / std;
                    }
                }
            });
    });
    pixels
}

fn prompt_batch(width: usize, height: usize, first: usize) -> [f32; POINTS_PER_BATCH * 2] {
    // The released crop generator uses longest-edge point coordinates even
    // though its image processor resizes the encoder input to a square.
    let scale = ENCODER_SIDE as f64 / width.max(height) as f64;
    let scale_x = ((width as f64 * scale).round() / width as f64) as f32;
    let scale_y = ((height as f64 * scale).round() / height as f64) as f32;
    let mut points = [0.0; POINTS_PER_BATCH * 2];
    for (index, point) in points.as_chunks_mut::<2>().0.iter_mut().enumerate() {
        let grid = first + index;
        let x = (grid % POINTS_PER_SIDE) as f32 + 0.5;
        let y = (grid / POINTS_PER_SIDE) as f32 + 0.5;
        point[0] = (x / POINTS_PER_SIDE as f32 * width as f32) * scale_x;
        point[1] = (y / POINTS_PER_SIDE as f32 * height as f32) * scale_y;
    }
    points
}

#[derive(Clone, Copy)]
struct Bounds {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl Bounds {
    fn iou(self, other: Self) -> f32 {
        let intersection = (self
            .right
            .min(other.right)
            .saturating_sub(self.left.max(other.left))
            * self
                .bottom
                .min(other.bottom)
                .saturating_sub(self.top.max(other.top))) as f32;
        let area = |b: Self| ((b.right - b.left) * (b.bottom - b.top)) as f32;
        intersection / (area(self) + area(other) - intersection)
    }
}

struct Mask {
    foreground: Vec<bool>,
    bounds: Bounds,
    quality: f32,
}

fn stable_mask(
    raw: &[f32],
    quality: f32,
    xs: &[AxisSample],
    ys: &[AxisSample],
) -> Result<Option<Mask>, &'static str> {
    if !quality.is_finite() {
        return Err("SAM2 decoder returned a non-finite quality score");
    }
    if quality <= 0.8 {
        return Ok(None);
    }
    if raw.iter().any(|value| !value.is_finite()) {
        return Err("SAM2 decoder returned non-finite logits for an eligible mask");
    }

    let width = xs.len();
    let mut upper = vec![0.0; width];
    let mut lower = vec![0.0; width];
    let mut upper_index = usize::MAX;
    let mut lower_index = usize::MAX;
    let mut intersection = 0;
    let mut union = 0;
    let mut foreground = vec![false; width * ys.len()];

    // Adjacent output rows reuse the same horizontal interpolation. Cache
    // the two source rows so the vertical pass scans contiguous slices.
    for (row, sample) in foreground.chunks_exact_mut(width).zip(ys) {
        if upper_index != sample.low {
            if lower_index == sample.low {
                std::mem::swap(&mut upper, &mut lower);
                std::mem::swap(&mut upper_index, &mut lower_index);
            } else {
                interpolate_row(raw, sample.low, xs, &mut upper);
                upper_index = sample.low;
            }
        }
        let lower = if sample.high == sample.low {
            &upper
        } else {
            if lower_index != sample.high {
                interpolate_row(raw, sample.high, xs, &mut lower);
                lower_index = sample.high;
            }
            &lower
        };
        let low_weight = 1.0 - sample.fraction;
        let high_weight = sample.fraction;
        for ((included, &low), &high) in row.iter_mut().zip(&upper).zip(lower) {
            let value = low_weight * low + high_weight * high;
            *included = value > 0.0;
            intersection += usize::from(value > 1.0);
            union += usize::from(value > -1.0);
        }
    }

    if union == 0 || intersection as f32 / union as f32 <= 0.9 {
        return Ok(None);
    }

    let mut bounds = Bounds {
        left: width,
        top: ys.len(),
        right: 0,
        bottom: 0,
    };
    for (y, row) in foreground.chunks_exact(width).enumerate() {
        if let Some(left) = row.iter().position(|&included| included) {
            let right = row.iter().rposition(|&included| included).unwrap();
            bounds.left = bounds.left.min(left);
            bounds.top = bounds.top.min(y);
            bounds.right = bounds.right.max(right);
            bounds.bottom = y;
        }
    }

    Ok(Some(Mask {
        foreground,
        bounds,
        quality,
    }))
}

fn interpolate_row(raw: &[f32], y: usize, xs: &[AxisSample], output: &mut [f32]) {
    let source = &raw[y * MASK_SIDE..][..MASK_SIDE];
    for (value, &sample) in output.iter_mut().zip(xs) {
        *value = interpolate(sample, source[sample.low], source[sample.high]);
    }
}

fn suppress(mut candidates: Vec<Mask>) -> (Vec<Vec<bool>>, Vec<f32>) {
    // Stable sorting preserves prompt order for equal box scores.
    candidates.sort_by(|a, b| b.quality.total_cmp(&a.quality));
    let mut boxes = Vec::<Bounds>::new();
    let mut masks = Vec::new();
    let mut scores = Vec::new();
    for candidate in candidates {
        if boxes
            .iter()
            .any(|bounds| bounds.iou(candidate.bounds) > 0.7)
        {
            continue;
        }
        boxes.push(candidate.bounds);
        masks.push(candidate.foreground);
        scores.push(candidate.quality);
    }
    (masks, scores)
}

impl Sam2 {
    pub(super) fn load(models: &Path, backend: Backend) -> Result<Self, String> {
        Ok(Self {
            // The CUDA provider can use its tensor cores for SAM2 projections.
            encoder: load_session(&models.join("sam_encoder.onnx"), backend, true)?,
            decoder: load_session(&models.join("sam_decoder.onnx"), backend, true)?,
        })
    }

    pub(super) fn masks(&mut self, preview: &Rgb8) -> Result<(Vec<Vec<bool>>, Vec<f32>), String> {
        let inputs = preprocess(preview);
        let pixels = Tensor::from_array(([1, 3, ENCODER_SIDE, ENCODER_SIDE], inputs))
            .map_err(|error| error.to_string())?;
        let features = self
            .encoder
            .run(ort::inputs!["pixels" => pixels])
            .map_err(|error| error.to_string())?;
        let feature = |name| {
            features
                .get(name)
                .ok_or_else(|| format!("SAM2 encoder omitted {name}"))
        };
        let (high, middle, low) = (feature("high")?, feature("middle")?, feature("low")?);
        let xs = sample_axis(MASK_SIDE, preview.width);
        let ys = sample_axis(MASK_SIDE, preview.height);
        let masks_per_batch = POINTS_PER_BATCH * MASKS_PER_POINT;
        let mut candidates = Vec::new();

        for start in (0..POINTS_PER_SIDE * POINTS_PER_SIDE).step_by(POINTS_PER_BATCH) {
            let points = prompt_batch(preview.width, preview.height, start);
            let points = Tensor::from_array(([1, POINTS_PER_BATCH, 1, 2], points.to_vec()))
                .map_err(|error| error.to_string())?;
            let results = self
                .decoder
                .run(ort::inputs![
                    "high" => high, "middle" => middle, "low" => low, "points" => points,
                ])
                .map_err(|error| error.to_string())?;

            let output = |name| {
                results
                    .get(name)
                    .ok_or_else(|| format!("SAM2 decoder omitted {name}"))?
                    .try_extract_tensor::<f32>()
                    .map_err(|error| error.to_string())
            };
            let (mask_shape, logits) = output("masks")?;
            let (quality_shape, scores) = output("quality")?;
            if **mask_shape
                != [
                    1,
                    POINTS_PER_BATCH as i64,
                    MASKS_PER_POINT as i64,
                    MASK_SIDE as i64,
                    MASK_SIDE as i64,
                ]
                || **quality_shape != [1, POINTS_PER_BATCH as i64, MASKS_PER_POINT as i64]
            {
                return Err(format!(
                    "SAM2 decoder returned mask shape {mask_shape:?} and quality shape {quality_shape:?}"
                ));
            }
            let batch = crate::parallel(|| {
                logits
                    .par_chunks_exact(MASK_SIDE * MASK_SIDE)
                    .zip(scores.par_iter())
                    .map(|(raw, &quality)| stable_mask(raw, quality, &xs, &ys))
                    .collect::<Result<Vec<_>, _>>()
            })?;
            debug_assert_eq!(batch.len(), masks_per_batch);
            candidates.extend(batch.into_iter().flatten());
        }

        Ok(suppress(candidates))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangular_preview_uses_canonical_prompt_geometry() {
        assert_eq!(prompt_batch(640, 480, 0)[..4], [32.0, 24.0, 96.0, 24.0]);
        assert_eq!(
            prompt_batch(427, 640, 224)[60..],
            [618.96875, 992.0, 661.65625, 992.0]
        );
    }

    #[test]
    fn stable_mask_matches_per_pixel_interpolation_at_thresholds() {
        let raw = (0..MASK_SIDE * MASK_SIDE)
            .map(|i| {
                let x = i % MASK_SIDE;
                let y = i / MASK_SIDE;
                if x > 9 && y > 7 {
                    match i % 41 {
                        0 => 1.0,
                        1 => 1.0_f32.next_down(),
                        2 => 1.0_f32.next_up(),
                        3 => 0.0,
                        4 => -1.0,
                        _ => 6.0,
                    }
                } else {
                    -6.0
                }
            })
            .collect::<Vec<_>>();
        for (width, height) in [(23, 19), (640, 427), (640, 640)] {
            let xs = sample_axis(MASK_SIDE, width);
            let ys = sample_axis(MASK_SIDE, height);
            let mask = stable_mask(&raw, 0.9, &xs, &ys).unwrap().unwrap();
            let mut coords = Vec::new();
            for (y, &vertical) in ys.iter().enumerate() {
                for (x, &horizontal) in xs.iter().enumerate() {
                    let sample = |row: usize| {
                        interpolate(
                            horizontal,
                            raw[row * MASK_SIDE + horizontal.low],
                            raw[row * MASK_SIDE + horizontal.high],
                        )
                    };
                    let value = interpolate(vertical, sample(vertical.low), sample(vertical.high));
                    assert_eq!(mask.foreground[y * width + x], value > 0.0);
                    if value > 0.0 {
                        coords.push((x, y));
                    }
                }
            }
            assert_eq!(mask.bounds.left, coords.iter().map(|c| c.0).min().unwrap());
            assert_eq!(mask.bounds.right, coords.iter().map(|c| c.0).max().unwrap());
            assert_eq!(mask.bounds.top, coords.iter().map(|c| c.1).min().unwrap());
            assert_eq!(
                mask.bounds.bottom,
                coords.iter().map(|c| c.1).max().unwrap()
            );
        }
    }

    #[test]
    fn stable_mask_thresholds_and_nms_keep_first_equal_score() {
        let xs = sample_axis(MASK_SIDE, 11);
        let ys = sample_axis(MASK_SIDE, 9);
        let raw = vec![2.0; MASK_SIDE * MASK_SIDE];
        assert!(stable_mask(&raw, 0.8, &xs, &ys).unwrap().is_none());
        assert!(stable_mask(&raw, 0.9, &xs, &ys).unwrap().is_some());
        assert!(
            stable_mask(&vec![0.0; raw.len()], 0.9, &xs, &ys)
                .unwrap()
                .is_none()
        );
        assert!(stable_mask(&raw, f32::NAN, &xs, &ys).is_err());
        assert!(stable_mask(&vec![f32::INFINITY; raw.len()], 0.9, &xs, &ys).is_err());

        let bounds = Bounds {
            left: 1,
            top: 1,
            right: 3,
            bottom: 3,
        };
        let masks = vec![
            Mask {
                foreground: vec![true, false],
                bounds,
                quality: 0.9,
            },
            Mask {
                foreground: vec![false, true],
                bounds,
                quality: 0.9,
            },
        ];
        assert_eq!(suppress(masks), (vec![vec![true, false]], vec![0.9]));
    }
}
