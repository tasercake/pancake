//! Shared source geometry and resampling.

mod estimate;
mod geometry;
mod kernels;
mod spectral;

use std::marker::PhantomData;
use std::rc::Rc;

use cubecl::prelude::{Client, CubeElement as _};
use cubecl::std::tensor::TensorHandle;

use estimate::THUMBNAIL_LIMIT;
use geometry::Extent;

pub use estimate::{Estimate, Model, Thumbnail, estimate_pair, thumbnail, thumbnail_with_scale};
pub use geometry::{Affine, Bounds, Frame, symmetric_crop};
pub use kernels::warp;

/// Immutable packed RGB retained by the owner of the current registration frame.
#[derive(Debug)]
pub(crate) struct ResidentRgb {
    /// The original raster extent checked before upload.
    extent: Extent,
    /// The exact allocation read by the luma kernel.
    tensor: TensorHandle,
    /// Preserve the originating client's implicit stream on the job owner thread.
    _owner_thread: PhantomData<Rc<()>>,
}

impl ResidentRgb {
    /// Keep the current frame's immutable allocation on its owner thread.
    const fn new(extent: Extent, tensor: TensorHandle) -> Self {
        Self {
            extent,
            tensor,
            _owner_thread: PhantomData,
        }
    }

    /// Consume the current frame on the same service and checked packed canvas.
    /// # Errors
    /// Returns an error before submission for incompatible geometry or device ownership.
    pub(crate) fn into_tensor(
        self,
        client: &Client,
        width: usize,
        height: usize,
    ) -> Result<TensorHandle, String> {
        let extent = Extent::new(width, height)?;
        let samples = extent.samples()?;
        if self.extent != extent
            || self.tensor.shape().as_slice() != [samples]
            || self.tensor.strides().iter().copied().ne([1])
            || self.tensor.dtype != f32::cube_type()
        {
            return Err("resident RGB does not match the packed fusion canvas".to_owned());
        }
        if self.tensor.handle.service != client.service_id() {
            return Err("resident RGB belongs to a different compute service".to_owned());
        }
        Ok(self.tensor)
    }
}

/// Prepare a scaled job proxy with an optional owner-thread original upload.
/// # Errors
/// Returns a registration shape, budget, or readback error.
pub(crate) fn frame_thumbnail(
    device: &cubecl::Device,
    frame: Frame<'_>,
    [width, height]: [usize; 2],
    scale_factor: u32,
    budget: usize,
    retain_input: bool,
) -> Result<(Thumbnail, Option<ResidentRgb>), String> {
    let canvas = Extent::new(width, height)?;
    let longest = u32::from(canvas.width.max(canvas.height));
    let requested = longest
        .checked_div(scale_factor)
        .ok_or("alignment scale factor must be positive")?;
    let side = requested.clamp(u32::from(THUMBNAIL_LIMIT), 768);
    estimate::prepare_thumbnail(device, frame, canvas, side, budget, retain_input)
}
