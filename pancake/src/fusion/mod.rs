//! Bounded focus fusion and RGB resampling on interchangeable `CubeCL` devices.

/// Shared multiscale focus and exposure methods.
pub mod advanced;

mod buffers;
mod focus;
mod resize;

pub use focus::{FocusAccumulator, FocusMethod, sharpness};
pub use resize::resize;
