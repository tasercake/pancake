use rayon::prelude::*;
use serde_json::Value;

use crate::pool::{OverlapDiagnostics, pool_overlap};
use crate::{accelerator, parallel, pixels};

pub(crate) struct Measurement {
    pub(crate) features: Value,
    pub(crate) diagnostics: Value,
    pub(crate) explanation: Option<Value>,
    pub(crate) score: f64,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn measure(
    candidate: &[f32],
    target: &[f32],
    height: usize,
    width: usize,
    support: Option<&[u8]>,
    exact_box: Option<[usize; 4]>,
    heat_weights: Option<(&[f32], f64)>,
    details: Option<&str>,
    device: &str,
) -> Result<Measurement, String> {
    let area = height
        .checked_mul(width)
        .filter(|&area| area > 0)
        .ok_or("Expected a nonempty overlap grid")?;
    let sample_count = area.checked_mul(3).ok_or("Overlap grid is too large")?;
    if target.len() != sample_count
        || (exact_box.is_none() && candidate.len() != sample_count)
        || support.is_some_and(|mask| mask.len() != area)
        || heat_weights.is_some_and(|(weights, _)| !weights.is_empty() && weights.len() != area)
    {
        return Err("Image, support, or inspection data do not match the overlap grid".into());
    }

    let inspection_mix = heat_weights.map(|(_, mix)| mix);
    let mut maps = None;
    let mut diagnostics = OverlapDiagnostics::default();
    if let Some([top, left, bottom, right]) = exact_box {
        if top >= bottom || left >= right || bottom > height || right > width {
            return Err("The exact overlap box is outside the reference grid".into());
        }
    } else {
        if support.is_some_and(|mask| !mask.iter().any(|&value| value != 0)) {
            return Err("No covered reference pixels".into());
        }
        let compare = || {
            let pixels = pixels::compare(candidate, target, support);
            let [cl, tl, delta, color, range] = pixels.planes;
            let [tone, lost, extra, boundary] =
                accelerator::lightness(device, height, width, &cl, &tl, &delta, support)?;
            Ok::<_, String>((
                [tone, color, lost, extra, boundary, range],
                pixels.mean_delta,
            ))
        };
        let rgb = || pixels::rgb_diagnostics(height, width, candidate, target, support);
        let (comparison, (rmse, exposure)) = if area >= 65_536 {
            parallel(|| rayon::join(compare, rgb))
        } else {
            (compare(), rgb())
        };
        let (mut planes, [dl, da, db]) = comparison?;
        if let Some((weights, _)) = heat_weights.filter(|(weights, _)| !weights.is_empty()) {
            let apply = |plane: &mut Vec<f32>| {
                for (damage, &weight) in plane.iter_mut().zip(weights) {
                    *damage *= weight;
                }
            };
            if area >= 65_536 {
                parallel(|| planes.par_iter_mut().for_each(apply));
            } else {
                planes.iter_mut().for_each(apply);
            }
        }
        maps = Some(planes);
        diagnostics = OverlapDiagnostics {
            mean_delta_lightness: dl,
            mean_delta_lab_a: da,
            mean_delta_lab_b: db,
            global_exposure_delta_stops: exposure,
            linear_rgb_rmse: rmse,
        };
    }
    let result = pool_overlap(
        height,
        width,
        maps.as_ref(),
        support,
        exact_box,
        inspection_mix,
        details,
    )
    .ok_or("No covered reference tiles")?;
    let (features, diagnostics, explanation) = result.describe(diagnostics);
    Ok(Measurement {
        features,
        diagnostics,
        explanation,
        score: result.score,
    })
}
