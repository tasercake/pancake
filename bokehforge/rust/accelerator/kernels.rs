use cubecl::prelude::*;

#[cube(launch, expand_only)]
pub(super) fn mask_samples(source: &Tensor<f32>, output: &mut Tensor<f32>, area: usize) {
    let index = ABSOLUTE_POS;
    if index < output.len() {
        let value = source[index];
        if index < 3 * area {
            output[index] = value * source[3 * area + index % area];
        } else {
            output[index] = value;
        }
    }
}

#[cube(expand_only)]
fn reflect(index: i32, length: i32) -> i32 {
    let period = 2 * length;
    let mut position = index % period;
    if position < 0 {
        position += period;
    }
    position.min(period - 1 - position)
}

#[cube(expand_only)]
fn reflected_pair(coordinate: usize, distance: usize, length: usize) -> (usize, usize) {
    let index = i32::cast_from(coordinate);
    let distance = i32::cast_from(distance);
    let size = i32::cast_from(length);
    let left = index - distance;
    let right = index + distance;
    if length > distance as usize {
        (
            left.max(-left - 1) as usize,
            right.min(2 * size - 1 - right) as usize,
        )
    } else {
        (reflect(left, size) as usize, reflect(right, size) as usize)
    }
}

#[cube(launch, expand_only)]
pub(super) fn gaussian(
    source: &Tensor<f32>,
    weights: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    planes: usize,
    first_plane: usize,
    #[comptime] radius: usize,
    #[comptime] vertical: bool,
) {
    let area = width * height;
    let index = ABSOLUTE_POS;
    if index >= area * planes {
        terminate!();
    }
    let position = index % area;
    let x = position % width;
    let y = position / width;
    let center = index + first_plane * area;
    let mut value = source[center] * weights[radius];
    let mut correction = 0.0f32;
    // Keep the reference's center-first, outside-in pair order. Compensated
    // float32 arithmetic is available on CPU, CUDA and Metal.
    for step in 0..radius {
        let distance = radius - step;
        let (left, right) = if vertical {
            let (a, b) = reflected_pair(y, distance, height);
            (
                (center - y * width) + a * width,
                (center - y * width) + b * width,
            )
        } else {
            let (a, b) = reflected_pair(x, distance, width);
            (center - x + a, center - x + b)
        };
        let pair = source[left] + source[right];
        let term = pair * weights[step];
        let corrected = term - correction;
        let updated = value + corrected;
        correction = (updated - value) - corrected;
        value = updated;
    }
    output[index] = value;
}

#[cube(launch, expand_only)]
pub(super) fn normalize(values: &mut Tensor<f32>, area: usize, count: usize) {
    let index = ABSOLUTE_POS;
    if index < area * count {
        let weight = values[area * count + index % area];
        if weight > 0. {
            values[index] /= weight;
        } else {
            values[index] = 0.;
        }
    }
}

#[cube(launch, expand_only)]
pub(super) fn copy_plane(
    source: &Tensor<f32>,
    destination: &mut Tensor<f32>,
    area: usize,
    source_plane: usize,
    destination_plane: usize,
) {
    let index = ABSOLUTE_POS;
    if index < area {
        destination[destination_plane * area + index] = source[source_plane * area + index];
    }
}

#[cube(expand_only)]
fn visibility(error: f32, scale: f32) -> f32 {
    error / (error + scale)
}

#[cube(launch, expand_only)]
pub(super) fn directional(
    source: &Tensor<f32>,
    original: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    #[comptime] masked: bool,
    #[comptime] initial: bool,
) {
    let area = width * height;
    let center = ABSOLUTE_POS;
    if center >= area {
        terminate!();
    }
    let x = center % width;
    let y = center / width;
    let c = source[center];
    let t = source[area + center];
    let mut most_lost = 0.0f32;
    let mut most_added = 0.0f32;
    let diagonal = std::f32::consts::SQRT_2;
    let mut include_center = true;
    if masked {
        include_center = original[3 * area + center] > 0.;
    }
    if include_center {
        #[unroll]
        for row in 0usize..3usize {
            #[unroll]
            for column in 0usize..3usize {
                if (row != 1usize || column != 1usize)
                    && y + row > 0
                    && y + row <= height
                    && x + column > 0
                    && x + column <= width
                {
                    let adjacent = (y + row - 1) * width + x + column - 1;
                    let mut include_neighbor = true;
                    if masked {
                        include_neighbor = original[3 * area + adjacent] > 0.;
                    }
                    if include_neighbor {
                        let dc = source[adjacent] - c;
                        let dt = source[area + adjacent] - t;
                        let ac = dc.abs();
                        let at = dt.abs();
                        let mut earned = 0.0f32;
                        if (dc > 0.0f32 && dt > 0.0f32) || (dc < 0.0f32 && dt < 0.0f32) {
                            earned = ac.min(at);
                        }
                        let mut loss = at - earned;
                        let mut gain = ac - earned;
                        if row != 1usize && column != 1usize {
                            loss /= diagonal;
                            gain /= diagonal;
                        }
                        most_lost = most_lost.max(loss);
                        most_added = most_added.max(gain);
                    }
                }
            }
        }
    }
    let mut lost = most_lost / (most_lost + 5.0f32);
    let mut added = most_added / (most_added + 5.0f32);
    if !initial {
        lost = lost.max(output[area + center]);
        added = added.max(output[2 * area + center]);
    }
    output[area + center] = lost;
    output[2 * area + center] = added;
}

#[cube(launch, expand_only)]
pub(super) fn extrema(source: &Tensor<f32>, edges: &mut Tensor<f32>, width: usize, height: usize) {
    let area = width * height;
    let index = ABSOLUTE_POS;
    if index >= area {
        terminate!();
    }
    let x = index % width;
    let y = index / width;
    let center = source[area + index];
    let mut upper = center;
    let mut lower = center;
    // Reflection revisits samples within these odd extrema windows.
    #[unroll]
    for row in 0usize..3usize {
        #[unroll]
        for column in 0usize..3usize {
            let py = (y + row).max(1).min(height) - 1;
            let px = (x + column).max(1).min(width) - 1;
            let value = source[area + py * width + px];
            upper = upper.max(value);
            lower = lower.min(value);
        }
    }
    edges[index] = upper;
    edges[area + index] = lower;
    edges[2 * area + index] = visibility(upper - lower, 6.);
}

#[cube(launch, expand_only)]
pub(super) fn nearby(
    source: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    #[comptime] vertical: bool,
) {
    let area = width * height;
    let index = ABSOLUTE_POS;
    if index >= area {
        terminate!();
    }
    let x = index % width;
    let y = index / width;
    let mut strongest = 0.0f32;
    #[unroll]
    for offset in 0usize..9usize {
        let value = if vertical {
            let py = (y + offset).max(4usize).min(height + 3usize) - 4usize;
            source[py * width + x]
        } else {
            let px = (x + offset).max(4usize).min(width + 3usize) - 4usize;
            source[2 * area + y * width + px]
        };
        if value > strongest {
            strongest = value;
        }
    }
    if vertical {
        output[3 * area + index] = strongest;
    } else {
        output[index] = strongest;
    }
}

#[cube(launch, expand_only)]
pub(super) fn finish(
    source: &Tensor<f32>,
    residuals: &Tensor<f32>,
    edges: &Tensor<f32>,
    output: &mut Tensor<f32>,
    area: usize,
) {
    let index = ABSOLUTE_POS;
    if index >= area {
        terminate!();
    }
    let candidate = source[index];
    let delta = source[2 * area + index];
    let fine = residuals[index];
    let local = residuals[area + index];
    let broad = residuals[2 * area + index];
    let upper = edges[index];
    let lower = edges[area + index];
    let strength = edges[2 * area + index];
    let neighborhood = edges[3 * area + index];
    let grain = visibility((delta - fine).abs(), 5.) * (1. - strength);
    let edge_shape = visibility((delta - local).abs(), 6.) * neighborhood;
    let overshoot = (candidate - upper).max(lower - candidate).max(0.);
    output[index] = visibility(local.abs().max(broad.abs()), 8.);
    output[2 * area + index] = output[2 * area + index].max(grain);
    output[3 * area + index] = edge_shape.max(visibility(overshoot, 6.) * neighborhood);
}
