//! Independent oracles for the shared CPU and physical-GPU fusion kernels.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cubecl::logging::{LaunchObservation, LaunchObserver};
use pancake::fusion::advanced::{AdvancedAccumulator, AdvancedMethod};

/// Width of the odd-sized fusion test scene.
pub const WIDTH: usize = 33;
/// Height of the odd-sized fusion test scene.
pub const HEIGHT: usize = 19;
/// Maximum logical bytes for the small fusion request.
pub const BUDGET: usize = 1 << 20;

/// Make odd-sized, non-power-of-two inputs with contrasting focus planes.
fn focus_frame(left_sharp: bool) -> Vec<f32> {
    let mut image = Vec::new();
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let sharp = (x < WIDTH / 2) == left_sharp;
            let sample = if !sharp {
                0.5_f32
            } else if x % 2 == y % 2 {
                0.22_f32
            } else {
                0.78_f32
            };
            image.extend([sample; 3]);
        }
    }
    image
}

/// Fuse a stream with the same public request API as application jobs.
///
/// # Errors
/// Returns an input, resource or execution failure.
fn fuse(
    device: &cubecl::Device,
    method: AdvancedMethod,
    frames: &[Vec<f32>],
) -> Result<Vec<f32>, String> {
    let mut accumulator = AdvancedAccumulator::new(device, WIDTH, HEIGHT, method, 3, 4, BUDGET)?;
    for frame in frames {
        accumulator.add_frame(frame)?;
    }
    accumulator.finish()
}

/// Focus detail survives on both sides, and on the input's original pixel grid.
///
/// # Errors
/// Returns fusion runtime or shape errors.
pub fn check_focus(device: &cubecl::Device) -> Result<(), String> {
    let frames = [focus_frame(true), focus_frame(false)];
    for method in [AdvancedMethod::Laplacian, AdvancedMethod::Hybrid] {
        let fused = fuse(device, method, &frames)?;
        if fused.len() != WIDTH * HEIGHT * 3 {
            return Err("odd-size output shape changed".to_owned());
        }
        let (rows, _) = fused.as_chunks::<{ WIDTH * 3 }>();
        for (y, row) in rows.iter().enumerate() {
            let (pixels, _) = row.as_chunks::<3>();
            for (x, rgb) in pixels.iter().enumerate() {
                // Exclude the border and the transition between depth planes.
                if !(3..14).contains(&x) && !(19..30).contains(&x) {
                    continue;
                }
                let truth = if x % 2 == y % 2 { 0.22_f32 } else { 0.78_f32 };
                if rgb.iter().any(|sample| (sample - truth).abs() > 0.045) {
                    return Err(format!(
                        "{method:?} blurred a focus plane at ({x}, {y}): {rgb:?}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// The last one-frame exposure batch has the same final weight as a full batch.
///
/// # Errors
/// Returns fusion runtime or shape errors.
pub fn check_exposure_batches(device: &cubecl::Device) -> Result<(), String> {
    let frames: Vec<_> = [0.1, 0.2, 0.3, 0.4, 0.2, 0.3, 0.4, 0.5, 0.9_f32]
        .into_iter()
        .map(|sample| vec![sample; WIDTH * HEIGHT * 3])
        .collect();
    let fused = fuse(device, AdvancedMethod::Exposure, &frames)?;
    if fused.len() != WIDTH * HEIGHT * 3 {
        return Err("exposure dimensions changed".to_owned());
    }
    // Average of 0.25, 0.35, and 0.9 is 0.5. A global average is about 0.367.
    if fused
        .iter()
        .any(|sample| (sample - 0.5).abs() > 1.0 / 65_535.0)
    {
        return Err("exposure batches did not contribute equally".to_owned());
    }
    // One-pixel grids never allocate a finer reconstruction. The batch
    // sum must still own separate storage from the current coefficient.
    let mut one = AdvancedAccumulator::new(device, 1, 1, AdvancedMethod::Exposure, 3, 4, BUDGET)?;
    for sample in [0.1_f32, 0.2, 0.3, 0.4, 0.2, 0.3, 0.4, 0.5, 0.9] {
        one.add_frame(&[sample; 3])?;
    }
    let single_pixel = one.finish()?;
    if single_pixel.len() != 3
        || single_pixel
            .iter()
            .any(|value| (value - 0.5).abs() > 1.0 / 65_535.0)
    {
        return Err(format!(
            "smallest exposure pyramid aliased its batch sum: {single_pixel:?}"
        ));
    }
    Ok(())
}

/// Exact odd pyramid reconstruction preserves identity within one u16 code.
///
/// # Errors
/// Returns fusion runtime or shape errors.
pub fn check_exposure_precision(device: &cubecl::Device) -> Result<(), String> {
    let original: Vec<_> = (1001..)
        .take(WIDTH * HEIGHT * 3)
        .map(|sample: u16| f32::from(sample) / 65_535.0)
        .collect();
    let frames = vec![original.clone(); 4];
    let fused = fuse(device, AdvancedMethod::Exposure, &frames)?;
    if fused.len() != original.len() {
        return Err("gradient export changed shape".to_owned());
    }
    if fused
        .iter()
        .zip(original)
        .any(|(actual, expected)| (actual - expected).abs() > 1.0 / 65_535.0)
    {
        return Err("sixteen-bit gradient lost precision".to_owned());
    }
    Ok(())
}

/// Compare a single level with progressively deeper reconstruction.
///
/// # Errors
/// Returns a launch error or the first nonconstant sample.
pub fn check_constant(device: &cubecl::Device) -> Result<(), String> {
    for method in [
        AdvancedMethod::Laplacian,
        AdvancedMethod::Hybrid,
        AdvancedMethod::Exposure,
    ] {
        for [width, height] in [[1_usize, 1], [3, 3], [33, 19]] {
            for depth in 0..=4 {
                let input: Vec<f32> = std::iter::repeat_n(
                    [0.25_f32, 0.5, 0.75],
                    width.checked_mul(height).ok_or("fixture area overflow")?,
                )
                .flatten()
                .collect();
                let mut accumulator =
                    AdvancedAccumulator::new(device, width, height, method, 3, depth, 1 << 20)?;
                accumulator.add_frame(&input)?;
                let actual = accumulator.finish()?;
                if let Some((index, (sample, expected))) = actual
                    .iter()
                    .zip(&input)
                    .enumerate()
                    .find(|(_, (sample, expected))| (*sample - *expected).abs() > 0.000_01)
                {
                    return Err(format!(
                        "{method:?} {width}x{height} depth {depth} at {index}: got {sample} wanted {expected}"
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Count submitted kernels during one numerical job without adding synchronization.
#[derive(Default)]
struct LaunchCount(AtomicUsize);

impl LaunchObserver for LaunchCount {
    fn launched(&self, _kernel: &'static str) {
        let _previous = self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// Keep the earlier RGB coefficient for zero and equal positive focus.
///
/// # Errors
/// Returns a launch, layout, or numerical failure.
pub fn check_hybrid_level_bounds(device: &cubecl::Device) -> Result<(), String> {
    for (width, height, depth) in [(1_usize, 1, 0), (33, 1, 1), (1, 33, 1), (33, 19, 4)] {
        let mut accumulator = AdvancedAccumulator::new(
            device,
            width,
            height,
            AdvancedMethod::Hybrid,
            6,
            depth,
            BUDGET,
        )?;
        let area = width.checked_mul(height).ok_or("fixture area overflow")?;
        let first = [0.25_f32, 0.5, 0.75].repeat(area);
        let second = [0.75_f32, 0.5, 0.25].repeat(area);
        accumulator.add_frame(&first)?;
        accumulator.add_frame(&second)?;
        let actual = accumulator.finish()?;
        if actual.len() != first.len()
            || actual
                .iter()
                .zip(first)
                .any(|(sample, expected)| sample.to_bits() != expected.to_bits())
        {
            return Err(format!(
                "Hybrid changed a zero-weight winner or the {width}×{height} depth-{depth} extent"
            ));
        }
    }

    let mut accumulator =
        AdvancedAccumulator::new(device, WIDTH, 1, AdvancedMethod::Hybrid, 6, 0, BUDGET)?;
    for parity in 0..2 {
        let frame: Vec<_> = (0..WIDTH)
            .flat_map(|x| [if x % 2 == parity { 0.0_f32 } else { 1.0 }; 3])
            .collect();
        accumulator.add_frame(&frame)?;
    }
    let actual = accumulator.finish()?;
    let expected = (0..WIDTH).flat_map(|x| [if x % 2 == 0 { 0.3_f32 } else { 0.7 }; 3]);
    // In the top level both inputs have equal positive contrast and their
    // mean, 0.5, gets 60% of the final coefficient.
    if actual.len() != WIDTH * 3
        || actual
            .iter()
            .zip(expected)
            .any(|(sample, expected)| sample.to_bits() != expected.to_bits())
    {
        return Err("Hybrid changed the earlier winner at equal positive focus".to_owned());
    }
    Ok(())
}

/// Avoid a separate band materialization pass for Hybrid coefficients.
///
/// # Errors
/// Returns a launch, shape, or numerical failure.
pub fn check_hybrid_frame_launches(device: &cubecl::Device) -> Result<(), String> {
    let mut accumulator =
        AdvancedAccumulator::new(device, WIDTH, HEIGHT, AdvancedMethod::Hybrid, 6, 4, BUDGET)?;
    let launches = Arc::new(LaunchCount::default());
    let observing = LaunchObservation::new(launches.clone());
    accumulator.add_frame(&focus_frame(true))?;
    accumulator.add_frame(&focus_frame(false))?;
    drop(observing);
    let output = accumulator.finish()?;
    if output.len() != WIDTH * HEIGHT * 3 || output.iter().any(|sample| !sample.is_finite()) {
        return Err("Hybrid launch witness did not complete a finite RGB output".to_owned());
    }
    // Per frame: three focus passes, two reductions per step, ten band updates.
    let issued = launches.0.load(Ordering::Relaxed);
    if issued > 42 {
        return Err(format!(
            "two four-reduction Hybrid frames should need at most 42 launches, issued {issued}"
        ));
    }
    Ok(())
}

/// A change of raster size alone should reuse the same compiled exposure kernels.
///
/// # Errors
/// Returns an execution failure or differing `CubeCL` kernel identities.
pub fn check_exposure_specialization(device: &cubecl::Device) -> Result<(), String> {
    use cubecl::launched::LaunchedKernels;

    let run = |width: usize, height: usize| {
        let collection = LaunchedKernels::new();
        let launches = Arc::new(LaunchCount::default());
        let observing = LaunchObservation::new(launches.clone());
        let mut accumulator = AdvancedAccumulator::new(
            device,
            width,
            height,
            AdvancedMethod::Exposure,
            6,
            4,
            1 << 23,
        )?;
        let samples = width
            .checked_mul(height)
            .and_then(|count| count.checked_mul(3))
            .ok_or("specialization test image exceeds addressing")?;
        accumulator.add_frame(&vec![0.3_f32; samples])?;
        accumulator.add_frame(&vec![0.6_f32; samples])?;
        let image = accumulator.finish()?;
        if image.len() != samples || image.iter().any(|sample| (sample - 0.45_f32).abs() > 1e-6) {
            return Err("the specialization test did not complete exposure fusion".to_owned());
        }
        drop(observing);
        let issued = launches.0.load(Ordering::Relaxed);
        if issued > 39 {
            return Err(format!(
                "two four-reduction exposures should need at most 39 launches, issued {issued}"
            ));
        }
        Ok(collection.finish())
    };
    let small = run(33, 19)?;
    let large = run(193, 129)?;
    if small.is_empty() || small != large {
        return Err(format!(
            "image sizes require different exposure kernel keys: small {}, large {}, common {}",
            small.len(),
            large.len(),
            small.intersection(&large).count()
        ));
    }
    Ok(())
}
