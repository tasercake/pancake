use numpy::ndarray::{Array2, ArrayView3, Axis, Dimension};
use numpy::{
    Element, IntoPyArray, PyArray, PyArray2, PyArrayMethods, PyReadonlyArray2, PyReadonlyArray3,
    PyReadwriteArray1, PyReadwriteArray2, PyUntypedArrayMethods,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rayon::prelude::*;

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

struct Rgb<'a> {
    view: ArrayView3<'a, f32>,
    packed: Option<&'a [RgbPixel]>,
    height: usize,
    width: usize,
}

impl<'a> Rgb<'a> {
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
        let packed = view.to_slice().map(|samples| samples.as_chunks::<3>().0);
        Ok(Self {
            view,
            packed,
            height,
            width,
        })
    }

    fn len(&self) -> usize {
        self.height * self.width
    }

    fn row(&self, y: usize) -> &'a [RgbPixel] {
        self.view
            .index_axis_move(Axis(0), y)
            .to_slice()
            .expect("RGB rows are packed")
            .as_chunks::<3>()
            .0
    }

    fn span(&self, start: usize, len: usize) -> &'a [RgbPixel] {
        if let Some(pixels) = self.packed {
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
        [116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z)]
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

fn range_row(candidate: &[RgbPixel], target: &[RgbPixel], output: &mut [f32]) {
    for (([cr, cg, cb], [tr, tg, tb]), result) in candidate.iter().zip(target).zip(output) {
        let red = channel_range_damage(*cr, *tr);
        let green = channel_range_damage(*cg, *tg);
        let blue = channel_range_damage(*cb, *tb);
        *result = select_max(select_max(red, green), blue) as f32;
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

#[pyfunction]
fn rgb_means(
    py: Python<'_>,
    image: PyReadonlyArray3<'_, f32>,
    support: Option<PyReadonlyArray2<'_, u8>>,
) -> PyResult<Option<(f64, f64, f64)>> {
    let image = Rgb::new(&image)?;
    let support = support_slice(&support, &image)?;
    Ok(py.detach(move || {
        // NumPy's HWC sum over H and W is linear in row-major order per channel.
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
    }))
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
    Ok(py.detach(move || {
        let calculate = || {
            if let Some(mask) = support {
                let mut sums = [0.0; 3];
                let mut count = 0;
                let mut start = 0;
                // NumPy's where reduction adds the result of one reduction
                // per consecutive run of covered samples.
                for run in mask.chunk_by(|a, b| (*a == 0) == (*b == 0)) {
                    if run[0] != 0 {
                        sums = add(
                            sums,
                            pairwise_square_sum(&candidate, &target, start, run.len()),
                        );
                        count += run.len();
                    }
                    start += run.len();
                }
                means(sums, count)
            } else {
                let count = target.len();
                means(pairwise_square_sum(&candidate, &target, 0, count), count)
            }
        };
        if target.len() >= 65_536 {
            parallel(calculate)
        } else {
            calculate()
        }
    }))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(lab_xyz, module)?)?;
    module.add_function(wrap_pyfunction!(lab_planes, module)?)?;
    module.add_function(wrap_pyfunction!(range_damage, module)?)?;
    module.add_function(wrap_pyfunction!(rgb_means, module)?)?;
    module.add_function(wrap_pyfunction!(rgb_mse, module)?)?;
    Ok(())
}
