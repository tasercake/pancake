use std::borrow::Cow;

use rayon::prelude::*;
use serde_json::{Map, Value, json};

use crate::parallel;

mod search;

const ALIGNMENT_VERSION: usize = 1;
const MAX_PLACEMENTS: usize = 3;
const NATIVE_DELIVERY: &str = "exact_native_delivery_in_reference";
const NATIVE_REFERENCE: &str = "exact_reference_in_native_delivery";
const EXACT_INTERIOR: &str = "exact_interior_in_reference";

type Bounds = [usize; 4];

#[derive(Clone, Copy)]
pub(crate) struct ImageView<'a> {
    pub h: usize,
    pub w: usize,
    pub data: &'a [f32],
    stride: usize,
}

impl<'a> ImageView<'a> {
    pub(crate) fn new(data: &'a [f32], h: usize, w: usize) -> Self {
        debug_assert!(h > 0 && w > 0 && data.len() == h * w * 3);
        Self {
            data,
            h,
            w,
            stride: w * 3,
        }
    }

    pub(crate) fn shape(self) -> [usize; 2] {
        [self.h, self.w]
    }

    fn bounds(self) -> Bounds {
        [0, 0, self.h, self.w]
    }

    fn crop(self, [top, left, bottom, right]: Bounds) -> Self {
        Self {
            data: &self.data[top * self.stride + left * 3..],
            h: bottom - top,
            w: right - left,
            stride: self.stride,
        }
    }

    #[inline]
    pub(crate) fn row(self, y: usize) -> &'a [f32] {
        let start = y * self.stride;
        &self.data[start..start + self.w * 3]
    }

    #[inline]
    pub(crate) fn pixel(self, y: usize, x: usize) -> [f32; 3] {
        let index = y * self.stride + 3 * x;
        [self.data[index], self.data[index + 1], self.data[index + 2]]
    }

    #[inline]
    pub(crate) fn sample(self, y: f64, x: f64) -> [f32; 3] {
        let top = y.floor();
        let left = x.floor();
        let y0 = (top as usize).min(self.h - 1);
        let x0 = (left as usize).min(self.w - 1);
        let upper_left = self.pixel(y0, x0);
        if y == top && x == left {
            return upper_left.map(|value| 0.0 + value);
        }
        let y1 = ((top + 1.0) as usize).min(self.h - 1);
        let x1 = ((left + 1.0) as usize).min(self.w - 1);
        let upper_right = self.pixel(y0, x1);
        let lower_left = self.pixel(y1, x0);
        let lower_right = self.pixel(y1, x1);
        let wy0 = 1.0 - (y - top);
        let wx0 = 1.0 - (x - left);
        let wy1 = 1.0 - wy0;
        let wx1 = 1.0 - wx0;
        std::array::from_fn(|c| {
            let mut value = 0.0;
            value += (f64::from(upper_left[c]) * wy0) * wx0;
            value += (f64::from(upper_right[c]) * wy0) * wx1;
            value += (f64::from(lower_left[c]) * wy1) * wx0;
            value += (f64::from(lower_right[c]) * wy1) * wx1;
            value as f32
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Pose {
    pub y: f64,
    pub x: f64,
    pub angle: f64,
    pub scale: f64,
}

impl Pose {
    pub(crate) fn new(y: f64, x: f64) -> Self {
        Self {
            y,
            x,
            angle: 0.0,
            scale: 1.0,
        }
    }

    pub(crate) fn matrix(self) -> [[f64; 2]; 2] {
        let (sine, cosine) = self.angle.to_radians().sin_cos();
        let a = self.scale * cosine;
        let b = self.scale * sine;
        [[a, -b], [b, a]]
    }

    pub(crate) fn transform(self, point: [f64; 2]) -> [f64; 2] {
        let [y, x] = map(self.matrix(), point);
        [y + self.y, x + self.x]
    }

    pub(crate) fn move_origin(self, point: [f64; 2]) -> Self {
        let [y, x] = self.transform(point);
        Self { y, x, ..self }
    }

    pub(crate) fn describe(self) -> Value {
        let [[d, c], [b, a]] = self.matrix();
        json!({
            "offset_yx": [self.y, self.x],
            "rotation_degrees": self.angle,
            "scale": self.scale,
            "candidate_to_reference_xy": [[a, b, self.x], [c, d, self.y]],
        })
    }

    fn integer_offset(self) -> Option<[isize; 2]> {
        (self.angle == 0.0 && self.scale == 1.0 && self.y.fract() == 0.0 && self.x.fract() == 0.0)
            .then_some([self.y as isize, self.x as isize])
    }
}

#[inline]
fn map(matrix: [[f64; 2]; 2], [y, x]: [f64; 2]) -> [f64; 2] {
    [
        matrix[0][0] * y + matrix[0][1] * x,
        matrix[1][0] * y + matrix[1][1] * x,
    ]
}

pub(crate) fn displacement(a: Pose, b: Pose, [h, w]: [usize; 2]) -> f64 {
    let am = a.matrix();
    let bm = b.matrix();
    let matrix = std::array::from_fn(|i| std::array::from_fn(|j| am[i][j] - bm[i][j]));
    let dy = a.y - b.y;
    let dx = a.x - b.x;
    let h = (h - 1) as f64;
    let w = (w - 1) as f64;
    [[0.0, 0.0], [0.0, w], [h, 0.0], [h, w]]
        .into_iter()
        .map(|corner| {
            let [y, x] = map(matrix, corner);
            (y + dy).abs().max((x + dx).abs())
        })
        .fold(0.0, f64::max)
}

pub(crate) fn linspace(start: f64, end: f64, count: usize) -> impl Iterator<Item = f64> {
    let step = (end - start) / count.saturating_sub(1).max(1) as f64;
    (0..count).map(move |index| {
        if index > 0 && index == count - 1 {
            end
        } else {
            start + step * index as f64
        }
    })
}

pub(crate) fn same_uniform(image: ImageView<'_>) -> bool {
    let color = image.pixel(0, 0);
    (0..image.h).step_by((image.h / 24).max(1)).all(|y| {
        (0..image.w)
            .step_by((image.w / 24).max(1))
            .all(|x| image.pixel(y, x) == color)
    }) && (0..image.h).all(|y| image.row(y).as_chunks::<3>().0.iter().all(|&p| p == color))
}

fn matches_at(needle: ImageView<'_>, image: ImageView<'_>, y: usize, x: usize) -> bool {
    let start = x * 3;
    let end = start + needle.w * 3;
    (0..needle.h).all(|row| needle.row(row) == &image.row(y + row)[start..end])
}

fn uniform_offsets(
    needle: ImageView<'_>,
    image: ImageView<'_>,
    trace: &mut Value,
) -> Vec<[usize; 2]> {
    let rows = image.h - needle.h + 1;
    let columns = image.w - needle.w + 1;
    let center = [(rows - 1) / 2, (columns - 1) / 2];
    let color = needle.pixel(0, 0);
    if same_uniform(image) {
        let equal = color == image.pixel(0, 0);
        trace["result"] = json!(if equal {
            "uniform_match"
        } else {
            "different_samples"
        });
        trace["verified_offsets"] = json!(1);
        trace["equivalent_offsets"] = json!(if equal { rows * columns } else { 0 });
        return equal.then_some(center).into_iter().collect();
    }

    // A vertical count per column supports a constant rectangle check at
    // every possible offset using only one row of counters.
    let mut counts = vec![0usize; image.w];
    for y in 0..needle.h {
        for (count, &pixel) in counts.iter_mut().zip(image.row(y).as_chunks::<3>().0) {
            *count += usize::from(pixel == color);
        }
    }
    let mut found = None;
    let mut equivalents = 0;
    for y in 0..rows {
        let mut count: usize = counts[..needle.w].iter().sum();
        for x in 0..columns {
            if count == needle.h * needle.w {
                equivalents += 1;
                if found.is_none() || [y, x] == center {
                    found = Some([y, x]);
                }
            }
            if x + 1 < columns {
                count += counts[x + needle.w];
                count -= counts[x];
            }
        }
        if y + 1 < rows {
            for ((count, &old), &new) in counts
                .iter_mut()
                .zip(image.row(y).as_chunks::<3>().0)
                .zip(image.row(y + needle.h).as_chunks::<3>().0)
            {
                *count += usize::from(new == color);
                *count -= usize::from(old == color);
            }
        }
    }
    trace["result"] = json!(if found.is_some() {
        "found"
    } else {
        "different_samples"
    });
    trace["equivalent_offsets"] = json!(equivalents);
    trace["verified_offsets"] = json!(rows * columns);
    found.into_iter().collect()
}

fn anchors(needle: ImageView<'_>) -> Vec<([usize; 2], [f32; 3])> {
    let mut points = Vec::new();
    for y in linspace(0.0, (needle.h - 1) as f64, needle.h.min(9)).map(|v| v as usize) {
        for x in linspace(0.0, (needle.w - 1) as f64, needle.w.min(9)).map(|v| v as usize) {
            points.push(([y, x], needle.pixel(y, x)));
        }
    }
    let median: [f64; 3] = std::array::from_fn(|c| {
        let mut values: Vec<_> = points.iter().map(|(_, rgb)| f64::from(rgb[c])).collect();
        values.sort_by(f64::total_cmp);
        (values[(values.len() - 1) / 2] + values[values.len() / 2]) / 2.0
    });
    let rarity = |rgb: [f32; 3]| {
        (0..3)
            .map(|c| (f64::from(rgb[c]) - median[c]).abs())
            .fold(0.0, f64::max)
    };
    points.sort_by(|(_, a), (_, b)| rarity(*b).total_cmp(&rarity(*a)));
    points.truncate(16);
    for [y, x] in [
        [needle.h / 2, needle.w / 2],
        [0, 0],
        [needle.h - 1, needle.w - 1],
    ] {
        points.push(([y, x], needle.pixel(y, x)));
    }
    points
}

const BX: u64 = 1_099_511_628_211;
const BY: u64 = 14_029_467_366_897_019_727;
const INV_X: u64 = 14_886_173_955_864_302_971;
const INV_Y: u64 = 839_798_700_976_720_815;

fn rgb_hash(rgb: &[f32]) -> u64 {
    let word = |v: f32| if v == 0.0 { 0 } else { u64::from(v.to_bits()) };
    word(rgb[0]).wrapping_mul(11_400_714_819_323_198_485)
        ^ word(rgb[1]).wrapping_mul(3_202_034_522_624_059_733)
        ^ word(rgb[2])
}

fn row_digest(row: &[f32]) -> u64 {
    row.rchunks_exact(3).fold(0u64, |sum, rgb| {
        sum.wrapping_mul(BX).wrapping_add(rgb_hash(rgb))
    })
}

fn row_hashes(row: &[f32], width: usize, right_power: u64, mut output: impl FnMut(usize, u64)) {
    let span = width * 3;
    let mut hash = row_digest(&row[..span]);
    output(0, hash);
    let (pixels, _) = row.as_chunks::<3>();
    for (index, (old, new)) in pixels.iter().zip(&pixels[width..]).enumerate() {
        hash = hash
            .wrapping_sub(rgb_hash(old))
            .wrapping_mul(INV_X)
            .wrapping_add(rgb_hash(new).wrapping_mul(right_power));
        output(index + 1, hash);
    }
}

fn rolling_offsets(
    needle: ImageView<'_>,
    image: ImageView<'_>,
    verified: &mut usize,
) -> Vec<[usize; 2]> {
    let columns = image.w - needle.w + 1;
    let rows = image.h - needle.h + 1;
    let right_power = BX.wrapping_pow((needle.w - 1) as u32);
    let bottom_power = BY.wrapping_pow((needle.h - 1) as u32);
    let digest = (0..needle.h).rev().fold(0u64, |sum, y| {
        sum.wrapping_mul(BY).wrapping_add(row_digest(needle.row(y)))
    });
    let mut hashes = vec![0u64; columns];
    for y in (0..needle.h).rev() {
        row_hashes(image.row(y), needle.w, right_power, |x, row_hash| {
            hashes[x] = hashes[x].wrapping_mul(BY).wrapping_add(row_hash);
        });
    }
    let mut matches = Vec::new();
    for y in 0..rows {
        for (x, &hash) in hashes.iter().enumerate() {
            if hash == digest {
                *verified += 1;
                if matches_at(needle, image, y, x) {
                    matches.push([y, x]);
                    if matches.len() == MAX_PLACEMENTS {
                        return matches;
                    }
                }
            }
        }
        if y + 1 < rows {
            row_hashes(image.row(y), needle.w, right_power, |x, old| {
                hashes[x] = hashes[x].wrapping_sub(old).wrapping_mul(INV_Y);
            });
            row_hashes(image.row(y + needle.h), needle.w, right_power, |x, new| {
                hashes[x] = hashes[x].wrapping_add(new.wrapping_mul(bottom_power));
            });
        }
    }
    matches
}

fn exact_offsets(
    needle: ImageView<'_>,
    image: ImageView<'_>,
    trace: &mut Value,
) -> Vec<[usize; 2]> {
    if needle.h > image.h || needle.w > image.w {
        trace["result"] = json!("does_not_fit");
        return Vec::new();
    }
    if needle.shape() == image.shape() {
        let screened = needle.h * needle.w < 262_144
            || (0..needle.h).step_by((needle.h / 24).max(1)).all(|y| {
                (0..needle.w)
                    .step_by((needle.w / 24).max(1))
                    .all(|x| needle.pixel(y, x) == image.pixel(y, x))
            });
        let equal = screened && matches_at(needle, image, 0, 0);
        trace["result"] = json!(if equal { "found" } else { "different_samples" });
        trace["verified_offsets"] = json!(1);
        return equal.then_some([0, 0]).into_iter().collect();
    }
    if same_uniform(needle) {
        return uniform_offsets(needle, image, trace);
    }
    let anchors = anchors(needle);
    let columns = image.w - needle.w + 1;
    let rows = image.h - needle.h + 1;
    let batch = (262_144 / columns).max(1);
    let verify_limit = (12_000_000 / (needle.h * needle.w)).max(16);
    let mut matches = Vec::new();
    let mut verified = 0;
    let mut examined = 0;
    'search: for start in (0..rows).step_by(batch) {
        let end = (start + batch).min(rows);
        examined += (end - start) * columns;
        for y in start..end {
            for x in 0..columns {
                if !anchors
                    .iter()
                    .all(|&([dy, dx], color)| image.pixel(y + dy, x + dx) == color)
                {
                    continue;
                }
                verified += 1;
                if matches_at(needle, image, y, x) {
                    matches.push([y, x]);
                    if matches.len() == MAX_PLACEMENTS {
                        break 'search;
                    }
                }
                if verified >= verify_limit {
                    trace["fallback"] = json!("rolling_rgb_rectangle_hash");
                    matches = rolling_offsets(needle, image, &mut verified);
                    break 'search;
                }
            }
        }
    }
    trace["result"] = json!(if matches.is_empty() {
        "different_samples"
    } else {
        "found"
    });
    trace["anchors"] = json!(anchors.len());
    trace["screened_offsets"] = json!(examined);
    trace["verified_offsets"] = json!(verified);
    trace["retained_limit"] = json!(MAX_PLACEMENTS);
    matches
}

struct Strip {
    bound: usize,
    pixels: usize,
    fill: bool,
}

impl Strip {
    fn describe(&self) -> Value {
        let side = ["top", "left", "bottom", "right"][self.bound];
        json!({
            "side": side,
            "kind": if self.fill { "flat_fill" } else { "repeated_edge" },
            "pixels": self.pixels,
        })
    }
}

fn proposed_content(image: ImageView<'_>) -> (Bounds, Vec<Strip>) {
    let shape = image.shape();
    let mut bounds = image.bounds();
    let mut strips = Vec::new();
    for bound in [0, 2, 1, 3] {
        let axis = bound % 2;
        let reverse = bound >= 2;
        let length = shape[axis];
        let across = shape[1 - axis];
        let margin = across / 6;
        let range = margin..across - margin;
        let pixel = |index, cross| {
            let along = if reverse { length - 1 - index } else { index };
            if axis == 0 {
                image.pixel(along, cross)
            } else {
                image.pixel(cross, along)
            }
        };
        let mut run = 1;
        for index in 1..length / 2 {
            if !range
                .clone()
                .all(|cross| pixel(index, cross) == pixel(0, cross))
            {
                break;
            }
            run += 1;
        }
        let fill = range
            .clone()
            .all(|cross| pixel(0, cross) == pixel(0, margin));
        let pixels = if fill { run } else { run - 1 };
        if pixels < if fill { 2 } else { 1 } || run >= length / 2 {
            continue;
        }
        if reverse {
            bounds[bound] -= pixels;
        } else {
            bounds[bound] += pixels;
        }
        strips.push(Strip {
            bound,
            pixels,
            fill,
        });
    }
    (bounds, strips)
}

fn rgb_delta(left: [f32; 3], right: [f32; 3]) -> f64 {
    left.into_iter()
        .zip(right)
        .map(|(a, b)| (f64::from(a.clamp(0.0, 1.0)) - f64::from(b.clamp(0.0, 1.0))).abs())
        .sum()
}

fn verify_strips(
    image: ImageView<'_>,
    target: ImageView<'_>,
    content: Bounds,
    strips: &[Strip],
    pose: Pose,
    exact: bool,
) -> (Bounds, Vec<Value>) {
    let mut bounds = image.bounds();
    let mut report = Vec::with_capacity(strips.len());
    let max = [target.h as f64, target.w as f64];
    for strip in strips {
        let axis = strip.bound % 2;
        let orth = 1 - axis;
        let end = strip.bound >= 2;
        let inner = (content[strip.bound] - usize::from(end)) as f64;
        let outer = if end {
            (image.shape()[axis] - 1) as f64
        } else {
            0.0
        };
        let across =
            linspace(content[orth] as f64, (content[orth + 2] - 1) as f64, 28).collect::<Vec<_>>();
        let count = strip.pixels.min(10);
        let first_outer = inner + if end { 1.0 } else { -1.0 };
        let mut inside = 0;
        let mut present = 0;
        let mut seam_error = 0.0;
        let mut strip_error = 0.0;
        let matrix = pose.matrix();
        let transform = |point| {
            let [y, x] = map(matrix, point);
            [y + pose.y, x + pose.x]
        };
        for along in linspace(first_outer, outer, count) {
            for &cross in &across {
                let mut point = [0.0; 2];
                point[axis] = along;
                point[orth] = cross;
                let [y, x] = transform(point);
                if y >= -0.5 && y <= max[0] - 0.5 && x >= -0.5 && x <= max[1] - 0.5 {
                    inside += 1;
                    strip_error += rgb_delta(image.sample(point[0], point[1]), target.sample(y, x));
                }
                point[axis] = inner;
                let [y, x] = transform(point);
                if y >= 0.0 && y <= max[0] - 1.0 && x >= 0.0 && x <= max[1] - 1.0 {
                    present += 1;
                    seam_error += rgb_delta(image.sample(point[0], point[1]), target.sample(y, x));
                }
            }
        }
        let outside = 1.0 - inside as f64 / (count * across.len()) as f64;
        let seam_error = if present == 0 {
            1.0
        } else {
            seam_error / (present * 3) as f64
        };
        let strip_error = if inside == 0 {
            0.0
        } else {
            strip_error / (inside * 3) as f64
        };
        let verified = (exact || seam_error < 0.045)
            && outside >= 0.60
            && (outside >= 0.98 || strip_error > (seam_error * 1.8).max(0.02));
        if verified {
            bounds[strip.bound] = content[strip.bound];
        }
        let mut row = strip.describe();
        row["outside_reference_fraction"] = json!(outside);
        row["seam_rgb_delta"] = json!(seam_error);
        row["strip_rgb_delta"] = json!(strip_error);
        row["decision"] = json!(if verified { "excluded" } else { "retained" });
        report.push(row);
    }
    (bounds, report)
}

pub(crate) struct Placement {
    pub pose: Pose,
    pub bounds: Bounds,
    pub weight: f64,
    pub evidence: Value,
    pub padding: Vec<Value>,
}

fn append_stage(trace: &mut Map<String, Value>, stage: Value) {
    trace
        .get_mut("stages")
        .and_then(Value::as_array_mut)
        .expect("alignment stages")
        .push(stage);
}

fn finish(
    mut trace: Map<String, Value>,
    placements: Vec<Placement>,
    reason: Value,
    confidence: f64,
) -> (Vec<Placement>, Value) {
    let status = if placements.is_empty() {
        "unmatched"
    } else if reason
        .as_str()
        .is_some_and(|reason| reason.contains("fallback"))
    {
        "uncertain"
    } else {
        "matched"
    };
    trace.insert("status".to_owned(), json!(status));
    trace.insert("reason".to_owned(), reason);
    trace.insert("confidence".to_owned(), json!(confidence));
    (placements, Value::Object(trace))
}

pub(crate) fn align(candidate: ImageView<'_>, target: ImageView<'_>) -> (Vec<Placement>, Value) {
    let mut trace = Map::from_iter([
        ("schema_version".to_owned(), json!(ALIGNMENT_VERSION)),
        ("reference_size_wh".to_owned(), json!([target.w, target.h])),
        (
            "candidate_size_wh".to_owned(),
            json!([candidate.w, candidate.h]),
        ),
        ("stages".to_owned(), json!([])),
        ("candidates".to_owned(), json!([])),
    ]);
    let full = candidate.bounds();
    let mut attempt = json!({"method": NATIVE_DELIVERY});
    let offsets = exact_offsets(candidate, target, &mut attempt);
    let result = attempt["result"].clone();
    append_stage(&mut trace, attempt);
    if !offsets.is_empty() {
        let weight = 1.0 / offsets.len() as f64;
        let placements = offsets
            .into_iter()
            .map(|[y, x]| Placement {
                pose: Pose::new(y as f64, x as f64),
                bounds: full,
                weight,
                evidence: json!({"source": NATIVE_DELIVERY}),
                padding: Vec::new(),
            })
            .collect();
        return finish(trace, placements, result, 1.0);
    }

    if candidate.h >= target.h && candidate.w >= target.w && candidate.shape() != target.shape() {
        let mut attempt = json!({"method": NATIVE_REFERENCE});
        let offsets = exact_offsets(target, candidate, &mut attempt);
        let result = attempt["result"].clone();
        append_stage(&mut trace, attempt);
        if !offsets.is_empty() {
            let (content, strips) = proposed_content(candidate);
            let weight = 1.0 / offsets.len() as f64;
            let placements = offsets
                .into_iter()
                .map(|[y, x]| {
                    let pose = Pose::new(-(y as f64), -(x as f64));
                    let (bounds, padding) =
                        verify_strips(candidate, target, content, &strips, pose, true);
                    Placement {
                        pose,
                        bounds,
                        weight,
                        evidence: json!({"source": NATIVE_REFERENCE}),
                        padding,
                    }
                })
                .collect();
            return finish(trace, placements, result, 1.0);
        }
    }

    let (content, strips) = proposed_content(candidate);
    append_stage(
        &mut trace,
        json!({
            "method": "exact_border_runs",
            "proposed_content_box_xyxy": [content[1], content[0], content[3], content[2]],
            "runs": strips.iter().map(Strip::describe).collect::<Vec<_>>(),
        }),
    );
    let crop = candidate.crop(content);
    let mut offsets = Vec::new();
    if !strips.is_empty() {
        let mut attempt = json!({"method": EXACT_INTERIOR});
        offsets = exact_offsets(crop, target, &mut attempt);
        append_stage(&mut trace, attempt);
    }
    let origin = [content[0] as f64, content[1] as f64];
    let exact = !offsets.is_empty();
    let (hypotheses, confidence, reason) = if exact {
        let entries = offsets
            .into_iter()
            .map(|[y, x]| {
                (
                    Pose::new(y as f64, x as f64),
                    1.0,
                    json!({"source": EXACT_INTERIOR}),
                )
            })
            .collect();
        (entries, 1.0, "exact_interior")
    } else {
        let near_y = ((target.h as f64 - candidate.h as f64) / 2.0).floor();
        let near_x = ((target.w as f64 - candidate.w as f64) / 2.0).floor();
        let native = Pose::new(near_y + origin[0], near_x + origin[1]);
        let found = search::approximate(crop, target, &mut trace, native, origin);
        (found.hypotheses, found.confidence, found.reason)
    };
    let total: f64 = hypotheses.iter().map(|(_, weight, _)| weight).sum();
    let placements = hypotheses
        .into_iter()
        .map(|(pose, weight, evidence)| {
            let pose = pose.move_origin([-origin[0], -origin[1]]);
            let (bounds, padding) = verify_strips(candidate, target, content, &strips, pose, exact);
            Placement {
                pose,
                bounds,
                weight: confidence * weight / total,
                evidence,
                padding,
            }
        })
        .collect();
    finish(trace, placements, json!(reason), confidence)
}

pub(crate) fn overlap(placement: &Placement, [h, w]: [usize; 2]) -> (f64, f64, Vec<[f64; 2]>) {
    let [y, x, y1, x1] = placement.bounds.map(|v| v as f64 - 0.5);
    let mut vertices = [[y, x], [y, x1], [y1, x1], [y1, x]]
        .into_iter()
        .map(|point| placement.pose.transform(point))
        .collect::<Vec<_>>();
    for (axis, size) in [h, w].into_iter().enumerate() {
        for (bound, sign) in [(-0.5, 1.0), (size as f64 - 0.5, -1.0)] {
            let Some(&last) = vertices.last() else {
                break;
            };
            let mut previous = last;
            let mut clipped = Vec::with_capacity(vertices.len() + 1);
            for vertex in vertices {
                let p = sign * (previous[axis] - bound);
                let v = sign * (vertex[axis] - bound);
                if (p < 0.0) != (v < 0.0) {
                    let fraction = p / (p - v);
                    clipped.push(std::array::from_fn(|i| {
                        previous[i] + (vertex[i] - previous[i]) * fraction
                    }));
                }
                if v >= 0.0 {
                    clipped.push(vertex);
                }
                previous = vertex;
            }
            vertices = clipped;
        }
    }
    let area = if vertices.len() < 3 {
        0.0
    } else {
        let [y0, x0] = vertices[0];
        let (mut left, mut right) = (0.0, 0.0);
        for (current, previous) in vertices
            .iter()
            .zip(vertices.iter().cycle().skip(vertices.len() - 1))
        {
            left += (current[0] - y0) * (previous[1] - x0);
            right += (current[1] - x0) * (previous[0] - y0);
        }
        (left - right).abs() / 2.0
    };
    let geometry = (area / (h * w) as f64).min(1.0);
    let resolution = (geometry / placement.pose.scale.powi(2)).min(1.0);
    (
        geometry,
        resolution,
        vertices.into_iter().map(|[y, x]| [x, y]).collect(),
    )
}

fn translate_bounds([y, x, y1, x1]: Bounds, [dy, dx]: [isize; 2], [h, w]: [usize; 2]) -> Bounds {
    let shift = |v: usize, delta: isize, limit: usize| v.saturating_add_signed(delta).min(limit);
    [
        shift(y, dy, h),
        shift(x, dx, w),
        shift(y1, dy, h),
        shift(x1, dx, w),
    ]
}

pub(crate) fn native_overlap(placement: &Placement, shape: [usize; 2]) -> Bounds {
    let offset = placement
        .pose
        .integer_offset()
        .expect("exact placement offset");
    translate_bounds(placement.bounds, offset, shape)
}

pub(crate) fn project<'a>(
    candidate: ImageView<'a>,
    [h, w]: [usize; 2],
    placement: &Placement,
) -> (Cow<'a, [f32]>, Option<Vec<u8>>) {
    let [top, left, _, _] = placement.bounds;
    let pose = placement.pose.move_origin([top as f64, left as f64]);
    let content = candidate.crop(placement.bounds);
    if let Some([dy, dx]) = pose.integer_offset() {
        let [y, x, y1, x1] = translate_bounds(content.bounds(), [dy, dx], [h, w]);
        let full = [y, x, y1, x1] == [0, 0, h, w];
        if full && content.stride == 3 * w {
            let first_row = 0usize.saturating_add_signed(-dy);
            let start = first_row * content.stride;
            return (Cow::Borrowed(&content.data[start..start + h * w * 3]), None);
        }
        let mut aligned = vec![0.0; h * w * 3];
        let mut support = (!full).then(|| vec![0u8; h * w]);
        if y < y1 && x < x1 {
            let start = x.saturating_add_signed(-dx) * 3;
            let end = start + (x1 - x) * 3;
            for row in y..y1 {
                let source = content.row(row.saturating_add_signed(-dy));
                aligned[(row * w + x) * 3..(row * w + x1) * 3].copy_from_slice(&source[start..end]);
                if let Some(support) = &mut support {
                    support[row * w + x..row * w + x1].fill(1);
                }
            }
        }
        return (Cow::Owned(aligned), support);
    }
    let [[a, b], [c, d]] = pose.matrix();
    let determinant = a * d - b * c;
    let inverse = [
        [d / determinant, -b / determinant],
        [-c / determinant, a / determinant],
    ];
    let offset = map(inverse, [-pose.y, -pose.x]);
    let mut aligned = vec![0.0; h * w * 3];
    let mut support = vec![0u8; h * w];
    let row = |(y, (pixels, coverage)): (usize, (&mut [f32], &mut [u8]))| {
        for (x, (rgb, present)) in pixels
            .as_chunks_mut::<3>()
            .0
            .iter_mut()
            .zip(coverage)
            .enumerate()
        {
            let [dy, dx] = map(inverse, [y as f64, x as f64]);
            let dy = dy + offset[0];
            let dx = dx + offset[1];
            let sy = (dy + 0.5).floor();
            let sx = (dx + 0.5).floor();
            *present =
                u8::from(sy >= 0.0 && sy < content.h as f64 && sx >= 0.0 && sx < content.w as f64);
            rgb.copy_from_slice(&content.sample(dy, dx));
        }
    };
    if h * w >= 262_144 {
        parallel(|| {
            aligned
                .par_chunks_exact_mut(w * 3)
                .zip(support.par_chunks_exact_mut(w))
                .enumerate()
                .for_each(row)
        });
    } else {
        aligned
            .chunks_exact_mut(w * 3)
            .zip(support.chunks_exact_mut(w))
            .enumerate()
            .for_each(row);
    }
    (Cow::Owned(aligned), support.contains(&0).then_some(support))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_search_keeps_sparse_match_after_repetitive_anchors() {
        let (h, w) = (480, 640);
        let mut pixels = vec![0.0; h * w * 3];
        pixels[(311 * w + 431) * 3..][..3].copy_from_slice(&[0.3, 0.5, 0.7]);
        let target = ImageView::new(&pixels, h, w);
        let crop = target.crop([300, 400, 420, 550]);
        let mut needle: Vec<_> = (0..crop.h)
            .flat_map(|row| crop.row(row).iter().copied())
            .collect();
        needle[0] = -0.0;
        let (placements, trace) = align(ImageView::new(&needle, crop.h, crop.w), target);
        assert_eq!(placements.len(), 1);
        assert_eq!(placements[0].pose, Pose::new(300.0, 400.0));
        assert_eq!(trace["stages"][0]["fallback"], "rolling_rgb_rectangle_hash");
        assert_eq!(trace["status"], "matched");
    }

    #[test]
    fn flat_patch_search_prefers_center_among_exact_offsets() {
        let mut pixels = vec![0.0; 8 * 10 * 3];
        pixels[..3].fill(1.0);
        let target = ImageView::new(&pixels, 8, 10);
        let needle = ImageView::new(&[0.0; 2 * 4 * 3], 2, 4);
        let (placements, trace) = align(needle, target);
        assert_eq!(placements[0].pose, Pose::new(3.0, 3.0));
        assert_eq!(trace["stages"][0]["equivalent_offsets"], 48);
    }

    #[test]
    fn exact_oversized_delivery_excludes_verified_padding() {
        let (h, w) = (10, 13);
        let source: Vec<_> = (0..h * w * 3).map(|i| (i + 1) as f32 / 512.0).collect();
        let target = ImageView::new(&source, h, w);
        let (oh, ow) = (h + 4, w + 4);
        let mut padded = vec![0.0; oh * ow * 3];
        for row in 0..h {
            let start = ((row + 2) * ow + 2) * 3;
            padded[start..start + w * 3].copy_from_slice(target.row(row));
        }
        let candidate = ImageView::new(&padded, oh, ow);
        let (placements, trace) = align(candidate, target);
        assert_eq!(trace["status"], "matched");
        assert_eq!(placements[0].pose, Pose::new(-2.0, -2.0));
        assert_eq!(placements[0].bounds, [2, 2, h + 2, w + 2]);
        assert!(
            placements[0]
                .padding
                .iter()
                .all(|part| part["decision"] == "excluded")
        );
        assert_eq!(native_overlap(&placements[0], target.shape()), [0, 0, h, w]);
        assert_eq!(overlap(&placements[0], target.shape()).0, 1.0);
        let (pixels, support) = project(candidate, target.shape(), &placements[0]);
        assert_eq!(pixels, source);
        assert!(support.is_none());
    }

    #[test]
    fn fractional_projection_uses_pixel_center_support() {
        let colors: Vec<_> = (0..3 * 4).flat_map(|i| [i as f32; 3]).collect();
        let candidate = ImageView::new(&colors, 3, 4);
        let placement = Placement {
            pose: Pose::new(0.0, 0.5),
            bounds: candidate.bounds(),
            weight: 1.0,
            evidence: Value::Null,
            padding: Vec::new(),
        };
        let (geometry, resolution, _) = overlap(&placement, [3, 5]);
        assert_eq!(geometry, 0.8);
        assert_eq!(resolution, geometry);
        let (pixels, support) = project(candidate, [3, 5], &placement);
        assert_eq!(
            &pixels[..15],
            &[
                0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.5, 1.5, 1.5, 2.5, 2.5, 2.5, 3.0, 3.0, 3.0
            ]
        );
        assert_eq!(support.unwrap(), [1, 1, 1, 1, 0].repeat(3));
    }
}
