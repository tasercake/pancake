//! Per-level selection and weighted accumulation for multiscale fusion.

use cubecl::prelude::*;
use cubecl::server::Handle;
use cubecl::std::tensor::TensorHandle;

use super::super::buffers::{ImageShape, empty};
use super::AdvancedMethod;
use super::elementwise;
use super::pyramid::expanded;

/// Persistent statistics for Hybrid's winner-versus-soft-weight blend.
#[derive(Debug)]
struct HybridSelection {
    /// Sharpest RGB coefficient encountered at each pixel.
    coefficients: TensorHandle,
    /// Weight of that selected coefficient.
    scores: TensorHandle,
}

/// Resident output and scores for one level of a streaming pyramid.
#[derive(Debug)]
pub(super) struct Level {
    /// Dimensions of the RGB and scalar surfaces.
    pub(super) grid: ImageShape,
    /// Laplacian selection or weighted sum of RGB bands.
    primary: TensorHandle,
    /// Winning Laplacian scores or the sum of the soft weights.
    scores: TensorHandle,
    /// Additional winner tracked by Hybrid.
    hybrid: Option<HybridSelection>,
}

impl Level {
    /// Allocate fixed-size device state for one level.
    pub(super) fn new(client: &Client, grid: ImageShape, method: AdvancedMethod) -> Self {
        let hybrid = (method == AdvancedMethod::Hybrid).then(|| HybridSelection {
            coefficients: empty(client, grid.samples),
            scores: empty(client, grid.area),
        });
        Self {
            grid,
            primary: empty(client, grid.samples),
            scores: empty(client, grid.area),
            hybrid,
        }
    }

    /// Return every live output handle that the current frame may have changed.
    pub(super) fn outputs(&self) -> impl Iterator<Item = &Handle> {
        [&self.primary.handle, &self.scores.handle]
            .into_iter()
            .chain(
                self.hybrid
                    .iter()
                    .flat_map(|hybrid| [&hybrid.coefficients.handle, &hybrid.scores.handle]),
            )
    }

    /// Accumulate new coefficients beginning with a completely defined first frame.
    pub(super) fn accumulate(
        &self,
        client: &Client,
        fine: &TensorHandle,
        coarse: Option<(&TensorHandle, ImageShape)>,
        weights: &TensorHandle,
        method: AdvancedMethod,
        first: bool,
    ) {
        let (blocks, threads) = elementwise(client, self.grid.area);
        if method == AdvancedMethod::Laplacian {
            let band = self.band(client, fine, coarse);
            select::launch(
                client,
                blocks,
                threads,
                band.into_arg(),
                weights.clone().into_arg(),
                self.primary.clone().into_arg(),
                self.scores.clone().into_arg(),
                u32::from(first),
            );
            return;
        }

        // Weighted methods form the band inside accumulation. A Hybrid winner
        // computes the coefficient again only where it replaces a previous one.
        let (lower, lower_width, lower_height) = coarse.map_or((fine, 0, 0), |(lower, grid)| {
            (lower, grid.width, grid.height)
        });
        add_weighted::launch(
            client,
            blocks.clone(),
            threads,
            fine.clone().into_arg(),
            lower.clone().into_arg(),
            weights.clone().into_arg(),
            self.primary.clone().into_arg(),
            self.scores.clone().into_arg(),
            u32::from(first),
            self.grid.width,
            lower_width,
            lower_height,
        );
        if let Some(hybrid) = &self.hybrid {
            select_hybrid::launch(
                client,
                blocks,
                threads,
                fine.clone().into_arg(),
                lower.clone().into_arg(),
                weights.clone().into_arg(),
                hybrid.coefficients.clone().into_arg(),
                hybrid.scores.clone().into_arg(),
                u32::from(first),
                self.grid.width,
                lower_width,
                lower_height,
            );
        }
    }

    /// Return the top RGB level or the difference from its expanded lower grid.
    fn band(
        &self,
        client: &Client,
        fine: &TensorHandle,
        coarse: Option<(&TensorHandle, ImageShape)>,
    ) -> TensorHandle {
        let Some((coarse, grid)) = coarse else {
            return fine.clone();
        };
        let band = empty(client, self.grid.samples);
        let (blocks, threads) = elementwise(client, self.grid.samples);
        subtract_coarse::launch(
            client,
            blocks,
            threads,
            fine.clone().into_arg(),
            coarse.clone().into_arg(),
            band.clone().into_arg(),
            self.grid.width,
            grid.width,
            grid.height,
        );
        band
    }

    /// Finish one band; the next batch overwrites the resulting device buffers.
    pub(super) fn finish(
        &self,
        client: &Client,
        method: AdvancedMethod,
        fine_detail: bool,
    ) -> TensorHandle {
        let (blocks, threads) = elementwise(client, self.grid.samples);
        match method {
            AdvancedMethod::Laplacian => {}
            AdvancedMethod::Hybrid => {
                if let Some(hybrid) = &self.hybrid {
                    normalize_hybrid::launch(
                        client,
                        blocks,
                        threads,
                        self.primary.clone().into_arg(),
                        self.scores.clone().into_arg(),
                        hybrid.coefficients.clone().into_arg(),
                        u32::from(fine_detail),
                    );
                }
            }
            AdvancedMethod::Exposure => {
                normalize::launch(
                    client,
                    blocks,
                    threads,
                    self.primary.clone().into_arg(),
                    self.scores.clone().into_arg(),
                );
            }
        }
        self.primary.clone()
    }
}

/// Subtract exactly the same expansion used for reconstruction.
#[cube(launch, expand_only)]
fn subtract_coarse(
    fine: &Tensor<f32>,
    coarse: &Tensor<f32>,
    band: &mut Tensor<f32>,
    width: usize,
    coarse_width: usize,
    coarse_height: usize,
) {
    let sample = ABSOLUTE_POS;
    if sample >= band.len() {
        terminate!();
    }
    let x = (sample / 3) % width;
    let y = (sample / 3) / width;
    band[sample] = fine[sample] - expanded(coarse, x, y, sample % 3, coarse_width, coarse_height);
}

/// Use a strict comparison to keep the earlier coefficient on ties.
#[cube(launch, expand_only)]
fn select(
    band: &Tensor<f32>,
    weight: &Tensor<f32>,
    selected: &mut Tensor<f32>,
    scores: &mut Tensor<f32>,
    first: u32,
) {
    let pixel = ABSOLUTE_POS;
    if pixel >= scores.len() {
        terminate!();
    }
    if first != 0 || weight[pixel] > scores[pixel] {
        scores[pixel] = weight[pixel];
        let address = pixel * 3;
        selected[address] = band[address];
        selected[address + 1] = band[address + 1];
        selected[address + 2] = band[address + 2];
    }
}

/// Derive a Hybrid band coefficient only for a new strict focus winner.
#[cube(launch, expand_only)]
fn select_hybrid(
    fine: &Tensor<f32>,
    lower: &Tensor<f32>,
    weight: &Tensor<f32>,
    selected: &mut Tensor<f32>,
    scores: &mut Tensor<f32>,
    first: u32,
    width: usize,
    lower_width: usize,
    lower_height: usize,
) {
    let pixel = ABSOLUTE_POS;
    if pixel >= scores.len() {
        terminate!();
    }
    let score = weight[pixel];
    let mut replaces_winner = true;
    if first == 0 {
        replaces_winner = score > scores[pixel];
    }
    if replaces_winner {
        scores[pixel] = score;
        let address = pixel * 3;
        let x = pixel % width;
        let y = pixel / width;
        for channel in 0..3 {
            let sample = address + channel;
            let mut coefficient = fine[sample];
            if lower_width != 0 {
                coefficient -= expanded(lower, x, y, channel, lower_width, lower_height);
            }
            selected[sample] = coefficient;
        }
    }
}

/// Accumulate weighted coefficients, deriving a band when a lower grid exists.
///
/// A zero lower width identifies a supplied coefficient or the top Gaussian
/// level. Otherwise the lower level is expanded at the original pixel grid.
#[cube(launch, expand_only)]
fn add_weighted(
    fine: &Tensor<f32>,
    lower: &Tensor<f32>,
    weight: &Tensor<f32>,
    primary: &mut Tensor<f32>,
    scores: &mut Tensor<f32>,
    first: u32,
    width: usize,
    lower_width: usize,
    lower_height: usize,
) {
    let pixel = ABSOLUTE_POS;
    if pixel >= scores.len() {
        terminate!();
    }
    let address = pixel * 3;
    let score = weight[pixel];
    if first != 0 {
        scores[pixel] = score;
    } else {
        scores[pixel] += score;
    }

    for channel in 0..3 {
        let sample = address + channel;
        let mut coefficient = fine[sample];
        if lower_width != 0 {
            let x = pixel % width;
            let y = pixel / width;
            coefficient -= expanded(lower, x, y, channel, lower_width, lower_height);
        }
        let scaled = score * coefficient;
        if first != 0 {
            primary[sample] = scaled;
        } else {
            primary[sample] += scaled;
        }
    }
}

/// Obtain one weighted exposure band before reconstructing the batch.
#[cube(launch, expand_only)]
fn normalize(primary: &mut Tensor<f32>, scores: &Tensor<f32>) {
    let sample = ABSOLUTE_POS;
    if sample >= primary.len() {
        terminate!();
    }
    primary[sample] /= scores[sample / 3];
}

/// Preserve dominant fine detail and blend low-frequency tone more softly.
#[cube(launch, expand_only)]
fn normalize_hybrid(
    primary: &mut Tensor<f32>,
    scores: &Tensor<f32>,
    selected: &Tensor<f32>,
    fine_detail: u32,
) {
    let sample = ABSOLUTE_POS;
    if sample >= primary.len() {
        terminate!();
    }
    let best = selected[sample];
    let total = scores[sample / 3];
    let mut coefficient = best;
    if total > 0.0 {
        let blended = primary[sample] / total;
        let alpha = if fine_detail != 0 { 0.14_f32 } else { 0.6_f32 };
        coefficient += (blended - best) * alpha;
    }
    primary[sample] = coefficient;
}

/// Add a reconstructed exposure batch at equal weight to earlier batches.
#[cube(launch, expand_only)]
pub(super) fn accumulate_batch(image: &Tensor<f32>, sum: &mut Tensor<f32>, first: u32) {
    let sample = ABSOLUTE_POS;
    if sample >= sum.len() {
        terminate!();
    }
    if first != 0 {
        sum[sample] = image[sample];
    } else {
        sum[sample] += image[sample];
    }
}

/// Scale the sum of equal-weight exposure batches to their mean.
#[cube(launch, expand_only)]
pub(super) fn average_batches(sum: &mut Tensor<f32>, batches: u32) {
    let sample = ABSOLUTE_POS;
    if sample >= sum.len() {
        terminate!();
    }
    sum[sample] /= f32::cast_from(batches);
}
