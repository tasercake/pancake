//! Independent contracts shared by CPU and physical-GPU qualification targets.

use cubecl::Device;
use pancake::{blur_horizontal_u16_bounded, row_variance_u16};

/// Verify centered variance without discarding the low bits of bright inputs.
///
/// # Errors
/// Returns a runtime failure or independently known numerical mismatch.
pub fn check_variance(device: &Device) -> Result<(), String> {
    if row_variance_u16(device, 0, 1, &[], 1_000)
        != Err("image dimensions must be nonzero".to_owned())
    {
        return Err("variance shape error did not propagate".to_owned());
    }
    let input = [
        0, 2, 4, 6, 8, 65_535, 65_535, 65_535, 65_535, 65_535, 65_531, 65_532, 65_533, 65_534,
        65_535,
    ];
    if blur_horizontal_u16_bounded(device, 5, 1, &[0, 4, 8, 12, 16], 100)? != [1, 4, 8, 12, 15] {
        return Err("exact blur budget must admit the job".to_owned());
    }
    if row_variance_u16(device, 5, 1, &[0, 2, 4, 6, 8], 110)? != [8.0] {
        return Err("exact variance budget must admit the job".to_owned());
    }
    let actual = row_variance_u16(device, 5, 3, &input, 1_000)?;
    if actual != [8.0, 0.0, 2.0] {
        return Err(format!("independent variance mismatch: {actual:?}"));
    }
    if row_variance_u16(device, 4, 1, &[65_532, 65_533, 65_534, 65_535], 1_000)? != [1.25] {
        return Err("bright low-bit variance changed".to_owned());
    }
    if row_variance_u16(device, 1, 3, &[0, 32_768, 65_535], 1_000)? != [0.0; 3] {
        return Err("width-one variance must be zero".to_owned());
    }
    if row_variance_u16(device, 257, 3, &vec![65_535; 771], 20_000)? != [0.0; 3] {
        return Err("flat odd-width variance must be exactly zero".to_owned());
    }
    Ok(())
}

/// Exercise repeated completed jobs and record live versus reserved allocations.
///
/// # Errors
/// Returns a runtime, numerical, ownership or artifact-write failure.
pub fn check_budget_and_reuse(device: &Device, name: &str) -> Result<(), String> {
    let pixels = vec![65_535; 263_425];
    let expected = vec![0.0_f32; 257];
    let client = device.client();
    if row_variance_u16(device, 1_025, 257, &pixels, 8_388_608)? != expected {
        return Err("large bright plane lost variance precision".to_owned());
    }
    if blur_horizontal_u16_bounded(device, 1_025, 257, &pixels, 8_388_608)? != pixels {
        return Err("large constant blur changed samples".to_owned());
    }
    let warm = client
        .memory_report(cubecl::MemoryScope::CurrentStream)
        .usage();
    for _ in 0_u8..8 {
        if row_variance_u16(device, 1_025, 257, &pixels, 8_388_608)? != expected {
            return Err("reused variance job changed samples".to_owned());
        }
        if blur_horizontal_u16_bounded(device, 1_025, 257, &pixels, 8_388_608)? != pixels {
            return Err("reused blur job changed samples".to_owned());
        }
    }
    let settled = client
        .memory_report(cubecl::MemoryScope::CurrentStream)
        .usage();
    if settled.bytes_in_use != warm.bytes_in_use || settled.number_allocs != warm.number_allocs {
        return Err(format!(
            "live allocations grew after reuse: {warm:?} -> {settled:?}"
        ));
    }
    std::fs::create_dir_all("target").map_err(|error| error.to_string())?;
    std::fs::write(
        format!("target/p0-numerical-{name}.txt"),
        format!(
            "device={:#?}\nwarm={warm:#?}\nsettled={settled:#?}\nlogical_request_budget=8388608\n",
            client.properties().identity
        ),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}
