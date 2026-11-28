use cubecl::prelude::*;

#[cube(expand_only)]
fn work_span(length: usize, #[comptime] span: usize) -> (usize, usize) {
    let first = ABSOLUTE_POS * span;
    (first, (first + span).min(length))
}

#[cube(launch, expand_only)]
pub(super) fn mask_samples(
    source: &Tensor<f32>,
    output: &mut Tensor<f32>,
    area: usize,
    #[comptime] span: usize,
) {
    let (first, end) = work_span(output.len(), span);
    for index in first..end {
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

#[cube(expand_only)]
fn gaussian_pair(
    center: usize,
    coordinate: usize,
    length: usize,
    stride: usize,
    distance: usize,
    #[comptime] interior: bool,
) -> (usize, usize) {
    if interior {
        let offset = stride * distance;
        (center - offset, center + offset)
    } else {
        let (left, right) = reflected_pair(coordinate, distance, length);
        let origin = center - coordinate * stride;
        (origin + left * stride, origin + right * stride)
    }
}

#[cube(expand_only)]
fn gaussian_span(
    source: &Tensor<f32>,
    weights: &Tensor<f32>,
    output: &mut Tensor<f32>,
    first: usize,
    center: usize,
    coordinate: usize,
    length: usize,
    stride: usize,
    #[comptime] radius: usize,
    #[comptime] span: usize,
    #[comptime] interior: bool,
) {
    let mut sums = Array::<f32>::new(span);
    let mut corrections = Array::<f32>::new(span);
    let center_weight = weights[radius];
    for lane in 0..span {
        sums[lane] = source[center + lane] * center_weight;
    }
    // Keep the center-first, outside-in tap order for every lane. Sharing
    // addresses across columns lets CPU backends vectorize independent sums.
    for step in 0..radius {
        let (left, right) =
            gaussian_pair(center, coordinate, length, stride, radius - step, interior);
        let weight = weights[step];
        for lane in 0..span {
            let term = (source[left + lane] + source[right + lane]) * weight;
            let corrected = term - corrections[lane];
            let value = sums[lane];
            let updated = value + corrected;
            corrections[lane] = (updated - value) - corrected;
            sums[lane] = updated;
        }
    }
    for lane in 0..span {
        output[first + lane] = sums[lane];
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
    #[comptime] span: usize,
) {
    let columns = width.div_ceil(span);
    let row = ABSOLUTE_POS / columns;
    if row >= planes * height {
        terminate!();
    }
    let x = (ABSOLUTE_POS % columns) * span;
    let y = row % height;
    let first = row * width + x;
    let center = first + first_plane * width * height;
    let (coordinate, length, stride) = if vertical {
        (y, height, width)
    } else {
        (x, width, 1usize)
    };
    let axis_span = if vertical { 1usize } else { span };
    let interior = coordinate >= radius && coordinate + radius + axis_span <= length;
    let full_span = x + span <= width;

    if full_span && interior {
        gaussian_span(
            source, weights, output, first, center, coordinate, length, stride, radius, span, true,
        );
    } else if vertical && full_span {
        gaussian_span(
            source, weights, output, first, center, coordinate, length, stride, radius, span, false,
        );
    } else {
        // Horizontal reflection can give each lane a different offset. The
        // same stencil handles those edge lanes and any short row tail.
        let end = width.min(x + span) - x;
        for lane in 0..end {
            let coordinate = if vertical { y } else { x + lane };
            gaussian_span(
                source,
                weights,
                output,
                first + lane,
                center + lane,
                coordinate,
                length,
                stride,
                radius,
                1usize,
                false,
            );
        }
    }
}

#[cube(launch, expand_only)]
pub(super) fn normalize(
    values: &mut Tensor<f32>,
    area: usize,
    count: usize,
    #[comptime] span: usize,
) {
    let (first, end) = work_span(area * count, span);
    for index in first..end {
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
    #[comptime] span: usize,
) {
    let (first, end) = work_span(area, span);
    for index in first..end {
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
    #[comptime] span: usize,
) {
    let area = width * height;
    let (first, end) = work_span(area, span);
    for center in first..end {
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
}

#[cube(launch, expand_only)]
pub(super) fn extrema(
    source: &Tensor<f32>,
    edges: &mut Tensor<f32>,
    width: usize,
    height: usize,
    #[comptime] span: usize,
) {
    let area = width * height;
    let (first, end) = work_span(area, span);
    for index in first..end {
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
}

#[cube(launch, expand_only)]
pub(super) fn nearby(
    source: &Tensor<f32>,
    output: &mut Tensor<f32>,
    width: usize,
    height: usize,
    #[comptime] vertical: bool,
    #[comptime] span: usize,
) {
    let area = width * height;
    let (first, end) = work_span(area, span);
    for index in first..end {
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
}

#[cube(launch, expand_only)]
pub(super) fn finish(
    source: &Tensor<f32>,
    residuals: &Tensor<f32>,
    edges: &Tensor<f32>,
    output: &mut Tensor<f32>,
    area: usize,
    #[comptime] span: usize,
) {
    let (first, end) = work_span(area, span);
    for index in first..end {
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
}
