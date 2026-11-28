use ort::{session::Session, value::Tensor};

use super::image::{Rgb8, resize_map_bilinear, resize_rgb_bilinear};

fn run(
    model: &mut Session,
    pixels: Vec<f32>,
    width: usize,
    height: usize,
    name: &str,
    expected: &[i64],
) -> Result<Vec<f32>, String> {
    let input =
        Tensor::from_array(([1, 3, height, width], pixels)).map_err(|err| err.to_string())?;
    let outputs = model
        .run(ort::inputs!["pixels" => input])
        .map_err(|err| err.to_string())?;
    let (shape, values) = outputs
        .get(name)
        .ok_or_else(|| format!("Missing {name} output"))?
        .try_extract_tensor::<f32>()
        .map_err(|err| err.to_string())?;
    if &shape[..] != expected {
        return Err(format!(
            "Expected {name} with shape {expected:?}, received {shape:?}"
        ));
    }
    Ok(values.to_vec())
}

fn channel_planes(image: &Rgb8, offset: f32, scale: f32) -> Vec<f32> {
    (0..3)
        .flat_map(|channel| image.pixels[channel..].iter().step_by(3))
        .map(|&value| f32::from(value) / scale - offset)
        .collect()
}

fn sample_axis(source: usize, size: usize) -> Vec<(usize, usize, f32)> {
    let scale = source as f32 / size as f32;
    (0..size)
        .map(|index| {
            let coord = scale.mul_add(index as f32 + 0.5, -0.5).max(0.0);
            let first = (coord as usize).min(source - 1);
            (first, (first + 1).min(source - 1), coord - first as f32)
        })
        .collect()
}

// PyTorch F.interpolate(mode="bilinear", align_corners=False).
fn resize_bilinear(
    plane: &[f32],
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> Vec<f32> {
    if (width, height) == (out_width, out_height) {
        return plane.to_vec();
    }
    let xs = sample_axis(width, out_width);
    let ys = sample_axis(height, out_height);
    let mut output = Vec::with_capacity(out_width * out_height);
    for (top, bottom, fy) in ys {
        let row = |y: usize, left: usize, right: usize, fx: f32| {
            (1.0_f32 - fx).mul_add(plane[y * width + left], fx * plane[y * width + right])
        };
        for &(left, right, fx) in &xs {
            output.push(
                (1.0 - fy).mul_add(row(top, left, right, fx), fy * row(bottom, left, right, fx)),
            );
        }
    }
    output
}

// F.interpolate(mode="area") uses adaptive pooling, so fractional windows overlap.
fn resize_area(
    plane: &[f32],
    width: usize,
    height: usize,
    out_width: usize,
    out_height: usize,
) -> Vec<f32> {
    let mut output = Vec::with_capacity(out_width * out_height);
    for y in 0..out_height {
        let top = y * height / out_height;
        let bottom = ((y + 1) * height).div_ceil(out_height);
        for x in 0..out_width {
            let left = x * width / out_width;
            let right = ((x + 1) * width).div_ceil(out_width);
            let mut sum = 0.0_f32;
            for row in top..bottom {
                for value in &plane[row * width + left..row * width + right] {
                    sum += value;
                }
            }
            output.push(sum / (bottom - top) as f32 / (right - left) as f32);
        }
    }
    output
}

fn reflect(index: usize, length: usize) -> usize {
    if index < length {
        index
    } else {
        length - 2 - (index - length)
    }
}

fn reliqs_input(preview: &Rgb8) -> (Vec<f32>, usize, usize, usize, usize) {
    let width = preview.width.max(64);
    let height = preview.height.max(64);
    let working = resize_rgb_bilinear(preview, width, height);
    let pixels = channel_planes(&working, 0.0, 255.0);

    let shorter = width.min(height);
    let scale = 224.0 / shorter as f64;
    let scaled_width = (width as f64 * scale).round_ties_even() as usize;
    let scaled_height = (height as f64 * scale).round_ties_even() as usize;
    let padded_width = scaled_width.next_multiple_of(16);
    let padded_height = scaled_height.next_multiple_of(16);
    let mut output = Vec::with_capacity(3 * padded_width * padded_height);
    for plane in pixels.chunks_exact(width * height) {
        let resized;
        let plane = match shorter.cmp(&224) {
            std::cmp::Ordering::Less => {
                resized = resize_bilinear(plane, width, height, scaled_width, scaled_height);
                resized.as_slice()
            }
            std::cmp::Ordering::Equal => plane,
            std::cmp::Ordering::Greater => {
                resized = resize_area(plane, width, height, scaled_width, scaled_height);
                resized.as_slice()
            }
        };
        for y in 0..padded_height {
            let first = reflect(y, scaled_height) * scaled_width;
            let row = &plane[first..first + scaled_width];
            output.extend_from_slice(row);
            output.extend((scaled_width..padded_width).map(|x| row[reflect(x, scaled_width)]));
        }
    }
    (
        output,
        scaled_width,
        scaled_height,
        padded_width,
        padded_height,
    )
}

pub(super) fn reliqs(model: &mut Session, preview: &Rgb8) -> Result<Vec<f32>, String> {
    let (input, width, height, padded_width, padded_height) = reliqs_input(preview);
    let logits = run(
        model,
        input,
        padded_width,
        padded_height,
        "logits",
        &[1, 1, padded_height as i64, padded_width as i64],
    )?;
    let mut density = Vec::with_capacity(width * height);
    for row in logits.chunks_exact(padded_width).take(height) {
        density.extend_from_slice(&row[..width]);
    }
    let peak = density.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    for value in &mut density {
        *value = (*value - peak).exp();
    }
    let sum = density.iter().sum::<f32>();
    for value in &mut density {
        *value /= sum;
    }
    let mut heat = resize_bilinear(&density, width, height, preview.width, preview.height);
    let limit = heat.iter().copied().fold(0.0, f32::max) + 1e-8;
    for value in &mut heat {
        *value /= limit;
    }
    Ok(heat)
}

pub(super) fn tempsal(model: &mut Session, preview: &Rgb8) -> Result<Vec<f32>, String> {
    let resized = resize_rgb_bilinear(preview, 256, 256);
    let input = channel_planes(&resized, 1.0, 127.5);
    let mut saliency = run(model, input, 256, 256, "saliency", &[1, 256, 256])?;
    let limit = saliency.iter().copied().fold(1e-8, f32::max);
    for value in &mut saliency {
        *value /= limit;
    }
    Ok(resize_map_bilinear(
        &saliency,
        256,
        256,
        preview.width,
        preview.height,
    ))
}
