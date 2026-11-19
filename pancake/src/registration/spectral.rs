//! Coarse translation from normalized cross-power of two luma thumbnails.

use cubecl::prelude::*;
use cubecl::std::tensor::TensorHandle;

use super::{
    geometry::Extent,
    kernels::{bilinear, read},
};

/// Small fixed maximum for two separable radix-two transforms.
const SPECTRUM_LIMIT: usize = 128;

/// Resolve a widely supported translation in thumbnail coordinates.
///
/// # Errors
///
/// Returns a failed device readback or a malformed transform buffer error.
pub(super) fn phase_peak(
    client: &Client,
    reference: &TensorHandle,
    source: &TensorHandle,
    extent: Extent,
) -> Result<Option<[f32; 2]>, String> {
    let side = if extent.width.min(extent.height) < 128 {
        64
    } else {
        SPECTRUM_LIMIT
    };
    let area = side.saturating_mul(side);
    let complex = area.saturating_mul(2);
    let twiddles = TensorHandle::empty(client, [side.saturating_mul(2)], f32::cube_type());
    let a = forward(client, reference, extent, side, complex, &twiddles, true);
    let b = forward(client, source, extent, side, complex, &twiddles, false);
    let phase = TensorHandle::empty(client, [complex], f32::cube_type());
    let dim = CubeDim::new(client, complex);
    let count = cubecl::calculate_cube_count_elemwise(client, complex, dim);
    cross_phase::launch(
        client,
        count,
        dim,
        a.into_arg(),
        b.into_arg(),
        phase.clone().into_arg(),
        side,
    );
    let inverse = transform(client, phase, side, complex, &twiddles, true);
    let samples = read(client, inverse, complex)?;
    dominant_peak(&samples, side, extent)
}

/// Window a grayscale plane and compute its complete complex Fourier transform.
fn forward(
    client: &Client,
    samples: &TensorHandle,
    extent: Extent,
    side: usize,
    complex: usize,
    twiddles: &TensorHandle,
    prepare_twiddles: bool,
) -> TensorHandle {
    let input = TensorHandle::empty(client, [complex], f32::cube_type());
    let dim = CubeDim::new(client, complex);
    let count = cubecl::calculate_cube_count_elemwise(client, complex, dim);
    window_luma::launch(
        client,
        count,
        dim,
        samples.clone().into_arg(),
        input.clone().into_arg(),
        twiddles.clone().into_arg(),
        usize::from(extent.width),
        usize::from(extent.height),
        side,
        u32::from(prepare_twiddles),
    );
    transform(client, input, side, complex, twiddles, false)
}

/// Execute row and column passes on the same client.
fn transform(
    client: &Client,
    input: TensorHandle,
    side: usize,
    complex: usize,
    twiddles: &TensorHandle,
    inverse: bool,
) -> TensorHandle {
    let dim = CubeDim::new(client, side);
    let count = cubecl::calculate_cube_count_elemwise(client, side, dim);
    let scratch = TensorHandle::empty(client, [complex], f32::cube_type());
    let result = TensorHandle::empty(client, [complex], f32::cube_type());
    let direction_offset = if inverse { 2 } else { 0 };
    fft_lines::launch(
        client,
        count.clone(),
        dim,
        input.into_arg(),
        scratch.clone().into_arg(),
        twiddles.clone().into_arg(),
        side,
        0,
        direction_offset,
    );
    fft_lines::launch(
        client,
        count,
        dim,
        scratch.into_arg(),
        result.clone().into_arg(),
        twiddles.clone().into_arg(),
        side,
        1,
        direction_offset,
    );
    result
}

/// Read the strongest unambiguous shift from the normalized correlation surface.
///
/// # Errors
///
/// Returns an error if coordinate arithmetic exceeds the bounded FFT layout.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "nonzero power-of-two FFT side bounds division and signed wraparound"
)]
fn dominant_peak(matrix: &[f32], side: usize, extent: Extent) -> Result<Option<[f32; 2]>, String> {
    let (bins, _) = matrix.as_chunks::<2>();
    let bins = bins.iter().enumerate();
    let Some((index, &[strength, _])) = bins
        .clone()
        .max_by(|(_, [a, _]), (_, [b, _])| a.total_cmp(b))
    else {
        return Ok(None);
    };
    let (peak_x, peak_y) = (index % side, index / side);
    let radius = |candidate: usize, peak: usize| {
        let distance = candidate.abs_diff(peak);
        distance.min(side.saturating_sub(distance))
    };
    let ambiguous = bins
        .filter(|(index, _)| radius(index % side, peak_x).max(radius(index / side, peak_y)) > 2)
        .any(|(_, &[other, _])| other >= 0.92 * strength);
    let side = i16::try_from(side).map_err(|error| error.to_string())?;
    if strength < 0.05 * f32::from(side).powi(2) || ambiguous {
        return Ok(None);
    }
    let displacement = |bin: usize, dimension: u16| -> Result<f32, String> {
        let bin = i16::try_from(bin).map_err(|error| error.to_string())?;
        let signed = if bin >= side / 2 { bin - side } else { bin };
        Ok((f32::from(signed) / f32::from(side)) * f32::from(dimension))
    };
    Ok(Some([
        displacement(peak_x, extent.width)?,
        displacement(peak_y, extent.height)?,
    ]))
}

/// Window image gradients to suppress false shifts at the periodic FFT boundary.
#[cube(launch, expand_only)]
fn window_luma(
    input: &Tensor<f32>,
    output: &mut Tensor<f32>,
    twiddles: &mut Tensor<f32>,
    width: usize,
    height: usize,
    side: usize,
    prepare_twiddles: u32,
) {
    let half = side / 2;
    if prepare_twiddles != 0 && ABSOLUTE_POS < half {
        let at = ABSOLUTE_POS * 4;
        let scale = f32::cast_from(half);
        let numerator = core::f32::consts::PI * f32::cast_from(ABSOLUTE_POS);
        let forward = -numerator / scale;
        let inverse = numerator / scale;
        twiddles[at] = forward.cos();
        twiddles[at + 1] = forward.sin();
        twiddles[at + 2] = inverse.cos();
        twiddles[at + 3] = inverse.sin();
    }
    let pixel = ABSOLUTE_POS / 2;
    if ABSOLUTE_POS % 2 == 0 {
        let edge = f32::cast_from(side);
        let col = f32::cast_from(pixel % side) + 0.5;
        let row = f32::cast_from(pixel / side) + 0.5;
        let sx = f32::cast_from(width) / edge;
        let sy = f32::cast_from(height) / edge;
        let x = col * sx - 0.5;
        let y = row * sy - 0.5;
        let center = bilinear(input, x, y, width, height, 0, 1);
        let left = bilinear(input, x - sx, y, width, height, 0, 1);
        let right = bilinear(input, x + sx, y, width, height, 0, 1);
        let top = bilinear(input, x, y - sy, width, height, 0, 1);
        let bottom = bilinear(input, x, y + sy, width, height, 0, 1);
        let dx = (col * core::f32::consts::PI / edge).sin();
        let dy = (row * core::f32::consts::PI / edge).sin();
        output[ABSOLUTE_POS] = dx * dx * dy * dy * (center - 0.25 * (left + right + top + bottom));
    } else {
        output[ABSOLUTE_POS] = 0.0;
    }
}

/// Reorder one index by its low-order bits for a radix-two butterfly.
#[cube(expand_only)]
fn bit_reverse(index: usize, side: usize) -> usize {
    let mut value = index;
    let mut remaining = side;
    let mut reversed = 0_usize;
    while remaining > 1 {
        reversed = (reversed << 1) | (value & 1);
        value >>= 1;
        remaining >>= 1;
    }
    reversed
}

/// Pick a complex cell in a row or column of a square image.
#[cube(expand_only)]
fn cell(line: usize, index: usize, side: usize, vertical: bool) -> usize {
    if vertical {
        (index * side + line) * 2
    } else {
        (line * side + index) * 2
    }
}

/// Compute independent FFTs using two fixed-size local planes per active line.
#[cube(launch, expand_only)]
fn fft_lines(
    input: &Tensor<f32>,
    output: &mut Tensor<f32>,
    twiddles: &Tensor<f32>,
    side: usize,
    axis: usize,
    direction_offset: usize,
) {
    if ABSOLUTE_POS >= side {
        terminate!();
    }
    let mut real = Array::<f32>::new(SPECTRUM_LIMIT);
    let mut imaginary = Array::<f32>::new(SPECTRUM_LIMIT);
    let line = ABSOLUTE_POS;
    let vertical = axis != 0;
    for index in 0..side {
        let input_at = cell(line, bit_reverse(index, side), side, vertical);
        real[index] = input[input_at];
        imaginary[index] = input[input_at + 1];
    }

    let mut half = 1_usize;
    while half < side {
        // Every earlier stage is a dyadic subsample of the final-stage angles.
        let step = side / (2 * half);
        for pair in 0..side / 2 {
            let offset = pair % half;
            let first = 2 * (pair - offset) + offset;
            let second = first + half;
            let term = 4 * offset * step + direction_offset;
            let cos = twiddles[term];
            let sin = twiddles[term + 1];
            let re = real[second] * cos - imaginary[second] * sin;
            let im = real[second] * sin + imaginary[second] * cos;
            real[second] = real[first] - re;
            imaginary[second] = imaginary[first] - im;
            real[first] += re;
            imaginary[first] += im;
        }
        half *= 2;
    }

    for index in 0..side {
        let output_at = cell(line, index, side, vertical);
        output[output_at] = real[index];
        output[output_at + 1] = imaginary[index];
    }
}

/// Normalize cross power and retain phase, with no contribution from zero bins.
#[cube(launch, expand_only)]
fn cross_phase(
    reference: &Tensor<f32>,
    source: &Tensor<f32>,
    phase: &mut Tensor<f32>,
    side: usize,
) {
    let index = (ABSOLUTE_POS / 2 % (side * side)) * 2;
    let dot = source[index] * reference[index] + source[index + 1] * reference[index + 1];
    let cross = source[index + 1] * reference[index] - source[index] * reference[index + 1];
    let magnitude = (dot * dot + cross * cross).sqrt();
    let value = if ABSOLUTE_POS % 2 == 0 { dot } else { cross };
    phase[ABSOLUTE_POS] = if magnitude > 1e-9 {
        value / magnitude
    } else {
        0.0_f32
    };
}

#[cfg(test)]
mod tests {
    use super::{Extent, dominant_peak};

    /// A unique weak peak abstains and the supported signed peak is retained.
    /// # Errors
    /// Returns invalid test geometry or an incorrect reduction result.
    #[test]
    fn weak_spectral_peak_abstains_before_displacement() -> Result<(), String> {
        const SIDE: usize = 64;
        // The interleaved real slot for column 61 (wrapped to -3), row 8.
        const REAL_PEAK_SLOT: usize = 1146;
        let extent = Extent::new(192, 80)?;
        let mut surface = vec![0.0_f32; 8192];
        // Five percent of the 4096-point normalization area is 204.8.
        for (strength, expected) in [(204.75, None), (204.875, Some([-9.0_f32, 10.0_f32]))] {
            *surface
                .get_mut(REAL_PEAK_SLOT)
                .ok_or("missing fixture peak slot")? = strength;
            let observed = dominant_peak(&surface, SIDE, extent)?;
            if observed.map(|shift| shift.map(f32::to_bits))
                != expected.map(|shift| shift.map(f32::to_bits))
            {
                return Err(format!(
                    "spectral peak {strength} returned {observed:?}, expected {expected:?}"
                ));
            }
        }
        Ok(())
    }
}
