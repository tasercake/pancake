//! Checked stage planning for focus fusion.

use std::ops::Range;

/// Suggested settings for a source size and number of images.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutomaticParameters {
    /// Width of the local sharpness window before odd-size normalization.
    pub kernel_size: usize,
    /// Number of times to reduce the source grid in a fusion pyramid.
    pub pyramid_levels: usize,
    /// Registration refinement scale.
    pub alignment_scale_factor: usize,
}

/// Derive sharpness support and pyramid depth from native input dimensions.
///
/// # Errors
/// Returns an error for an empty stack, invalid dimensions or an unaddressable image.
pub fn automatic_parameters(
    width: usize,
    height: usize,
    frame_count: usize,
) -> Result<AutomaticParameters, String> {
    if width == 0 || height == 0 || frame_count == 0 {
        return Err("automatic parameters need nonempty images and a nonempty stack".to_owned());
    }
    let pixels = width.checked_mul(height).ok_or("source area overflow")?;
    let log_short_edge = usize::try_from(width.min(height).ilog2())
        .map_err(|_| "pyramid levels exceed host addressing")?;
    let (kernel_size, alignment_scale_factor) = match pixels {
        0..=2_000_000 => (3, 6),
        2_000_001..=8_000_000 => (4, 6),
        8_000_001..=20_000_000 => (6, 10),
        _ => (8, 15),
    };
    Ok(AutomaticParameters {
        kernel_size,
        pyramid_levels: log_short_edge.saturating_sub(3).clamp(2, 12),
        alignment_scale_factor,
    })
}

/// Return ordered slab ranges, stopping after a slab reaches the last frame.
///
/// Size zero disables slabbing. The caller controls the real loading, alignment
/// and inter-slab fusion so expanding the plan does not retain input images.
///
/// # Errors
/// Returns an error for zero frames, invalid slab size or nonadvancing overlap.
pub fn slab_ranges(
    frame_count: usize,
    size: usize,
    overlap: usize,
) -> Result<Vec<Range<usize>>, String> {
    if frame_count == 0 {
        return Err("a slab plan needs at least one frame".to_owned());
    }
    if size > 0 && (size < 2 || overlap >= size) {
        return Err(format!(
            "slab size {size} and overlap {overlap} do not advance"
        ));
    }
    if size == 0 || frame_count <= size {
        return Ok(std::iter::once(0..frame_count).collect());
    }

    let stride = size
        .checked_sub(overlap)
        .ok_or("slab overlap exceeds range")?;
    let trailing = frame_count
        .checked_sub(size)
        .ok_or("slab starts outside the stack")?;
    let capacity = trailing
        .div_ceil(stride)
        .checked_add(1)
        .ok_or("slab count overflow")?;
    let mut ranges = Vec::new();
    ranges
        .try_reserve_exact(capacity)
        .map_err(|error| format!("cannot allocate slab ranges: {error}"))?;
    let mut start = 0;
    while start < frame_count {
        let end = start.saturating_add(size).min(frame_count);
        ranges.push(start..end);
        if end == frame_count {
            break;
        }
        start = end
            .checked_sub(overlap)
            .ok_or("slab overlap exceeds range")?;
    }
    Ok(ranges)
}
