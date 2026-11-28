use std::collections::HashMap;
use std::sync::Arc;

use rayon::prelude::*;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rustfft::{Fft, FftPlanner, num_complex::Complex32};

use super::Pose;
use super::texture::{Texture, contiguous_sum, deviation};

pub(super) struct CoarseSearch {
    pub peaks: Vec<(f64, Pose)>,
    pub attempted: usize,
    pub anchors: Vec<[f64; 2]>,
}

fn anchors(texture: &Texture) -> Vec<[f64; 2]> {
    let [height, width] = texture.shape;
    let mut cells = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            if (y + x) % 2 != 0 {
                continue;
            }
            let mut values = Vec::new();
            for row in y * height / 4..((y + 1) * height).div_ceil(4) {
                values.extend_from_slice(
                    &texture.pixels
                        [row * width + x * width / 4..row * width + ((x + 1) * width).div_ceil(4)],
                );
            }
            let contrast = deviation(&values).into_iter().fold(0.0, f32::max);
            cells.push((contrast, y, x));
        }
    }
    cells.sort_by(|a, b| b.0.total_cmp(&a.0).then(b.1.cmp(&a.1)).then(b.2.cmp(&a.2)));
    let mut seen = [[false; 2]; 2];
    let mut anchors = Vec::with_capacity(4);
    for (_, y, x) in cells {
        if !std::mem::replace(&mut seen[y / 2][x / 2], true) {
            let cell = [y, x];
            anchors.push(std::array::from_fn(|axis| {
                let center = texture.shape[axis] as f64 * (cell[axis] as f64 + 0.5) / 4.0;
                center * texture.step[axis] - 0.5
            }));
        }
    }
    anchors
}

struct Offsets {
    transform: Pose,
    shape: [usize; 2],
    points: Vec<[f64; 2]>,
}

fn offsets(pose: Pose, dimensions: [usize; 2], reference: &Texture) -> Option<Offsets> {
    let m = pose.matrix();
    let determinant = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    let inverse = [
        [m[1][1] / determinant, -m[0][1] / determinant],
        [-m[1][0] / determinant, m[0][0] / determinant],
    ];
    let half = dimensions.map(|n| n as f64 * pose.scale * 0.115);
    let shrink = (0..2)
        .map(|axis| {
            let extent = inverse[axis][0].abs() * half[0] + inverse[axis][1].abs() * half[1];
            dimensions[axis] as f64 * 0.115 / extent
        })
        .fold(1.0, f64::min);
    let [hy, hx] = std::array::from_fn(|axis| {
        ((half[axis] * shrink / reference.step[axis]).floor() as usize)
            .min((reference.shape[axis] - 1) / 2)
            .min(20)
    });
    if hy.min(hx) < 2 {
        return None;
    }
    let mut points = Vec::with_capacity((2 * hy + 1) * (2 * hx + 1));
    for y in -(hy as isize)..=hy as isize {
        for x in -(hx as isize)..=hx as isize {
            let y = y as f64 * reference.step[0];
            let x = x as f64 * reference.step[1];
            points.push(inverse.map(|row| row[0] * y + row[1] * x));
        }
    }
    Some(Offsets {
        transform: pose,
        shape: [2 * hy + 1, 2 * hx + 1],
        points,
    })
}

fn smooth_length(minimum: usize) -> usize {
    let mut size = minimum;
    loop {
        let mut rest = size;
        for prime in [2, 3, 5] {
            while rest.is_multiple_of(prime) {
                rest /= prime;
            }
        }
        if rest == 1 {
            return size;
        }
        size += 1;
    }
}

struct FourierGrid {
    height: usize,
    width: usize,
    rows_forward: Arc<dyn RealToComplex<f32>>,
    rows_inverse: Arc<dyn ComplexToReal<f32>>,
    columns_forward: Arc<dyn Fft<f32>>,
    columns_inverse: Arc<dyn Fft<f32>>,
}

struct FftWork {
    values: Vec<Complex32>,
    product: Vec<Complex32>,
    row: Vec<f32>,
    row_spectrum: Vec<Complex32>,
    scratch: Vec<Complex32>,
}

impl FourierGrid {
    fn new(shape: [usize; 2]) -> Self {
        let [height, width] = shape.map(|n| smooth_length(n + 42));
        let mut real = RealFftPlanner::new();
        let mut complex = FftPlanner::new();
        Self {
            height,
            width,
            rows_forward: real.plan_fft_forward(width),
            rows_inverse: real.plan_fft_inverse(width),
            columns_forward: complex.plan_fft_forward(height),
            columns_inverse: complex.plan_fft_inverse(height),
        }
    }

    fn workspace(&self) -> FftWork {
        let bins = self.width / 2 + 1;
        let scratch = [
            self.rows_forward.get_scratch_len(),
            self.rows_inverse.get_scratch_len(),
            self.columns_forward.get_inplace_scratch_len(),
            self.columns_inverse.get_inplace_scratch_len(),
        ]
        .into_iter()
        .max()
        .unwrap_or_default();
        FftWork {
            values: vec![Complex32::ZERO; self.height * bins],
            product: vec![Complex32::ZERO; self.height * bins],
            row: vec![0.0; self.width],
            row_spectrum: vec![Complex32::ZERO; bins],
            scratch: vec![Complex32::ZERO; scratch],
        }
    }

    fn forward(&self, work: &mut FftWork, shape: [usize; 2], pixels: &[[f32; 3]], channel: usize) {
        work.values.fill(Complex32::ZERO);
        for (y, pixels) in pixels.chunks_exact(shape[1]).take(shape[0]).enumerate() {
            work.row.fill(0.0);
            for (value, rgb) in work.row.iter_mut().zip(pixels) {
                *value = rgb[channel];
            }
            // Each buffer has the sizes requested by this FFT plan.
            self.rows_forward
                .process_with_scratch(&mut work.row, &mut work.row_spectrum, &mut work.scratch)
                .expect("real FFT workspace matches the plan");
            for (x, &value) in work.row_spectrum.iter().enumerate() {
                work.values[x * self.height + y] = value;
            }
        }
        self.columns_forward
            .process_with_scratch(&mut work.values, &mut work.scratch);
    }

    fn inverse(&self, work: &mut FftWork, shape: [usize; 2], result: &mut Vec<f32>) {
        self.columns_inverse
            .process_with_scratch(&mut work.product, &mut work.scratch);
        let multiplier = (1.0 / (self.height * self.width) as f64) as f32;
        result.clear();
        for y in 0..shape[0] {
            let bins = work.row_spectrum.len();
            for (x, sample) in work.row_spectrum.iter_mut().enumerate() {
                *sample = work.product[x * self.height + y];
                // These bins are real in a Hermitian spectrum. Discard inverse
                // column roundoff before the real FFT validates its input.
                if x == 0 || (self.width.is_multiple_of(2) && x == bins - 1) {
                    sample.im = 0.0;
                }
            }
            self.rows_inverse
                .process_with_scratch(&mut work.row_spectrum, &mut work.row, &mut work.scratch)
                .expect("Hermitian FFT workspace matches the plan");
            result.extend(work.row[..shape[1]].iter().map(|value| value * multiplier));
        }
    }
}

/// Summed-area reference sums and squared RGB norm, accumulated vertically first.
struct Integral {
    width: usize,
    values: Vec<[f64; 4]>,
}

impl Integral {
    fn new(reference: &Texture) -> Self {
        let [height, width] = reference.shape;
        let stride = width + 1;
        let mut values = vec![[0.0; 4]; stride * (height + 1)];
        for y in 0..height {
            for x in 0..width {
                let v = reference.pixels[y * width + x];
                let squared = v[0] * v[0] + v[1] * v[1] + v[2] * v[2];
                let sample = [v[0], v[1], v[2], squared];
                let previous = values[y * stride + x + 1];
                values[(y + 1) * stride + x + 1] =
                    std::array::from_fn(|i| previous[i] + f64::from(sample[i]));
            }
        }
        for row in values.chunks_exact_mut(stride).skip(1) {
            let mut prefix = [0.0; 4];
            for point in row {
                for c in 0..4 {
                    prefix[c] += point[c];
                }
                *point = prefix;
            }
        }
        Self {
            width: stride,
            values,
        }
    }

    fn energy(&self, shape: [usize; 2], reach: [usize; 2]) -> Vec<f64> {
        let [h, w] = shape;
        let count = (h * w) as f64;
        let mut result = Vec::with_capacity(reach[0] * reach[1]);
        for y in 0..reach[0] {
            for x in 0..reach[1] {
                let a = &self.values[y * self.width + x];
                let b = &self.values[y * self.width + x + w];
                let c = &self.values[(y + h) * self.width + x];
                let d = &self.values[(y + h) * self.width + x + w];
                let summed = |c_index| d[c_index] - b[c_index] - c[c_index] + a[c_index];
                let [r, g, b] = [0, 1, 2].map(summed);
                let energy = summed(3) - (r * r + g * g + b * b) / count;
                result.push(energy.max(count * 1e-13));
            }
        }
        result
    }
}

pub(super) fn candidates(
    crop: &Texture,
    reference: &Texture,
    dimensions: [usize; 2],
    scales: &[f64],
    angles: &[f64],
) -> CoarseSearch {
    let anchors = anchors(crop);
    let offsets: Vec<_> = scales
        .iter()
        .flat_map(|&scale| {
            angles.iter().filter_map(move |&angle| {
                offsets(
                    Pose {
                        y: 0.0,
                        x: 0.0,
                        angle,
                        scale,
                    },
                    dimensions,
                    reference,
                )
            })
        })
        .collect();
    let grid = FourierGrid::new(reference.shape);
    let mut workspace = grid.workspace();
    let reference_spectrum = [0, 1, 2].map(|channel| {
        grid.forward(&mut workspace, reference.shape, &reference.pixels, channel);
        workspace.values.clone()
    });
    let integral = Integral::new(reference);
    let mut energies = HashMap::new();
    for template in &offsets {
        energies.entry(template.shape).or_insert_with(|| {
            let reach = [
                reference.shape[0] - template.shape[0] + 1,
                reference.shape[1] - template.shape[1] + 1,
            ];
            (reach, integral.energy(template.shape, reach))
        });
    }
    let tasks: Vec<_> = anchors
        .iter()
        .flat_map(|&anchor| offsets.iter().map(move |offset| (anchor, offset)))
        .collect();
    let mut result = CoarseSearch {
        peaks: Vec::new(),
        attempted: tasks.len(),
        anchors,
    };
    let all_peaks: Vec<_> = tasks
        .par_iter()
        .map_init(
            || (grid.workspace(), Vec::new(), Vec::new(), Vec::new()),
            |(work, template, correlations, agreements), (anchor, offsets)| {
                template.clear();
                template.extend(
                    offsets
                        .points
                        .iter()
                        .map(|p| crop.sample([p[0] + anchor[0], p[1] + anchor[1]])),
                );
                let count = template.len();
                let means = [0, 1, 2].map(|c| contiguous_sum(template, |v| v[c]) / count as f32);
                let mut energy = 0.0;
                for pixel in template.iter_mut() {
                    for channel in 0..3 {
                        pixel[channel] -= means[channel];
                        energy += f64::from(pixel[channel] * pixel[channel]);
                    }
                }
                let mut peaks = Vec::with_capacity(2);
                if energy <= count as f64 * 1e-13 {
                    return peaks;
                }
                work.product.fill(Complex32::ZERO);
                for (channel, spectrum) in reference_spectrum.iter().enumerate() {
                    grid.forward(work, offsets.shape, template, channel);
                    for ((product, &reference), &delivery) in
                        work.product.iter_mut().zip(spectrum).zip(&work.values)
                    {
                        *product += reference * delivery.conj();
                    }
                }
                let (reach, normalization) = &energies[&offsets.shape];
                grid.inverse(work, *reach, correlations);
                // Both peaks use the same normalization. Retain the f64
                // values and visit the unsuppressed windows in row-major order.
                agreements.clear();
                agreements.extend(correlations.iter().zip(normalization).map(
                    |(&value, &reference)| {
                        if value <= 0.0 {
                            return -1.0;
                        }
                        (f64::from(value) / (energy * reference).sqrt()).clamp(-1.0, 1.0)
                    },
                ));
                let mut selected = None;
                for _ in 0..2 {
                    let mut peak = 0.18;
                    let mut peak_at = None;
                    for (y, row) in agreements.chunks_exact(reach[1]).enumerate() {
                        let [left, right] = if let Some((_, px)) =
                            selected.filter(|(py, _): &(usize, usize)| y.abs_diff(*py) <= 3)
                        {
                            [px.saturating_sub(3), (px + 4).min(row.len())]
                        } else {
                            [row.len(), row.len()]
                        };
                        for range in [0..left, right..row.len()] {
                            let start = range.start;
                            for (x, &agreement) in row[range].iter().enumerate() {
                                if agreement > peak || (peak_at.is_none() && agreement == peak) {
                                    peak = agreement;
                                    peak_at = Some((y, start + x));
                                }
                            }
                        }
                    }
                    let Some((y, x)) = peak_at else { break };
                    selected = Some((y, x));
                    let center = [
                        (y + offsets.shape[0] / 2) as f64,
                        (x + offsets.shape[1] / 2) as f64,
                    ];
                    let rotated = offsets.transform.transform(*anchor);
                    let pose = Pose {
                        y: (center[0] + 0.5) * reference.step[0] - 0.5 - rotated[0],
                        x: (center[1] + 0.5) * reference.step[1] - 0.5 - rotated[1],
                        ..offsets.transform
                    };
                    let reliability = 0.30 * (-((count as f64) / 100.0)).exp();
                    peaks.push((peak - reliability, pose));
                }
                peaks
            },
        )
        .collect();
    result.peaks.extend(all_peaks.into_iter().flatten());
    result
}
