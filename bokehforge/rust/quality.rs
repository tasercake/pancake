use rayon::prelude::*;
use serde_json::Value;

use crate::pool::{AFFECTED_THRESHOLD, FamilyInput, OverlapDiagnostics, pool_overlap};
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

    let mut damage = None;
    let native_weights = heat_weights
        .map(|(weights, _)| weights)
        .filter(|weights| !weights.is_empty());
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
            let plan = accelerator::lightness_plan(height, width);
            if let Some(prepared) =
                pixels::compare_prepared(candidate, target, height, width, support, &plan)
            {
                let packed = prepared
                    .tiles
                    .into_iter()
                    .map(pixels::PackedLightness::into_samples)
                    .collect();
                let lightness = accelerator::lightness_packed_cells(
                    device,
                    height,
                    width,
                    &plan,
                    packed,
                    support,
                    native_weights,
                    AFFECTED_THRESHOLD,
                )?;
                Ok::<_, String>((
                    lightness,
                    [prepared.color, prepared.range],
                    prepared.mean_delta,
                ))
            } else {
                let pixels = pixels::compare(candidate, target, support);
                let [cl, tl, delta, color, range] = pixels.planes;
                let lightness = accelerator::lightness_cells(
                    device,
                    height,
                    width,
                    [&cl, &tl, &delta],
                    support,
                    native_weights,
                    AFFECTED_THRESHOLD,
                )?;
                Ok((lightness, [color, range], pixels.mean_delta))
            }
        };
        let rgb = || pixels::rgb_diagnostics(height, width, candidate, target, support);
        let (comparison, (rmse, exposure)) = if area >= 65_536 {
            parallel(|| rayon::join(compare, rgb))
        } else {
            (compare(), rgb())
        };
        let (lightness, planes, [dl, da, db]) = comparison?;
        damage = Some((lightness, planes));
        diagnostics = OverlapDiagnostics {
            mean_delta_lightness: dl,
            mean_delta_lab_a: da,
            mean_delta_lab_b: db,
            global_exposure_delta_stops: exposure,
            linear_rgb_rmse: rmse,
        };
    }
    finish(
        [height, width],
        support,
        exact_box,
        damage,
        diagnostics,
        heat_weights,
        details,
    )
}

type Damage = ([Vec<[f32; 3]>; 4], [Vec<f32>; 2]);

#[allow(clippy::too_many_arguments)]
fn finish(
    [height, width]: [usize; 2],
    support: Option<&[u8]>,
    exact_box: Option<[usize; 4]>,
    mut damage: Option<Damage>,
    diagnostics: OverlapDiagnostics,
    heat_weights: Option<(&[f32], f64)>,
    details: Option<&str>,
) -> Result<Measurement, String> {
    let area = height * width;
    let inspection_mix = heat_weights.map(|(_, mix)| mix);
    let native_weights = heat_weights
        .map(|(weights, _)| weights)
        .filter(|weights| !weights.is_empty());
    if let (Some((_, planes)), Some(weights)) = (&mut damage, native_weights) {
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
    let maps = damage
        .as_ref()
        .map(|([tone, lost, added, boundary], [color, range])| {
            [
                FamilyInput::Cells(tone),
                FamilyInput::Pixels(color),
                FamilyInput::Cells(lost),
                FamilyInput::Cells(added),
                FamilyInput::Cells(boundary),
                FamilyInput::Pixels(range),
            ]
        });
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

pub(crate) struct DeferredQuality {
    lightness: accelerator::DeferredLightness,
    planes: [Vec<f32>; 2],
    diagnostics: OverlapDiagnostics,
}

pub(crate) fn defer(
    [height, width]: [usize; 2],
    candidate: &[f32],
    target: &[f32],
    support: Option<&[u8]>,
) -> Result<Option<DeferredQuality>, String> {
    let plan = accelerator::lightness_plan(height, width);
    let compare = || pixels::compare_prepared(candidate, target, height, width, support, &plan);
    let (prepared, (rmse, exposure)) = parallel(|| {
        rayon::join(compare, || {
            pixels::rgb_diagnostics(height, width, candidate, target, support)
        })
    });
    let Some(prepared) = prepared else {
        return Ok(None);
    };
    let packed = prepared
        .tiles
        .into_iter()
        .map(pixels::PackedLightness::into_samples)
        .collect();
    let [dl, da, db] = prepared.mean_delta;
    // The caller also finalizes the same CubeCL stream.
    let lightness = accelerator::lightness_deferred("cuda", height, width, &plan, packed, support)?;
    Ok(Some(DeferredQuality {
        lightness,
        planes: [prepared.color, prepared.range],
        diagnostics: OverlapDiagnostics {
            mean_delta_lightness: dl,
            mean_delta_lab_a: da,
            mean_delta_lab_b: db,
            global_exposure_delta_stops: exposure,
            linear_rgb_rmse: rmse,
        },
    }))
}

impl DeferredQuality {
    pub(crate) fn finish(
        self,
        shape: [usize; 2],
        support: Option<&[u8]>,
        heat_weights: Option<(&[f32], f64)>,
        details: Option<&str>,
    ) -> Result<Measurement, String> {
        let weights = heat_weights
            .map(|(weights, _)| weights)
            .filter(|weights| !weights.is_empty());
        let lightness = self.lightness.finish(weights, AFFECTED_THRESHOLD)?;
        finish(
            shape,
            support,
            None,
            Some((lightness, self.planes)),
            self.diagnostics,
            heat_weights,
            details,
        )
    }
}
