//! Host-only boundaries for streaming fusion resource planning.

use super::{ImageShape, validate_buffer_limit};

/// Admit an exactly sized source allocation and reject a one-byte short page.
///
/// # Errors
/// Returns an unexpected geometry or resource validation result.
#[test]
fn native_rgb_must_fit_one_device_buffer() -> Result<(), String> {
    let grid = ImageShape::new([5, 3])?;
    validate_buffer_limit(grid, 180)?;
    if !matches!(validate_buffer_limit(grid, 179), Err(error) if error.contains("needs 180 bytes"))
    {
        return Err("oversized contiguous device buffer was accepted".to_owned());
    }
    Ok(())
}
