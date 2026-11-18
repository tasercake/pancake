//! Bounded subpixel registration with a conservative confidence decision.

use super::{
    geometry::{Affine, Extent, Frame},
    kernels::{Pair, make_luma},
};

/// Longest side of an estimation thumbnail, independent of camera resolution.
const THUMBNAIL_LIMIT: u16 = 384;
/// Minimum independent row/column structure for an identifiable fit.
const MIN_SIDE: u16 = 12;
/// Sampled pixel spacing during wide integer translation search.
const SEARCH_STRIDE: u16 = 4;
/// Conservative minimum aligned correlation for structure to agree.
const MIN_CORRELATION: f32 = 0.75;
/// Fraction of unregistered detail error that the fit must remove.
const MIN_ERROR_REDUCTION: f32 = 0.35;

/// Allowed geometric degrees of freedom in a pair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Model {
    /// Independent horizontal and vertical translation.
    Translation,
    /// Translation and rotation, with unit scale.
    Euclidean,
    /// Translation, rotation and isotropic scale.
    Similarity,
    /// Translation and a full linear map, including shear.
    Affine,
    /// Translation and bounded isotropic scale.
    Breathing,
}

impl Model {
    /// Number of active optimization parameters.
    const fn parameters(self) -> usize {
        match self {
            Self::Translation => 2,
            Self::Euclidean | Self::Breathing => 3,
            Self::Similarity => 4,
            Self::Affine => 6,
        }
    }
}

/// Compact luma proxy of a source normalized to the original first-frame canvas.
#[derive(Clone, Debug)]
pub struct Thumbnail {
    /// Coordinate extent used for estimating a pair.
    extent: Extent,
    /// Original common canvas before any estimation downsampling.
    canvas: Extent,
    /// Row-major luma samples produced by the shared device kernel.
    pixels: Vec<f32>,
}

/// A common-canvas map and evidence for accepting its motion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    /// Reference-to-source map in coordinates of the common canvas.
    pub transform: Affine,
    /// Product of achieved correlation and fractional error reduction; zero
    /// confirms an ambiguous or featureless identity fallback.
    pub confidence: f32,
}

impl Estimate {
    /// Ambiguous or unsupported geometry leaves coordinates unchanged.
    const UNCERTAIN: Self = Self {
        transform: Affine::IDENTITY,
        confidence: 0.0,
    };
}

/// Downsample and smooth the registration texture before comparing a pair.
///
/// Frame dimensions may differ; thumbnail coordinates still refer to the
/// supplied first-frame canvas. Prepared proxies avoid retaining full decoded
/// source images throughout sequential or middle-reference estimation.
///
/// # Errors
///
/// Returns an error for bad dimensions, insufficient memory or device readback.
/// The caller must supply a device whose runtime is linked and available.
pub fn thumbnail(
    device: &cubecl::Device,
    frame: Frame<'_>,
    reference_width: usize,
    reference_height: usize,
    budget: usize,
) -> Result<Thumbnail, String> {
    let canvas = Extent::new(reference_width, reference_height)?;
    prepare_thumbnail(device, frame, canvas, u32::from(THUMBNAIL_LIMIT), budget)
}

/// Prepare a luma proxy using an explicit reduction factor on large inputs.
///
/// Native small images retain their resolution. Larger images allow a
/// configurable reduction while reserving enough texture for reliable matching.
///
/// # Errors
///
/// Returns an error for invalid dimensions or factor, insufficient memory,
/// or a device readback error.
pub fn thumbnail_with_scale(
    device: &cubecl::Device,
    frame: Frame<'_>,
    reference_width: usize,
    reference_height: usize,
    scale_factor: u32,
    budget: usize,
) -> Result<Thumbnail, String> {
    let canvas = Extent::new(reference_width, reference_height)?;
    let longest = u32::from(canvas.width.max(canvas.height));
    let requested = longest
        .checked_div(scale_factor)
        .ok_or("alignment scale factor must be positive")?;
    let side = requested.clamp(u32::from(THUMBNAIL_LIMIT), 768);
    prepare_thumbnail(device, frame, canvas, side, budget)
}

/// Apply an aspect-preserving size limit and prepare the device luma proxy.
///
/// # Errors
///
/// Returns a shape, budget or readback error.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "u16 sizes and a <=768 factor bound the u32 rounding calculation"
)]
fn prepare_thumbnail(
    device: &cubecl::Device,
    frame: Frame<'_>,
    canvas: Extent,
    limit: u32,
    budget: usize,
) -> Result<Thumbnail, String> {
    let longest = u32::from(canvas.width.max(canvas.height));
    let factor = limit.min(longest);
    let shrink = |side: u16| -> Result<usize, String> {
        let size = (u32::from(side) * factor + longest / 2) / longest;
        usize::try_from(size.max(1)).map_err(|error| error.to_string())
    };
    let extent = Extent::new(shrink(canvas.width)?, shrink(canvas.height)?)?;
    let pixels = make_luma(device, frame, extent, budget)?;
    Ok(Thumbnail {
        extent,
        canvas,
        pixels,
    })
}

/// Estimate the reference-to-source map from one independent pair of proxies.
///
/// Wide integer matching seeds subpixel correlation refinement. Scoring
/// normalizes gain and bias and compares a fixed interior region. A proposed
/// transform must be plausible and explain enough detail error to beat identity.
/// Large unsupported scale or rotation, repeated peaks and focus-only changes
/// therefore leave an explicit low-confidence identity. The returned matrix
/// is lifted from thumbnail back to the common full-resolution canvas.
///
/// # Errors
///
/// Returns an error for incompatible canvases, invalid limits, device/readback
/// failures, or invalid estimated geometry.
pub fn estimate_pair(
    device: &cubecl::Device,
    reference: &Thumbnail,
    source: &Thumbnail,
    model: Model,
    max_scale_step: f32,
    max_precision: bool,
) -> Result<Estimate, String> {
    if reference.extent != source.extent || reference.canvas != source.canvas {
        return Err("registration thumbnails must share the same canvas".to_owned());
    }
    if !max_scale_step.is_finite() || max_scale_step <= 0.0 || max_scale_step >= 1.0 {
        return Err("max scale step must be finite and between zero and one".to_owned());
    }
    let extent = reference.extent;
    if extent.width.min(extent.height) < MIN_SIDE {
        return Ok(Estimate::UNCERTAIN);
    }
    let pair = Pair::new(device, &reference.pixels, &source.pixels, extent);
    let stride = if max_precision { 1 } else { 2 };
    let baseline = one(&pair.correlation(&[Affine::IDENTITY], stride)?)?;
    if baseline >= 0.9998 {
        return Ok(Estimate::UNCERTAIN);
    }
    let coordinates = Coordinates::new(extent);
    let (translation, ambiguous_peak) = seed(&pair, coordinates)?;
    if ambiguous_peak {
        return Ok(Estimate::UNCERTAIN);
    }
    let search = Refiner {
        pair: &pair,
        coordinates,
        model,
        stride,
    };
    let (parameters, score) = search.run(translation, baseline, max_precision)?;
    let improvement = score - baseline;
    let removed = improvement / (1.0 - baseline);
    if score < MIN_CORRELATION || improvement < 0.006 || removed < MIN_ERROR_REDUCTION {
        return Ok(Estimate::UNCERTAIN);
    }
    let map = coordinates.convert(model, parameters)?;
    if !is_plausible(map, coordinates.span) {
        return Ok(Estimate::UNCERTAIN);
    }
    let Parameters([dx, dy, gain, next, fifth, sixth]) = parameters;
    let chosen = if model == Model::Breathing {
        let limit = max_scale_step * coordinates.span;
        coordinates.convert(
            model,
            Parameters([dx, dy, gain.clamp(-limit, limit), next, fifth, sixth]),
        )?
    } else {
        map
    };
    let full = [
        usize::from(source.canvas.width),
        usize::from(source.canvas.height),
    ];
    let small = [usize::from(extent.width), usize::from(extent.height)];
    let down = Affine::IDENTITY.rescale_source(full, small)?;
    let up = Affine::IDENTITY.rescale_source(small, full)?;
    Ok(Estimate {
        transform: up.compose(chosen)?.compose(down)?,
        confidence: score * removed,
    })
}

/// Parameters express center displacement and edge movement in pixels.
#[derive(Clone, Copy, Debug, Default)]
struct Parameters([f32; 6]);

/// Constrain rotation and scaling around the canvas center.
#[derive(Clone, Copy, Debug)]
struct Coordinates {
    /// Center pixel x-coordinate.
    x: f32,
    /// Center pixel y-coordinate.
    y: f32,
    /// Normalize scale/shear parameters to roughly one pixel of edge motion.
    span: f32,
}

impl Coordinates {
    /// Prepare symmetric coordinates from checked dimensions.
    fn new(extent: Extent) -> Self {
        let x = 0.5_f32.mul_add(f32::from(extent.width), -0.5);
        let y = 0.5_f32.mul_add(f32::from(extent.height), -0.5);
        Self {
            x,
            y,
            span: x.max(y).max(1.0),
        }
    }

    /// Convert one candidate for correlation scoring or final publication.
    ///
    /// # Errors
    ///
    /// Returns an error for nonfinite coefficients.
    fn convert(self, model: Model, parameters: Parameters) -> Result<Affine, String> {
        let Parameters([dx, dy, scale, second, third, fourth]) = parameters;
        let increase = scale / self.span;
        let slope = second / self.span;
        let [[da, b], [c, dd]] = match model {
            Model::Translation => [[0.0, 0.0], [0.0, 0.0]],
            Model::Euclidean => {
                let (s, cos) = increase.sin_cos();
                [[cos - 1.0, -s], [s, cos - 1.0]]
            }
            Model::Similarity => [[increase, -slope], [slope, increase]],
            Model::Breathing => [[increase, 0.0], [0.0, increase]],
            Model::Affine => [[increase, slope], [third / self.span, fourth / self.span]],
        };
        let tx = (-da).mul_add(self.x, (-b).mul_add(self.y, dx));
        let ty = (-c).mul_add(self.x, (-dd).mul_add(self.y, dy));
        Affine::from_matrix([[1.0 + da, b, tx], [c, 1.0 + dd, ty]])
    }
}

/// Find a bounded integer seed and reject widely separated near-equal peaks.
///
/// # Errors
///
/// Returns a device error or an empty search result.
fn seed(pair: &Pair, coords: Coordinates) -> Result<(Parameters, bool), String> {
    let step = (coords.span / 32.0).round().clamp(3.0, 6.0);
    let mut trials = Vec::new();
    for y in -4_i16..=4 {
        for x in -4_i16..=4 {
            trials.push(Parameters([
                f32::from(x) * step,
                f32::from(y) * step,
                0.0,
                0.0,
                0.0,
                0.0,
            ]));
        }
    }
    let maps = trials
        .iter()
        .map(|&p| coords.convert(Model::Translation, p))
        .collect::<Result<Vec<_>, _>>()?;
    let scores = pair.correlation(&maps, SEARCH_STRIDE)?;
    let (&chosen, &score) = trials
        .iter()
        .zip(&scores)
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .ok_or("registration search returned no candidates")?;
    let Parameters([x, y, ..]) = chosen;
    let ambiguous = score >= MIN_CORRELATION
        && trials
            .iter()
            .zip(&scores)
            .any(|(&Parameters([u, v, ..]), &other)| {
                ((x - u).abs() > 2.0 * step || (y - v).abs() > 2.0 * step) && score - other < 0.001
            });
    Ok((chosen, ambiguous))
}

/// Numeric context shared across refinement levels.
struct Refiner<'a> {
    /// Prepared device luma and scoring operations.
    pair: &'a Pair,
    /// Center and scale used for geometric constraints.
    coordinates: Coordinates,
    /// Active model coefficients.
    model: Model,
    /// Pixel sampling rate used for subpixel refinement.
    stride: u16,
}

impl Refiner<'_> {
    /// Map and evaluate a batch of refinement candidates.
    ///
    /// # Errors
    ///
    /// Returns a map error or device scoring failure.
    fn scores(&self, candidates: &[Parameters]) -> Result<Vec<f32>, String> {
        let maps = candidates
            .iter()
            .map(|&p| self.coordinates.convert(self.model, p))
            .collect::<Result<Vec<_>, _>>()?;
        self.pair.correlation(&maps, self.stride)
    }

    /// Refine translation and model coefficients at progressively finer steps.
    ///
    /// # Errors
    ///
    /// Returns a geometry or device scoring error.
    fn run(
        &self,
        seed: Parameters,
        baseline: f32,
        max_precision: bool,
    ) -> Result<(Parameters, f32), String> {
        let mut score = one(&self.scores(&[seed])?)?;
        let mut parameters = seed;
        if score < baseline {
            parameters = Parameters::default();
            score = baseline;
        }
        let levels = [4.0, 2.0, 1.0, 0.5, 0.25, 0.125];
        let level_count = if max_precision { 6_usize } else { 5_usize };
        for step in levels.into_iter().take(level_count) {
            for _ in 0..7_usize {
                let Some((next, next_score)) = self.improve(parameters, score, step)? else {
                    break;
                };
                parameters = next;
                score = next_score;
            }
        }
        Ok((parameters, score))
    }

    /// Probe each active axis and try a combined bounded quadratic step.
    ///
    /// # Errors
    ///
    /// Returns a bad-model or device scoring error.
    fn improve(
        &self,
        current: Parameters,
        score: f32,
        step: f32,
    ) -> Result<Option<(Parameters, f32)>, String> {
        let mut probes = Vec::new();
        for axis in 0..self.model.parameters() {
            for direction in [-1.0_f32, 1.0] {
                let mut trial = current;
                let value = trial
                    .0
                    .get_mut(axis)
                    .ok_or("registration model parameter is missing")?;
                *value = direction.mul_add(step, *value);
                probes.push(trial);
            }
        }
        let neighbors = self.scores(&probes)?;
        let (pairs, _) = neighbors.as_chunks::<2>();
        let fraction = |&[left, right]: &[f32; 2]| {
            let curvature = 2.0_f32.mul_add(score, -(left + right));
            if curvature > 1e-6 {
                ((right - left) / (2.0_f32 * curvature)).clamp(-1.0, 1.0)
            } else if right > left {
                1.0
            } else {
                -1.0
            }
        };
        let mut combined = current;
        let mut half = current;
        for ((full, damped), displacement) in combined
            .0
            .iter_mut()
            .zip(&mut half.0)
            .zip(pairs.iter().map(fraction))
        {
            *full = step.mul_add(displacement, *full);
            *damped = (0.5 * step).mul_add(displacement, *damped);
        }
        let refinement = [combined, half];
        let refined_scores = self.scores(&refinement)?;
        Ok(probes
            .into_iter()
            .zip(neighbors)
            .chain(refinement.into_iter().zip(refined_scores))
            .filter(|&(_, next_score)| next_score > score + 1e-6)
            .max_by(|(_, a), (_, b)| a.total_cmp(b)))
    }
}

/// Confirm transform scale, orthogonality, skew, rotation and center drift.
fn is_plausible(map: Affine, radius: f32) -> bool {
    let [[xx, xy, tx], [yx, yy, ty]] = map.matrix();
    let sx = xx.hypot(yx);
    let sy = xy.hypot(yy);
    let determinant = xx.mul_add(yy, -(xy * yx));
    let cosine = xx.mul_add(xy, yx * yy) / (sx * sy);
    (0.82..=1.2).contains(&sx)
        && (0.82..=1.2).contains(&sy)
        && (0.85..=1.18).contains(&(sx / sy))
        && cosine.abs() <= 0.15
        && (0.7..=1.45).contains(&determinant)
        && xy.abs().max(yx.abs()) <= 0.195
        && tx.abs().max(ty.abs()) <= radius * 0.65
}

/// Require exactly one score for a single-map evaluation.
///
/// # Errors
///
/// Returns an error if no single score was returned.
fn one(scores: &[f32]) -> Result<f32, String> {
    let &[score] = scores else {
        return Err("registration did not return one candidate score".to_owned());
    };
    Ok(score)
}
