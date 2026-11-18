//! Validated geometry for registration, crop, and resampling.

/// A reference-to-source affine map in pixel-center coordinates.
///
/// Rows are `[a, b, tx]` and `[c, d, ty]`. Map a reference point
/// `(x, y)` to source pixel `(a*x + b*y + tx, c*x + d*y + ty)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    /// Matrix in x/y order, with implicit final row `[0, 0, 1]`.
    rows: [[f32; 3]; 2],
}

impl Affine {
    /// Leave both coordinates unchanged.
    pub const IDENTITY: Self = Self {
        rows: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
    };

    /// Construct a map from its two rows.
    ///
    /// # Errors
    ///
    /// Returns an error if any coefficient is nonfinite.
    pub fn from_matrix(rows: [[f32; 3]; 2]) -> Result<Self, String> {
        if rows.into_iter().flatten().all(f32::is_finite) {
            Ok(Self { rows })
        } else {
            Err("registration matrix must be finite".to_owned())
        }
    }

    /// Return the two rows in x/y order for geometry reporting.
    #[must_use]
    pub const fn matrix(self) -> [[f32; 3]; 2] {
        self.rows
    }

    /// Apply `after` to the reference coordinate, then apply `self`.
    ///
    /// # Errors
    ///
    /// Returns an error if the resulting coefficients overflow.
    pub fn compose(self, after: Self) -> Result<Self, String> {
        let [[a, b, horizontal_offset], [c, d, vertical_offset]] = after.rows;
        let rows = self.rows.map(|[from_x, from_y, translation]| {
            [
                from_x.mul_add(a, from_y * c),
                from_x.mul_add(b, from_y * d),
                from_x.mul_add(
                    horizontal_offset,
                    from_y.mul_add(vertical_offset, translation),
                ),
            ]
        });
        Self::from_matrix(rows)
    }

    /// Map source coordinates back to the reference.
    ///
    /// # Errors
    ///
    /// Returns an error for singular or nonfinite maps and arithmetic overflow.
    pub fn inverse(self) -> Result<Self, String> {
        let [[a, b, tx], [c, d, ty]] = self.rows;
        let determinant = a.mul_add(d, -(b * c));
        if !determinant.is_finite() || determinant.abs() <= f32::EPSILON {
            return Err("registration matrix is singular".to_owned());
        }
        Self::from_matrix([
            [
                d / determinant,
                -b / determinant,
                b.mul_add(ty, -(d * tx)) / determinant,
            ],
            [
                -c / determinant,
                a / determinant,
                c.mul_add(tx, -(a * ty)) / determinant,
            ],
        ])
    }

    /// Lift the normalized source canvas to actual source coordinates.
    ///
    /// Both dimension pairs are `[width, height]`. The half-pixel offset
    /// matches the thumbnail resize and aligns the centers of the edge pixels.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions or nonfinite coefficients.
    pub fn rescale_source(
        self,
        reference_dimensions: [usize; 2],
        source_dimensions: [usize; 2],
    ) -> Result<Self, String> {
        let [rw, rh] = reference_dimensions;
        let [sw, sh] = source_dimensions;
        let reference = Extent::new(rw, rh)?;
        let source = Extent::new(sw, sh)?;
        let x = f32::from(source.width) / f32::from(reference.width);
        let y = f32::from(source.height) / f32::from(reference.height);
        Self {
            rows: [
                [x, 0.0, 0.5_f32.mul_add(x, -0.5)],
                [0.0, y, 0.5_f32.mul_add(y, -0.5)],
            ],
        }
        .compose(self)
    }

    /// Apply the map to one x/y point.
    const fn point(self, column: f32, row: f32) -> [f32; 2] {
        let [[a, b, horizontal_offset], [c, d, vertical_offset]] = self.rows;
        [
            a.mul_add(column, b.mul_add(row, horizontal_offset)),
            c.mul_add(column, d.mul_add(row, vertical_offset)),
        ]
    }
}

/// Packed RGB source data borrowed for upload.
#[derive(Clone, Copy, Debug)]
pub struct Frame<'a> {
    /// Source pixel dimensions.
    pub(super) extent: Extent,
    /// Three neighboring f32 channel values per pixel.
    pub(super) samples: &'a [f32],
}

impl<'a> Frame<'a> {
    /// Borrow a row-major, interleaved RGB image.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid dimensions or a mismatched sample count.
    pub fn new(width: usize, height: usize, samples: &'a [f32]) -> Result<Self, String> {
        let extent = Extent::new(width, height)?;
        if extent.samples()? != samples.len() {
            return Err("registration frame does not match RGB dimensions".to_owned());
        }
        Ok(Self { extent, samples })
    }
}

/// Checked image dimensions small enough to convert exactly to f32.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Extent {
    /// Pixel columns.
    pub(super) width: u16,
    /// Pixel rows.
    pub(super) height: u16,
}

impl Extent {
    /// Require two nonempty dimensions in the resampler's coordinate range.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero or unaddressable axis.
    pub(super) fn new(width: usize, height: usize) -> Result<Self, String> {
        let width = u16::try_from(width).map_err(|_| "image width exceeds registration limit")?;
        let height =
            u16::try_from(height).map_err(|_| "image height exceeds registration limit")?;
        if width == 0 || height == 0 {
            return Err("registration dimensions must be nonzero".to_owned());
        }
        Ok(Self { width, height })
    }

    /// Compute the number of addressable pixels.
    ///
    /// # Errors
    ///
    /// Returns an error if the area exceeds host addressing.
    pub(super) fn area(self) -> Result<usize, String> {
        usize::from(self.width)
            .checked_mul(usize::from(self.height))
            .ok_or_else(|| "registration area exceeds address space".to_owned())
    }

    /// Compute packed RGB sample count within device addressing.
    ///
    /// # Errors
    ///
    /// Returns an error if the sample count overflows host or device addressing.
    pub(super) fn samples(self) -> Result<usize, String> {
        self.area()?
            .checked_mul(3)
            .filter(|count| u32::try_from(*count).is_ok())
            .ok_or_else(|| "registration RGB samples exceed device addressing".to_owned())
    }
}

/// An output region in the untrimmed reference coordinate system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Bounds {
    /// Reference column where the first output pixel begins.
    pub(super) x: u16,
    /// Reference row where the first output pixel begins.
    pub(super) y: u16,
    /// Size of the returned output.
    pub(super) extent: Extent,
}

impl Bounds {
    /// Create a nonempty output rectangle at the given reference origin.
    ///
    /// # Errors
    ///
    /// Returns an error if the rectangle exceeds addressable coordinates.
    pub fn new(x: usize, y: usize, width: usize, height: usize) -> Result<Self, String> {
        let extent = Extent::new(width, height)?;
        let x = u16::try_from(x).map_err(|_| "crop x exceeds registration coordinate limit")?;
        let y = u16::try_from(y).map_err(|_| "crop y exceeds registration coordinate limit")?;
        let _right = x
            .checked_add(extent.width)
            .ok_or("crop right exceeds registration limit")?;
        let _bottom = y
            .checked_add(extent.height)
            .ok_or("crop bottom exceeds registration limit")?;
        Ok(Self { x, y, extent })
    }

    /// First output column in the reference canvas.
    #[must_use]
    pub fn x(self) -> usize {
        usize::from(self.x)
    }

    /// First output row in the reference canvas.
    #[must_use]
    pub fn y(self) -> usize {
        usize::from(self.y)
    }

    /// Number of output columns.
    #[must_use]
    pub fn width(self) -> usize {
        usize::from(self.extent.width)
    }

    /// Number of output rows.
    #[must_use]
    pub fn height(self) -> usize {
        usize::from(self.extent.height)
    }
}

/// Compute a symmetric crop that removes the largest corner displacement per axis.
///
/// The operation expects reference-to-source matrices on the common, untrimmed
/// canvas. A moving axis is trimmed by the rounded-up displacement and one extra
/// pixel for interpolation. An unchanged axis retains its original size.
///
/// # Errors
///
/// Returns an error for unaddressable dimensions or transforms that leave no
/// nonempty symmetric crop.
pub fn symmetric_crop(width: usize, height: usize, maps: &[Affine]) -> Result<Bounds, String> {
    let extent = Extent::new(width, height)?;
    let right = f32::from(extent.width) - 1.0;
    let bottom = f32::from(extent.height) - 1.0;
    let points = [[0.0, 0.0], [right, 0.0], [0.0, bottom], [right, bottom]];
    let [mut dx, mut dy] = [0.0_f32; 2];
    for &map in maps {
        for [x, y] in points {
            let [sx, sy] = map.point(x, y);
            dx = dx.max((sx - x).abs());
            dy = dy.max((sy - y).abs());
        }
    }
    let (x, width) = trimmed_axis(extent.width, dx)?;
    let (y, height) = trimmed_axis(extent.height, dy)?;
    Bounds::new(x, y, width, height)
}

/// Round a finite displacement outward and return the remaining span.
///
/// # Errors
///
/// Returns an error if the displacement leaves no nonempty crop.
#[expect(
    clippy::as_conversions,
    reason = "finite rounded inset is range checked before converting"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "ceil gives an integer and the u16 extent bounds the value"
)]
#[expect(
    clippy::cast_sign_loss,
    reason = "absolute displacement with interpolation margin is positive"
)]
fn trimmed_axis(full: u16, displacement: f32) -> Result<(usize, usize), String> {
    if displacement <= f32::EPSILON {
        return Ok((0, usize::from(full)));
    }
    let margin = displacement.ceil().mul_add(1.0, 1.0);
    if !margin.is_finite() || margin >= f32::from(full) {
        return Err("registration leaves no valid crop".to_owned());
    }
    let inset = margin as u16;
    let removed = inset
        .checked_mul(2)
        .ok_or("crop exceeds registration coordinate limit")?;
    let remaining = full
        .checked_sub(removed)
        .filter(|size| *size != 0)
        .ok_or("registration leaves no valid crop")?;
    Ok((usize::from(inset), usize::from(remaining)))
}
