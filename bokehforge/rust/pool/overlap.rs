use rayon::prelude::*;
use serde_json::{Map, Value, json};

use super::{Grid, coverage_prefix, damage_prefix, damage_sum, tail_tile_count};
use crate::score::compensated_sum;

pub(crate) const AFFECTED_THRESHOLD: f32 = 0.25;
const TAIL_FRACTION: f64 = 0.01;
const POOL_WEIGHTS: [f64; 3] = [0.40, 0.35, 0.25];
const FAMILIES: [(&str, f64, &str, &str); 6] = [
    ("tone", 0.17, "Tone", "Brightness and contrast differences"),
    (
        "color",
        0.16,
        "Color",
        "Differences in tint or color intensity",
    ),
    (
        "lost_detail",
        0.18,
        "Lost detail",
        "Local contrast missing from the delivery",
    ),
    (
        "added_structure",
        0.15,
        "Added structure",
        "Excess local contrast and fine variation",
    ),
    (
        "boundary",
        0.23,
        "Boundary",
        "Shape and brightness differences near reference edges",
    ),
    (
        "range",
        0.11,
        "Range",
        "Changes beyond display black or white and near clipping",
    ),
];

#[derive(Clone, Copy, Default)]
pub(crate) struct OverlapDiagnostics {
    pub mean_delta_lightness: f64,
    pub global_exposure_delta_stops: f64,
    pub mean_delta_lab_a: f64,
    pub mean_delta_lab_b: f64,
    pub linear_rgb_rmse: f64,
}

struct TileGrid {
    side: usize,
    offset: [usize; 2],
    y: Vec<isize>,
    x: Vec<isize>,
    sample_y: Vec<isize>,
    sample_x: Vec<isize>,
    nominal_pixels: usize,
    covered: Option<Vec<i32>>,
}

impl TileGrid {
    fn grid(&self) -> Grid<'_> {
        Grid {
            y: &self.sample_y,
            x: &self.sample_x,
            coverage: self.covered.as_deref(),
            nominal_pixels: self.nominal_pixels,
        }
    }

    fn id(&self) -> String {
        format!("{}:{}:{}", self.side, self.offset[0], self.offset[1])
    }

    fn tile_count(&self) -> usize {
        (self.y.len() - 1) * (self.x.len() - 1)
    }

    fn tile_box(&self, index: usize) -> [isize; 4] {
        let columns = self.x.len() - 1;
        let (row, column) = (index / columns, index % columns);
        [
            self.x[column],
            self.y[row],
            self.x[column + 1],
            self.y[row + 1],
        ]
    }

    fn rectangle_coverage(&self, [top, left, bottom, right]: [usize; 4]) -> Vec<i32> {
        fn overlap(axis: &[isize], start: usize, end: usize) -> impl Iterator<Item = usize> + '_ {
            axis.windows(2).map(move |edges| {
                (edges[1] as usize)
                    .min(end)
                    .saturating_sub((edges[0] as usize).max(start))
            })
        }
        overlap(&self.y, top, bottom)
            .flat_map(|rows| {
                overlap(&self.x, left, right).map(move |columns| (rows * columns) as i32)
            })
            .collect()
    }

    fn describe(&self, values: Map<String, Value>) -> Value {
        json!({
            "grid_id": self.id(), "tile_size": self.side, "offset_yx": self.offset,
            "x_edges": self.x, "y_edges": self.y, "nominal_pixels": self.nominal_pixels,
            "covered_pixels": self.covered, "values": values,
        })
    }
}

fn axis_tiles(length: usize, side: usize, offset: usize) -> Vec<isize> {
    let start = if offset == 0 { side } else { offset };
    std::iter::once(0)
        .chain((start..length).step_by(side).map(|edge| edge as isize))
        .chain(std::iter::once(length as isize))
        .collect()
}

fn sample_axis(axis: &[isize]) -> Vec<isize> {
    axis.iter()
        .map(|&edge| (edge as usize).div_ceil(4) as isize)
        .collect()
}

fn tile_grids(
    height: usize,
    width: usize,
    support: Option<&[u8]>,
    exact_box: Option<[usize; 4]>,
    y_samples: &[isize],
    x_samples: &[isize],
) -> Vec<TileGrid> {
    let coverage = support.map(|mask| coverage_prefix(mask, y_samples, x_samples));
    let box_coverage = exact_box.filter(|&bounds| bounds != [0, 0, height, width]);
    let mut grids = Vec::with_capacity(8);
    for side in [8, 32] {
        for offset in [[0, 0], [0, side / 2], [side / 2, 0], [side / 2, side / 2]] {
            let y = axis_tiles(height, side, offset[0]);
            let x = axis_tiles(width, side, offset[1]);
            let mut grid = TileGrid {
                side,
                offset,
                sample_y: sample_axis(&y),
                sample_x: sample_axis(&x),
                y,
                x,
                nominal_pixels: side.min(height) * side.min(width),
                covered: None,
            };
            grid.covered = if let Some(bounds) = box_coverage {
                Some(grid.rectangle_coverage(bounds))
            } else {
                coverage.as_ref().map(|prefix| {
                    grid.grid()
                        .totals(prefix, x_samples.len())
                        .map(|total| total as i32)
                        .collect()
                })
            };
            grids.push(grid);
        }
    }
    grids
}

struct FamilyTail {
    value: f64,
    grid: usize,
    count: usize,
    eligible_count: usize,
    cutoff: f64,
}

#[derive(Default)]
struct FamilyPool {
    mean: f64,
    affected_fraction: f64,
    worst: Option<(f64, usize, usize)>,
    tail: Option<FamilyTail>,
    tiles: Vec<Vec<f64>>,
}

impl FamilyPool {
    fn exact(grids: &[TileGrid]) -> Self {
        let tail = grids.first().map(|grid| {
            let eligible_count = grid.covered.as_ref().map_or_else(
                || grid.tile_count(),
                |covered| covered.iter().filter(|&&count| count > 0).count(),
            );
            FamilyTail {
                value: 0.0,
                grid: 0,
                count: tail_tile_count(TAIL_FRACTION, eligible_count),
                eligible_count,
                cutoff: 0.0,
            }
        });
        Self {
            tail,
            ..Self::default()
        }
    }

    fn contributions(&self) -> [f64; 3] {
        [
            POOL_WEIGHTS[0] * self.mean,
            POOL_WEIGHTS[1] * self.tail.as_ref().map_or(0.0, |tail| tail.value),
            POOL_WEIGHTS[2] * self.worst.map_or(0.0, |(value, _, _)| value),
        ]
    }

    fn burden(&self) -> f64 {
        let [mean, tail, worst] = self.contributions();
        mean + tail + worst
    }

    fn tail_indices(&self, tail: &FamilyTail, grid: &TileGrid) -> Vec<usize> {
        let coverage = grid.grid();
        let Some(tiles) = self.tiles.get(tail.grid) else {
            return (0..grid.tile_count())
                .filter(|&index| coverage.contains(index))
                .take(tail.count)
                .collect();
        };
        let above = tiles
            .iter()
            .enumerate()
            .filter(|&(index, &value)| value > tail.cutoff && coverage.contains(index))
            .count();
        let mut remaining = tail.count.saturating_sub(above);
        tiles
            .iter()
            .enumerate()
            .filter_map(|(index, &value)| {
                if !coverage.contains(index) {
                    return None;
                }
                if value > tail.cutoff {
                    return Some(index);
                }
                if value == tail.cutoff && remaining > 0 {
                    remaining -= 1;
                    return Some(index);
                }
                None
            })
            .collect()
    }

    fn summary(&self, grids: &[TileGrid]) -> Value {
        let region = self
            .worst
            .map(|(_, grid, index)| grids[grid].tile_box(index));
        let mut result = json!({
            "burden": self.burden(), "mean": self.mean, "tail": self.tail.as_ref().map_or(0.0, |tail| tail.value),
            "worst": self.worst.map_or(0.0, |(value, _, _)| value), "worst_box_view_xyxy": region,
            "affected_fraction": self.affected_fraction,
        });
        if let Some(region) = region {
            result["worst_box_native_xyxy"] = json!(region);
        }
        result
    }

    fn explain(&self, grids: &[TileGrid], keep_tiles: bool) -> Value {
        let [mean, tail, worst] = self.contributions();
        let mut worst_term = json!({
            "label": "Worst single tile", "value": 0.0, "weight": POOL_WEIGHTS[2], "contribution": worst,
            "grid_id": null, "tile_size": null, "offset_yx": null, "tile_index": null, "box_xyxy": null,
        });
        if let Some((value, grid, index)) = self.worst {
            let tile = &grids[grid];
            worst_term["value"] = json!(value);
            worst_term["grid_id"] = json!(tile.id());
            worst_term["tile_size"] = json!(tile.side);
            worst_term["offset_yx"] = json!(tile.offset);
            worst_term["tile_index"] = json!(index);
            worst_term["box_xyxy"] = json!(tile.tile_box(index));
        }

        let selected = self.tail.as_ref().expect("an explained overlap has a tail");
        let tile = &grids[selected.grid];
        let mut tail_term = json!({
            "label": "Average of the worst 1% of tiles", "value": selected.value,
            "weight": POOL_WEIGHTS[1], "contribution": tail, "fraction": TAIL_FRACTION,
            "grid_id": tile.id(), "tile_size": tile.side, "offset_yx": tile.offset,
            "tile_count": selected.count, "eligible_tile_count": selected.eligible_count, "cutoff": selected.cutoff,
        });
        if keep_tiles {
            tail_term["tile_indices"] = json!(self.tail_indices(selected, tile));
        }
        json!({
            "mean": {"label": "Average damage", "value": self.mean, "weight": POOL_WEIGHTS[0], "contribution": mean},
            "tail": tail_term,
            "worst": worst_term,
        })
    }
}

struct PoolGeometry<'a> {
    y: &'a [isize],
    x: &'a [isize],
    grids: &'a [TileGrid],
    support: Option<&'a [u8]>,
    measured_pixels: usize,
    damage_max: f64,
    keep_tiles: bool,
}

#[derive(Clone, Copy)]
pub(crate) enum FamilyInput<'a> {
    Pixels(&'a [f32]),
    Cells(&'a [[f32; 3]]),
}

impl PoolGeometry<'_> {
    fn pool(&self, data: FamilyInput<'_>) -> FamilyPool {
        let (integral, sum, affected) = match data {
            FamilyInput::Pixels(image) => {
                let (integral, affected) =
                    damage_prefix(image, self.support, self.y, self.x, AFFECTED_THRESHOLD);
                (integral, damage_sum(image, self.support, 8_192), affected)
            }
            FamilyInput::Cells(cells) => self.cells_prefix(cells),
        };
        let mut family = FamilyPool {
            mean: sum / self.measured_pixels as f64,
            affected_fraction: affected as f64 / self.measured_pixels as f64,
            ..FamilyPool::default()
        };
        let mut eligible = Vec::new();
        for (index, tile) in self.grids.iter().enumerate() {
            let Some(selected) = tile.grid().select(
                &integral,
                self.x.len(),
                self.damage_max,
                TAIL_FRACTION,
                self.keep_tiles,
                &mut eligible,
            ) else {
                continue;
            };
            if selected.worst > family.worst.map_or(0.0, |(value, _, _)| value) {
                family.worst = Some((selected.worst, index, selected.worst_index));
            }
            let value = compensated_sum(selected.top(&eligible).iter().copied())
                / selected.tail_count as f64;
            if family.tail.as_ref().is_none_or(|old| value > old.value) {
                family.tail = Some(FamilyTail {
                    value,
                    grid: index,
                    count: selected.tail_count,
                    eligible_count: selected.eligible_count,
                    cutoff: selected.cutoff,
                });
            }
            if let Some(values) = selected.tiles {
                family.tiles.push(values);
            }
        }
        family
    }

    fn cells_prefix(&self, cells: &[[f32; 3]]) -> (Vec<f64>, f64, usize) {
        let pitch = self.x.len();
        let mut integral = vec![0.0f64; pitch * self.y.len()];
        let mut sum = 0.0f64;
        let mut affected = 0;
        for (row, cell_row) in cells.chunks_exact(pitch - 1).enumerate() {
            let (previous, current) = integral.split_at_mut((row + 1) * pitch);
            let before = &previous[row * pitch + 1..];
            let after = &mut current[1..pitch];
            let mut subtotal = 0.0f64;
            for ((&above, output), &[high, low, count]) in before.iter().zip(after).zip(cell_row) {
                let value = f64::from(high) + f64::from(low);
                subtotal += value;
                *output = above + subtotal;
                sum += value;
                affected += count as usize;
            }
        }
        (integral, sum, affected)
    }
}

pub(crate) struct PooledOverlap {
    pub score: f64,
    weighted_damage: f64,
    families: Vec<FamilyPool>,
    grids: Vec<TileGrid>,
    width: usize,
    height: usize,
    measured_pixels: usize,
    inspection_mix: Option<f64>,
    damage_max: f64,
    explain: bool,
    keep_tiles: bool,
}

impl PooledOverlap {
    pub fn describe(&self, diagnostics: OverlapDiagnostics) -> (Value, Value, Option<Value>) {
        let mut features = Map::new();
        let mut summaries = Map::new();
        let mut families = Map::new();
        for ((name, weight, label, description), pool) in FAMILIES.into_iter().zip(&self.families) {
            let burden = pool.burden();
            features.insert(format!("native.{name}"), json!(burden));
            summaries.insert(name.to_owned(), pool.summary(&self.grids));
            if self.explain {
                families.insert(name.to_owned(), json!({
                    "label": label, "description": description, "weight": weight, "burden": burden,
                    "weighted_damage": weight * burden, "pool": pool.explain(&self.grids, self.keep_tiles),
                    "affected_fraction": pool.affected_fraction,
                }));
            }
        }
        let signed = json!({
            "mean_delta_lightness": diagnostics.mean_delta_lightness,
            "global_exposure_delta_stops": diagnostics.global_exposure_delta_stops,
            "mean_delta_lab_a": diagnostics.mean_delta_lab_a,
            "mean_delta_lab_b": diagnostics.mean_delta_lab_b,
        });
        let mut native = signed.clone();
        native["native_box_xyxy"] = json!([0, 0, self.width, self.height]);
        native["view_size_wh"] = json!([self.width, self.height]);
        native["families"] = Value::Object(summaries);
        if self.measured_pixels < self.height * self.width {
            native["covered_fraction"] =
                json!(self.measured_pixels as f64 / (self.height * self.width) as f64);
        }
        let diagnostic_values =
            json!({"views": {"native": [native]}, "linear_rgb_rmse": diagnostics.linear_rgb_rmse});
        let explanation = self.explain.then(|| {
            let mut signed = signed;
            signed["linear_rgb_rmse"] = json!(diagnostics.linear_rgb_rmse);
            let damage_range = if self.inspection_mix.is_some() {
                json!([0, self.damage_max])
            } else {
                json!([0, 1])
            };
            let mut result = json!({
                "reference_size_wh": [self.width, self.height], "damage_range": damage_range,
                "affected_threshold": AFFECTED_THRESHOLD, "weighted_damage": self.weighted_damage,
                "families": families, "measured_pixels": self.measured_pixels, "diagnostics": signed,
            });
            if let Some(mix) = self.inspection_mix {
                result["inspection_mix"] = json!(mix);
            }
            if self.keep_tiles {
                result["tile_grids"] = Value::Object(self.describe_tiles());
            }
            result
        });
        (Value::Object(features), diagnostic_values, explanation)
    }

    fn describe_tiles(&self) -> Map<String, Value> {
        self.grids
            .iter()
            .enumerate()
            .map(|(index, grid)| {
                let values = FAMILIES
                    .iter()
                    .zip(&self.families)
                    .map(|(&(name, ..), family)| {
                        let tiles = family.tiles.get(index).map_or_else(
                            || Value::Array(vec![json!(0.0); grid.tile_count()]),
                            |values| json!(values),
                        );
                        (name.to_owned(), tiles)
                    })
                    .collect();
                (grid.id(), grid.describe(values))
            })
            .collect()
    }
}

pub(crate) fn pool_overlap(
    height: usize,
    width: usize,
    damage: Option<&[FamilyInput<'_>; 6]>,
    support: Option<&[u8]>,
    exact_box: Option<[usize; 4]>,
    inspection_mix: Option<f64>,
    details: Option<&str>,
) -> Option<PooledOverlap> {
    let pixels = height.checked_mul(width)?;
    if pixels == 0
        || support.is_some_and(|mask| mask.len() != pixels)
        || damage.is_some_and(|maps| {
            maps.iter().any(|map| match map {
                FamilyInput::Pixels(samples) => samples.len() != pixels,
                FamilyInput::Cells(cells) => cells.len() != height.div_ceil(4) * width.div_ceil(4),
            })
        })
    {
        return None;
    }
    let measured_pixels = if let Some([top, left, bottom, right]) = exact_box {
        if bottom > height || right > width {
            return None;
        }
        bottom.checked_sub(top)? * right.checked_sub(left)?
    } else {
        support.map_or(pixels, |mask| {
            mask.iter().filter(|&&valid| valid != 0).count()
        })
    };
    if measured_pixels == 0 {
        return None;
    }
    let explain = details.is_some();
    let keep_tiles = matches!(details, Some("tiles" | "full"));
    let damage_max = if inspection_mix.is_some() { 2.0 } else { 1.0 };

    let (families, grids): (Vec<FamilyPool>, _) = if damage.is_none() && !explain {
        (
            (0..FAMILIES.len()).map(|_| FamilyPool::default()).collect(),
            Vec::new(),
        )
    } else {
        // All 8- and 32-pixel grids and their half offsets share four-pixel samples.
        let y = axis_tiles(height, 4, 0);
        let x = axis_tiles(width, 4, 0);
        let grids = tile_grids(height, width, support, exact_box, &y, &x);
        let geometry = PoolGeometry {
            y: &y,
            x: &x,
            grids: &grids,
            support,
            measured_pixels,
            damage_max,
            keep_tiles,
        };
        let families = if let Some(maps) = damage {
            if pixels >= 1_000_000 && !keep_tiles {
                crate::parallel(|| maps.par_iter().map(|&image| geometry.pool(image)).collect())
            } else {
                maps.iter().map(|&image| geometry.pool(image)).collect()
            }
        } else {
            (0..FAMILIES.len())
                .map(|_| FamilyPool::exact(&grids))
                .collect()
        };
        (families, grids)
    };
    let weighted_damage = compensated_sum(
        FAMILIES
            .iter()
            .zip(&families)
            .map(|(&(_, weight, ..), pool)| weight * pool.burden()),
    );
    Some(PooledOverlap {
        score: 100.0 * (-5.0 * weighted_damage).exp(),
        weighted_damage,
        families,
        grids,
        width,
        height,
        measured_pixels,
        inspection_mix,
        damage_max,
        explain,
        keep_tiles,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_crop_reports_coverage_and_zero_tile_values() {
        let pooled = pool_overlap(11, 13, None, None, Some([2, 6, 9, 12]), None, Some("full"))
            .expect("visible exact crop");
        assert_eq!(pooled.score, 100.0);
        let (_, diagnostics, explanation) = pooled.describe(OverlapDiagnostics::default());
        let explanation = explanation.expect("full explanation");
        assert_eq!(explanation["measured_pixels"], json!(42));
        let grid = &explanation["tile_grids"]["8:0:0"];
        assert_eq!(grid["covered_pixels"], json!([12, 24, 2, 4]));
        assert_eq!(grid["values"]["tone"], json!([0.0, 0.0, 0.0, 0.0]));
        let tail = &explanation["families"]["tone"]["pool"]["tail"];
        assert_eq!(tail["tile_indices"], json!([0]));
        assert_eq!(tail["eligible_tile_count"], json!(4));
        assert_eq!(
            diagnostics["views"]["native"][0]["covered_fraction"],
            json!(42.0 / 143.0)
        );
    }

    #[test]
    fn tied_tiles_use_the_first_eligible_index_and_skip_empty_support() {
        let (height, width) = (17, 17);
        let maps = std::array::from_fn(|_| vec![0.5; height * width]);
        let mut support = vec![0; height * width];
        assert!(
            pool_overlap(
                height,
                width,
                Some(&maps.each_ref().map(|plane| FamilyInput::Pixels(plane))),
                Some(&support),
                None,
                None,
                None
            )
            .is_none()
        );
        support.fill(7);
        for row in support.chunks_exact_mut(width).take(8) {
            row[..8].fill(0);
        }
        let pooled = pool_overlap(
            height,
            width,
            Some(&maps.each_ref().map(|plane| FamilyInput::Pixels(plane))),
            Some(&support),
            None,
            None,
            Some("tiles"),
        )
        .expect("partially measured tile grid");
        let (features, diagnostics, explanation) = pooled.describe(OverlapDiagnostics::default());
        assert_eq!(features["native.tone"], json!(0.5));
        assert_eq!(
            diagnostics["views"]["native"][0]["families"]["tone"]["affected_fraction"],
            json!(1.0)
        );
        let explanation = explanation.expect("tile explanation");
        let pool = &explanation["families"]["tone"]["pool"];
        assert_eq!(pool["worst"]["grid_id"], "8:0:0");
        assert_eq!(pool["worst"]["tile_index"], 1);
        assert_eq!(pool["tail"]["grid_id"], "8:0:0");
        assert_eq!(pool["tail"]["tile_indices"], json!([1]));
        assert_eq!(pool["tail"]["eligible_tile_count"], 8);
    }
}
