use numpy::ndarray::{Array2, ArrayView3, Axis, Dimension};
use numpy::{
    Element, IntoPyArray, PyArray, PyArray2, PyArrayMethods, PyReadonlyArray2, PyReadonlyArray3,
    PyReadwriteArray1, PyReadwriteArray2, PyUntypedArrayMethods,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::prelude::*;

use crate::accelerator::TilePlan;
use crate::parallel;

type RgbPixel = [f32; 3];
type Channels = [f64; 3];

fn aligned<T: Element, D: Dimension>(array: &Bound<'_, PyArray<T, D>>) -> bool {
    (array.data() as usize).is_multiple_of(align_of::<T>())
        && array
            .strides()
            .iter()
            .zip(array.shape())
            .all(|(stride, &len)| len <= 1 || stride.unsigned_abs().is_multiple_of(size_of::<T>()))
}

enum RgbLayout<'a> {
    Packed(&'a [RgbPixel]),
    Padded(ArrayView3<'a, f32>),
}

struct Rgb<'a> {
    layout: RgbLayout<'a>,
    height: usize,
    width: usize,
}

impl<'a> Rgb<'a> {
    fn from_packed(samples: &'a [f32], height: usize, width: usize) -> Self {
        Self {
            layout: RgbLayout::Packed(samples.as_chunks::<3>().0),
            height,
            width,
        }
    }

    fn new(array: &'a PyReadonlyArray3<'_, f32>) -> PyResult<Self> {
        let &[height, width, channels] = array.shape() else {
            unreachable!("PyReadonlyArray3 has exactly three axes")
        };
        let &[row_stride, pixel_stride, channel_stride] = array.strides() else {
            unreachable!("PyReadonlyArray3 has exactly three strides")
        };
        let sample_bytes = size_of::<f32>() as isize;
        let pixel_bytes = 3 * sample_bytes;
        if height == 0
            || width == 0
            || channels != 3
            || !aligned(array)
            || channel_stride != sample_bytes
            || (width > 1 && pixel_stride != pixel_bytes)
            || (height > 1 && row_stride < width as isize * pixel_bytes)
        {
            return Err(PyValueError::new_err(
                "Expected an aligned row-packed RGB array",
            ));
        }
        let view = array.as_array();
        let layout = match view.to_slice() {
            Some(samples) => RgbLayout::Packed(samples.as_chunks::<3>().0),
            None => RgbLayout::Padded(view),
        };
        Ok(Self {
            layout,
            height,
            width,
        })
    }

    fn len(&self) -> usize {
        self.height * self.width
    }

    fn row(&self, y: usize) -> &'a [RgbPixel] {
        match self.layout {
            RgbLayout::Packed(pixels) => &pixels[y * self.width..(y + 1) * self.width],
            RgbLayout::Padded(view) => {
                view.index_axis_move(Axis(0), y)
                    .to_slice()
                    .expect("RGB rows are packed")
                    .as_chunks::<3>()
                    .0
            }
        }
    }

    fn span(&self, start: usize, len: usize) -> &'a [RgbPixel] {
        if let RgbLayout::Packed(pixels) = self.layout {
            return &pixels[start..start + len];
        }
        let col = start % self.width;
        let row = self.row(start / self.width);
        &row[col..col + len.min(self.width - col)]
    }
}

fn validate_pair(candidate: &Rgb<'_>, target: &Rgb<'_>) -> PyResult<()> {
    if (candidate.height, candidate.width) != (target.height, target.width) {
        return Err(PyValueError::new_err("RGB array shapes do not match"));
    }
    Ok(())
}

fn support_slice<'a>(
    support: &'a Option<PyReadonlyArray2<'_, u8>>,
    rgb: &Rgb<'_>,
) -> PyResult<Option<&'a [u8]>> {
    support
        .as_ref()
        .map(|mask| {
            if mask.shape() != [rgb.height, rgb.width] || !mask.is_c_contiguous() {
                return Err(PyValueError::new_err(
                    "Expected packed support on the RGB grid",
                ));
            }
            mask.as_slice().map_err(Into::into)
        })
        .transpose()
}

#[inline]
fn xyz_pixel([r, g, b]: RgbPixel, matrix: &[RgbPixel; 3]) -> RgbPixel {
    let r = r.clamp(0.0, 1.0);
    let g = g.clamp(0.0, 1.0);
    let b = b.clamp(0.0, 1.0);
    matrix.map(|[x, y, z]| (r * x + g * y) + b * z)
}

#[pyfunction]
fn lab_xyz(
    py: Python<'_>,
    images: Vec<PyReadonlyArray3<'_, f32>>,
    weights: PyReadonlyArray2<'_, f32>,
    mut output: PyReadwriteArray2<'_, f32>,
    start: usize,
) -> PyResult<()> {
    let &[planes, count] = output.shape() else {
        unreachable!("PyReadwriteArray2 has exactly two axes")
    };
    if !matches!((images.len(), planes), (1, 3) | (2, 6))
        || weights.shape() != [3, 3]
        || !weights.is_c_contiguous()
        || !output.is_c_contiguous()
        || !aligned(&weights)
        || !aligned(&output)
    {
        return Err(PyValueError::new_err(
            "XYZ array shapes or layouts do not match",
        ));
    }
    let images = images.iter().map(Rgb::new).collect::<PyResult<Vec<_>>>()?;
    if images
        .iter()
        .any(|rgb| start > rgb.len() || count > rgb.len() - start)
    {
        return Err(PyValueError::new_err("XYZ work exceeds the RGB grid"));
    }
    if count == 0 {
        return Ok(());
    }
    let matrix = weights.as_slice()?.as_chunks::<3>().0.try_into().unwrap();
    let output = output.as_slice_mut()?;
    py.detach(move || {
        for (rgb, xyz) in images.iter().zip(output.chunks_exact_mut(3 * count)) {
            let (x, yz) = xyz.split_at_mut(count);
            let (y, z) = yz.split_at_mut(count);
            let mut written = 0;
            while written < count {
                let pixels = rgb.span(start + written, count - written);
                let end = written + pixels.len();
                let coordinates = x[written..end]
                    .iter_mut()
                    .zip(&mut y[written..end])
                    .zip(&mut z[written..end]);
                for (&pixel, ((x, y), z)) in pixels.iter().zip(coordinates) {
                    [*x, *y, *z] = xyz_pixel(pixel, matrix);
                }
                written = end;
            }
        }
    });
    Ok(())
}

const LAB_THRESHOLD: f32 = ((6.0_f64 / 29.0) * (6.0_f64 / 29.0) * (6.0_f64 / 29.0)) as f32;

#[inline]
fn lab_curve(xyz: f32, cube: f32) -> f32 {
    if xyz > LAB_THRESHOLD {
        cube
    } else {
        xyz * (841.0_f64 / 108.0) as f32 + (4.0_f64 / 29.0) as f32
    }
}

#[inline]
fn lab_components([x, y, z]: RgbPixel) -> RgbPixel {
    [lab_lightness(y), 500.0 * (x - y), 200.0 * (y - z)]
}

#[inline]
fn lab_lightness(y: f32) -> f32 {
    116.0 * y - 16.0
}

struct LabInput<'a> {
    x: &'a [f32],
    y: &'a [f32],
    z: &'a [f32],
    cx: &'a [f32],
    cy: &'a [f32],
    cz: &'a [f32],
}

impl<'a> LabInput<'a> {
    fn new(xyz: &'a [f32], cubes: &'a [f32], count: usize) -> Self {
        let (x, yz) = xyz.split_at(count);
        let (y, z) = yz.split_at(count);
        let (cx, cyz) = cubes.split_at(count);
        let (cy, cz) = cyz.split_at(count);
        Self {
            x,
            y,
            z,
            cx,
            cy,
            cz,
        }
    }

    #[inline]
    fn lab(&self, i: usize) -> RgbPixel {
        let x = lab_curve(self.x[i], self.cx[i]);
        let y = lab_curve(self.y[i], self.cy[i]);
        let z = lab_curve(self.z[i], self.cz[i]);
        lab_components([x, y, z])
    }
}

#[pyfunction]
fn lab_planes(
    py: Python<'_>,
    xyz: PyReadonlyArray2<'_, f32>,
    cubes: PyReadonlyArray2<'_, f32>,
    mut output: Vec<PyReadwriteArray1<'_, f32>>,
) -> PyResult<()> {
    let &[planes, count] = xyz.shape() else {
        unreachable!("PyReadonlyArray2 has exactly two axes")
    };
    if cubes.shape() != [planes, count]
        || !matches!((planes, output.len()), (3, 3) | (6, 5))
        || !aligned(&xyz)
        || !aligned(&cubes)
        || !xyz.is_c_contiguous()
        || !cubes.is_c_contiguous()
        || output
            .iter()
            .any(|plane| plane.len() != count || !plane.is_c_contiguous() || !aligned(plane))
    {
        return Err(PyValueError::new_err(
            "Lab array shapes or layouts do not match",
        ));
    }
    let (xyz_c, xyz_t) = xyz.as_slice()?.split_at(3 * count);
    let (cubes_c, cubes_t) = cubes.as_slice()?.split_at(3 * count);
    let candidate = LabInput::new(xyz_c, cubes_c, count);
    match output.as_mut_slice() {
        [lightness, a, b] => {
            let lightness = lightness.as_slice_mut()?;
            let a = a.as_slice_mut()?;
            let b = b.as_slice_mut()?;
            py.detach(move || {
                for i in 0..count {
                    [lightness[i], a[i], b[i]] = candidate.lab(i);
                }
            });
        }
        [cl, tl, delta, delta_a, delta_b] => {
            let target = LabInput::new(xyz_t, cubes_t, count);
            let cl = cl.as_slice_mut()?;
            let tl = tl.as_slice_mut()?;
            let delta = delta.as_slice_mut()?;
            let delta_a = delta_a.as_slice_mut()?;
            let delta_b = delta_b.as_slice_mut()?;
            py.detach(move || {
                for i in 0..count {
                    let [c, ca, cb] = candidate.lab(i);
                    let [t, ta, tb] = target.lab(i);
                    cl[i] = c;
                    tl[i] = t;
                    delta[i] = c - t;
                    delta_a[i] = ca - ta;
                    delta_b[i] = cb - tb;
                }
            });
        }
        _ => unreachable!("lab planes have three or five outputs"),
    }
    Ok(())
}

#[inline]
fn select_max(a: f64, b: f64) -> f64 {
    if a > b { a } else { b }
}

#[inline]
fn channel_range_damage(c: f32, t: f32) -> f64 {
    // Subtract in float64 so differences between finite float32 samples stay finite.
    let c = f64::from(c);
    let t = f64::from(t);
    let c_display = c.clamp(0.0, 1.0);
    let t_display = t.clamp(0.0, 1.0);
    let headroom = ((c - c_display) - (t - t_display)).abs();
    let distance = if c_display < 0.5 {
        c_display
    } else {
        1.0 - c_display
    };
    let endpoint = 1.0 - (distance / 0.012).clamp(0.0, 1.0);
    let clipping = (c_display - t_display).abs() * endpoint;
    let error = select_max(headroom, clipping);
    error / (error + 0.075)
}

#[inline]
fn range_pixel([cr, cg, cb]: RgbPixel, [tr, tg, tb]: RgbPixel) -> f32 {
    let red = channel_range_damage(cr, tr);
    let green = channel_range_damage(cg, tg);
    let blue = channel_range_damage(cb, tb);
    select_max(select_max(red, green), blue) as f32
}

fn range_row(candidate: &[RgbPixel], target: &[RgbPixel], output: &mut [f32]) {
    for ((&c, &t), result) in candidate.iter().zip(target).zip(output) {
        *result = range_pixel(c, t);
    }
}

#[pyfunction]
fn range_damage<'py>(
    py: Python<'py>,
    candidate: PyReadonlyArray3<'py, f32>,
    target: PyReadonlyArray3<'py, f32>,
) -> PyResult<Bound<'py, PyArray2<f32>>> {
    let candidate = Rgb::new(&candidate)?;
    let target = Rgb::new(&target)?;
    validate_pair(&candidate, &target)?;
    let output = py.detach(move || {
        let mut output = Array2::zeros((target.height, target.width));
        let rows = output.as_slice_mut().unwrap();
        let run = |(y, row): (usize, &mut [f32])| range_row(candidate.row(y), target.row(y), row);
        if rows.len() >= 65_536 {
            parallel(|| rows.par_chunks_mut(target.width).enumerate().for_each(run));
        } else {
            rows.chunks_mut(target.width).enumerate().for_each(run);
        }
        output
    });
    Ok(output.into_pyarray(py))
}

#[inline]
fn add([ar, ag, ab]: Channels, [br, bg, bb]: Channels) -> Channels {
    [ar + br, ag + bg, ab + bb]
}

fn means(sums: Channels, count: usize) -> Option<(f64, f64, f64)> {
    (count != 0).then(|| {
        let [r, g, b] = sums.map(|sum| sum / count as f64);
        (r, g, b)
    })
}

fn channel_means(image: &Rgb<'_>, support: Option<&[u8]>) -> Option<(f64, f64, f64)> {
    // Preserve the row-major sum per channel when extended-range terms cancel.
    let mut sums = [0.0; 3];
    let mut count = 0;
    if let Some(mask) = support {
        for (y, covered) in mask.chunks_exact(image.width).enumerate() {
            for (&pixel, &valid) in image.row(y).iter().zip(covered) {
                if valid != 0 {
                    sums = add(sums, pixel.map(f64::from));
                    count += 1;
                }
            }
        }
    } else {
        for y in 0..image.height {
            for &pixel in image.row(y) {
                sums = add(sums, pixel.map(f64::from));
            }
        }
        count = image.len();
    }
    means(sums, count)
}

#[pyfunction]
fn rgb_means(
    py: Python<'_>,
    image: PyReadonlyArray3<'_, f32>,
    support: Option<PyReadonlyArray2<'_, u8>>,
) -> PyResult<Option<(f64, f64, f64)>> {
    let image = Rgb::new(&image)?;
    let support = support_slice(&support, &image)?;
    Ok(py.detach(move || channel_means(&image, support)))
}

#[inline]
fn squared_difference(c: RgbPixel, t: RgbPixel) -> Channels {
    std::array::from_fn(|i| {
        let delta = f64::from(c[i]) - f64::from(t[i]);
        delta * delta
    })
}

fn for_each_difference(
    candidate: &Rgb<'_>,
    target: &Rgb<'_>,
    mut start: usize,
    mut count: usize,
    mut apply: impl FnMut(Channels),
) {
    while count != 0 {
        let c = candidate.span(start, count);
        let t = target.span(start, c.len());
        for (&c, &t) in c.iter().zip(t) {
            apply(squared_difference(c, t));
        }
        start += t.len();
        count -= t.len();
    }
}

fn pairwise_square_sum(
    candidate: &Rgb<'_>,
    target: &Rgb<'_>,
    start: usize,
    count: usize,
) -> Channels {
    // Match NumPy's contiguous float64 add reduction. Its tree uses blocks
    // of up to 128 values, each reduced across eight accumulators.
    if count > 128 {
        let middle = (count / 2) & !7;
        let left = || pairwise_square_sum(candidate, target, start, middle);
        let right = || pairwise_square_sum(candidate, target, start + middle, count - middle);
        let (a, b) = if count >= 65_536 {
            rayon::join(left, right)
        } else {
            (left(), right())
        };
        return add(a, b);
    }

    let mut lanes = [[0.0; 3]; 8];
    let mut lane = 0;
    let full_lanes = count & !7;
    for_each_difference(candidate, target, start, full_lanes, |squares| {
        lanes[lane] = add(lanes[lane], squares);
        lane = (lane + 1) & 7;
    });
    let [a, b, c, d, e, f, g, h] = lanes;
    let mut sum = add(add(add(a, b), add(c, d)), add(add(e, f), add(g, h)));
    for_each_difference(
        candidate,
        target,
        start + full_lanes,
        count - full_lanes,
        |squares| {
            sum = add(sum, squares);
        },
    );
    sum
}

fn mse(candidate: &Rgb<'_>, target: &Rgb<'_>, support: Option<&[u8]>) -> Option<(f64, f64, f64)> {
    let calculate = || {
        if let Some(mask) = support {
            let mut sums = [0.0; 3];
            let mut count = 0;
            let mut start = 0;
            // A masked reduction adds the sum of each consecutive run of covered pixels.
            for run in mask.chunk_by(|a, b| (*a == 0) == (*b == 0)) {
                if run[0] != 0 {
                    sums = add(
                        sums,
                        pairwise_square_sum(candidate, target, start, run.len()),
                    );
                    count += run.len();
                }
                start += run.len();
            }
            means(sums, count)
        } else {
            let count = target.len();
            means(pairwise_square_sum(candidate, target, 0, count), count)
        }
    };
    if target.len() >= 65_536 {
        parallel(calculate)
    } else {
        calculate()
    }
}

pub(crate) fn rgb_diagnostics(
    height: usize,
    width: usize,
    candidate: &[f32],
    target: &[f32],
    support: Option<&[u8]>,
) -> (f64, f64) {
    let candidate = Rgb::from_packed(candidate, height, width);
    let target = Rgb::from_packed(target, height, width);
    let (r, g, b) = mse(&candidate, &target, support).expect("nonempty measured support");
    let rms = (((r + g) + b) / 3.0).sqrt();
    let exposure = |image| {
        let (r, g, b) = channel_means(image, support).expect("nonempty measured support");
        let [yr, yg, yb] = XYZ[1].map(f64::from);
        (r.max(0.0) * yr + g.max(0.0) * yg) + b.max(0.0) * yb
    };
    let c = exposure(&candidate);
    let t = exposure(&target);
    (rms, ((c + 1e-5) / (t + 1e-5)).log2())
}

#[pyfunction]
fn rgb_mse(
    py: Python<'_>,
    candidate: PyReadonlyArray3<'_, f32>,
    target: PyReadonlyArray3<'_, f32>,
    support: Option<PyReadonlyArray2<'_, u8>>,
) -> PyResult<Option<(f64, f64, f64)>> {
    let candidate = Rgb::new(&candidate)?;
    let target = Rgb::new(&target)?;
    validate_pair(&candidate, &target)?;
    let support = support_slice(&support, &target)?;
    Ok(py.detach(move || mse(&candidate, &target, support)))
}

// Linear sRGB to D65 XYZ, normalized to the reference white.
const XYZ: [RgbPixel; 3] = [
    [
        0.4124564 / 0.95047,
        0.3575761 / 0.95047,
        0.1804375 / 0.95047,
    ],
    [0.2126729, 0.7151522, 0.0721750],
    [
        0.0193339 / 1.08883,
        0.119_192 / 1.08883,
        0.9503041 / 1.08883,
    ],
];

#[inline]
fn lab_root(value: f32) -> f32 {
    // On the positive normal XYZ range, the float bits approximate log2.
    // This seed is within 3.5% there. Two Halley refinements in double
    // precision reduce the error below a float32 rounding unit.
    const CUBE_ROOT_SEED: u32 = 709_921_077;
    let sample = f64::from(value);
    let mut root = f64::from(f32::from_bits(value.to_bits() / 3 + CUBE_ROOT_SEED));
    for _ in 0..2 {
        let cube = root * root * root;
        root *= (cube + 2.0 * sample) / (2.0 * cube + sample);
    }
    root as f32
}

#[inline]
fn lab_transfer(value: f32) -> f32 {
    if value > LAB_THRESHOLD {
        lab_root(value)
    } else {
        lab_curve(value, 0.0)
    }
}

#[inline]
fn lab_pixel(rgb: RgbPixel) -> RgbPixel {
    lab_components(xyz_pixel(rgb, &XYZ).map(lab_transfer))
}

#[derive(Default)]
struct PixelSums {
    delta: Channels,
    count: usize,
}

impl PixelSums {
    fn combine(self, other: Self) -> Self {
        Self {
            delta: add(self.delta, other.delta),
            count: self.count + other.count,
        }
    }

    fn mean_delta(self) -> Channels {
        self.delta.map(|value| value / self.count as f64)
    }
}

fn compare_chunk(
    candidate: &[RgbPixel],
    target: &[RgbPixel],
    support: Option<&[u8]>,
    [cl, tl, delta, color, range]: [&mut [f32]; 5],
) -> PixelSums {
    let mut sums = PixelSums::default();
    let length = target.len();
    for i in 0..length {
        let c = candidate[i];
        let t = target[i];
        let [lc, ac, bc] = lab_pixel(c);
        let [lt, at, bt] = lab_pixel(t);
        let [dl, da, db] = [lc - lt, ac - at, bc - bt];
        let [a64, b64] = [f64::from(da), f64::from(db)];
        let chroma = (a64 * a64 + b64 * b64).sqrt() as f32;
        cl[i] = lc;
        tl[i] = lt;
        delta[i] = dl;
        color[i] = chroma / (chroma + 10.0);
        range[i] = range_pixel(c, t);
        if support.is_none_or(|mask| mask[i] != 0) {
            sums.delta = add(sums.delta, [dl, da, db].map(f64::from));
            sums.count += 1;
        }
    }
    sums
}

pub(crate) struct Comparison {
    pub(crate) planes: [Vec<f32>; 5],
    pub(crate) mean_delta: [f64; 3],
}

pub(crate) fn compare(candidate: &[f32], target: &[f32], support: Option<&[u8]>) -> Comparison {
    let candidate = candidate.as_chunks::<3>().0;
    let target = target.as_chunks::<3>().0;
    let mut planes = std::array::from_fn(|_| vec![0.0; target.len()]);
    let [cl, tl, delta, color, range] = &mut planes;
    let sums = compare_into(candidate, target, support, [cl, tl, delta, color, range]);
    Comparison {
        planes,
        mean_delta: sums.mean_delta(),
    }
}

fn compare_into(
    candidate: &[RgbPixel],
    target: &[RgbPixel],
    support: Option<&[u8]>,
    [cl, tl, delta, color, range]: [&mut [f32]; 5],
) -> PixelSums {
    const BLOCK: usize = 16_384;
    if target.len() >= 2 * BLOCK {
        parallel(|| {
            cl.par_chunks_mut(BLOCK)
                .zip(tl.par_chunks_mut(BLOCK))
                .zip(delta.par_chunks_mut(BLOCK))
                .zip(color.par_chunks_mut(BLOCK))
                .zip(range.par_chunks_mut(BLOCK))
                .enumerate()
                .map(|(i, ((((cl, tl), delta), color), range))| {
                    let first = i * BLOCK;
                    let rows = first..first + cl.len();
                    compare_chunk(
                        &candidate[rows.clone()],
                        &target[rows.clone()],
                        support.map(|mask| &mask[rows]),
                        [cl, tl, delta, color, range],
                    )
                })
                .collect::<Vec<_>>()
        })
        .into_iter()
        .fold(PixelSums::default(), PixelSums::combine)
    } else {
        compare_chunk(candidate, target, support, [cl, tl, delta, color, range])
    }
}

pub(crate) struct PackedLightness {
    samples: Vec<f32>,
}

impl PackedLightness {
    fn new(area: usize, reserve_mask: bool) -> Self {
        let samples_len = 3 * area;
        let mut samples = vec![0.0; samples_len + if reserve_mask { area } else { 0 }];
        samples.truncate(samples_len);
        Self { samples }
    }

    pub(crate) fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    fn planes_mut(&mut self) -> [&mut [f32]; 3] {
        let area = self.samples.len() / 3;
        let (candidate, tail) = self.samples.split_at_mut(area);
        let (target, delta) = tail.split_at_mut(area);
        [candidate, target, delta]
    }
}

pub(crate) struct PreparedComparison {
    pub(crate) tiles: Vec<PackedLightness>,
    pub(crate) color: Vec<f32>,
    pub(crate) range: Vec<f32>,
    pub(crate) mean_delta: Channels,
}

fn fill_lightness(candidate: &[RgbPixel], target: &[RgbPixel], [cl, tl, delta]: [&mut [f32]; 3]) {
    let lightness = |rgb| lab_lightness(lab_transfer(xyz_pixel(rgb, &XYZ)[1]));
    for (i, (&candidate, &target)) in candidate.iter().zip(target).enumerate() {
        cl[i] = lightness(candidate);
        tl[i] = lightness(target);
        delta[i] = cl[i] - tl[i];
    }
}

pub(crate) fn compare_prepared(
    candidate: &[f32],
    target: &[f32],
    height: usize,
    width: usize,
    support: Option<&[u8]>,
    plan: &[TilePlan],
) -> Option<PreparedComparison> {
    // Disjoint full-width cores can write the final color and range planes
    // in place. A horizontal split uses the bounded row-slice packing path.
    if plan.is_empty()
        || plan
            .iter()
            .any(|tile| tile.core[1] != 0 || tile.core[3] != width)
    {
        return None;
    }
    let candidate = candidate.as_chunks::<3>().0;
    let target = target.as_chunks::<3>().0;
    let mut color = vec![0.0; height * width];
    let mut range = vec![0.0; height * width];
    let mut color_left = color.as_mut_slice();
    let mut range_left = range.as_mut_slice();
    let mut tasks = Vec::with_capacity(plan.len());
    let mut row = 0;
    for tile in plan {
        let [top, _, bottom, _] = tile.core;
        let [first, _, last, _] = tile.halo;
        debug_assert_eq!(top, row);
        let length = (bottom - top) * width;
        let color_out;
        let range_out;
        (color_out, color_left) = color_left.split_at_mut(length);
        (range_out, range_left) = range_left.split_at_mut(length);
        tasks.push((
            first * width..last * width,
            top * width..bottom * width,
            color_out,
            range_out,
        ));
        row = bottom;
    }
    debug_assert_eq!(row, height);

    let prepare = |(halo, core, color, range): (
        std::ops::Range<usize>,
        std::ops::Range<usize>,
        &mut [f32],
        &mut [f32],
    )| {
        let mask = support.map(|support| &support[core.clone()]);
        let reserve_mask = support.is_some_and(|support| support[halo.clone()].contains(&0));
        let mut tile = PackedLightness::new(halo.len(), reserve_mask);
        let [cl, tl, delta] = tile.planes_mut();
        let start = core.start - halo.start;
        let end = core.end - halo.start;

        let sums = compare_into(
            &candidate[core.clone()],
            &target[core],
            mask,
            [
                &mut cl[start..end],
                &mut tl[start..end],
                &mut delta[start..end],
                color,
                range,
            ],
        );
        for rows in [0..start, end..halo.len()] {
            let input = halo.start + rows.start..halo.start + rows.end;
            fill_lightness(
                &candidate[input.clone()],
                &target[input],
                [
                    &mut cl[rows.clone()],
                    &mut tl[rows.clone()],
                    &mut delta[rows],
                ],
            );
        }
        (tile, sums)
    };
    let prepared: Vec<_> = if tasks.len() > 1 {
        parallel(|| tasks.into_par_iter().map(prepare).collect())
    } else {
        tasks.into_iter().map(prepare).collect()
    };
    let (tiles, stats): (Vec<_>, Vec<_>) = prepared.into_iter().unzip();
    let sums = stats
        .into_iter()
        .fold(PixelSums::default(), PixelSums::combine);
    Some(PreparedComparison {
        tiles,
        color,
        range,
        mean_delta: sums.mean_delta(),
    })
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(lab_xyz, module)?)?;
    module.add_function(wrap_pyfunction!(lab_planes, module)?)?;
    module.add_function(wrap_pyfunction!(range_damage, module)?)?;
    module.add_function(wrap_pyfunction!(rgb_means, module)?)?;
    module.add_function(wrap_pyfunction!(rgb_mse, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{XYZ, compare, compare_prepared, rgb_diagnostics};
    use crate::accelerator::TilePlan;

    #[test]
    fn exposure_preserves_row_major_cancellation_at_image_scale() {
        let width = 256;
        let height = 256;
        let target = vec![0.25; 3 * width * height];
        let mut candidate = target.clone();
        let halfway = 3 * width * height / 2;
        candidate[..3].fill(f32::MAX);
        candidate[halfway..halfway + 3].fill(-f32::MAX);

        let (_, exposure) = rgb_diagnostics(height, width, &candidate, &target, None);
        // Terms before the cancellation vanish in a linear reduction.
        let exposure_t = 0.25 * XYZ[1].iter().copied().map(f64::from).sum::<f64>();
        let exposure_c = exposure_t * (width * height / 2 - 1) as f64 / (width * height) as f64;
        assert!((exposure - ((exposure_c + 1e-5) / (exposure_t + 1e-5)).log2()).abs() < 1e-12);
    }

    #[test]
    fn prepared_payload_matches_full_image_planes_and_counts_only_cores() {
        let (height, width) = (173, 477);
        let count = height * width;
        let candidate = (0..3 * count)
            .map(|i| (i % 313) as f32 / 257.0 - 0.05)
            .collect::<Vec<_>>();
        let target = (0..3 * count)
            .map(|i| (i % 249) as f32 / 211.0)
            .collect::<Vec<_>>();
        let support = (0..count)
            .map(|i| if i % 17 == 0 { 0 } else { 255 })
            .collect::<Vec<_>>();
        let plan = [
            TilePlan {
                core: [0, 0, 71, width],
                halo: [0, 0, 111, width],
            },
            TilePlan {
                core: [71, 0, height, width],
                halo: [31, 0, height, width],
            },
        ];
        let baseline = compare(&candidate, &target, Some(&support));
        let packed = compare_prepared(&candidate, &target, height, width, Some(&support), &plan)
            .expect("full-width tiles are prepared");

        for (tile, plan) in packed.tiles.into_iter().zip(&plan) {
            let rows = plan.halo[0] * width..plan.halo[2] * width;
            let samples = tile.into_samples();
            assert_eq!(samples.len(), 3 * rows.len());
            assert!(samples.capacity() >= 4 * rows.len());
            for (samples, image) in samples.chunks_exact(rows.len()).zip(&baseline.planes[..3]) {
                assert_eq!(samples, &image[rows.clone()]);
            }
        }
        assert_eq!(packed.color, baseline.planes[3]);
        assert_eq!(packed.range, baseline.planes[4]);
        for (prepared, full_image) in packed.mean_delta.into_iter().zip(baseline.mean_delta) {
            assert!((prepared - full_image).abs() < 1e-12);
        }
    }
}
