// Shared independent numeric oracles for CPU and physical-GPU execution.

use cubecl::Device;
use pancake::blur_horizontal_u16;

/// Check independent hand-calculated values, clamped borders and full u16 range.
///
/// # Errors
///
/// Returns runtime failures or a numerical contract mismatch.
fn check_odd_plane(device: &Device) -> Result<(), String> {
    let input = [0, 4, 8, 12, 16, 65_535, 65_535, 0, 1, 65_534, 1, 2, 3, 4, 5];
    let expected = [
        1, 4, 8, 12, 15, 65_535, 49_151, 16_384, 16_384, 49_150, 1, 2, 3, 4, 4,
    ];
    let actual = blur_horizontal_u16(device, 5, 3, &input)?;
    if actual != expected {
        return Err(format!(
            "clamped u16 blur mismatch: {actual:?} != {expected:?}"
        ));
    }
    Ok(())
}

/// Check workgroup tails and one-pixel-wide borders without a scalar twin kernel.
///
/// # Errors
///
/// Returns runtime failures or a boundary contract mismatch.
fn check_boundaries(device: &Device) -> Result<(), String> {
    if blur_horizontal_u16(device, 1, 3, &[0, 32_768, 65_535])? != [0, 32_768, 65_535] {
        return Err("one-pixel-wide images must be unchanged".to_owned());
    }
    let constant = vec![65_535; 771];
    if blur_horizontal_u16(device, 257, 3, &constant)? != constant {
        return Err("an odd workgroup tail must preserve a constant image".to_owned());
    }
    Ok(())
}
