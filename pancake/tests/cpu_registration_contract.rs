//! Run independent registration oracles through the production CPU backend.

include!("common/registration_cases.rs");

/// Recover known translation and crop coordinates, and reject featureless motion.
///
/// # Errors
///
/// Returns a device or numerical error.
#[test]
fn cpu_registration_geometry() -> Result<(), String> {
    let device = cubecl::Device::cpu().map_err(|error| error.to_string())?;
    crop_and_warp_preserve_rgb_coordinates(&device)?;
    registration_recovers_subpixel_translation(&device)?;
    featureless_pair_has_zero_confidence(&device)?;
    registration_recovers_wide_shift(&device)
}
