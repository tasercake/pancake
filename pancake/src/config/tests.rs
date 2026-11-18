//! Shared configuration checks that do not depend on filesystem or device state.

use std::str::FromStr;

use super::{AlignmentMode, AlignmentReference, BitDepth, DevicePreference, JobSettings, Method};

/// Assert both directions of the documented spelling checks.
///
/// # Errors
/// Returns a mismatch or an unexpected successful parse.
fn names<T>(values: &[(&str, T)]) -> Result<(), String>
where
    T: Copy + std::fmt::Debug + PartialEq + FromStr<Err = &'static str>,
{
    for &(name, expected) in values {
        let actual = T::from_str(name)?;
        if actual != expected {
            return Err(format!(
                "{name}: expected {expected:?}, received {actual:?}"
            ));
        }
    }
    if T::from_str("unsupported").is_ok() {
        return Err("unknown spelling must fail".into());
    }
    Ok(())
}

/// Public flags expand to specific program settings.
///
/// # Errors
/// Returns a mismatch in a documented flag spelling.
#[test]
fn configuration_names() -> Result<(), String> {
    names(&[
        ("laplacian", Method::Laplacian),
        ("hybrid", Method::Hybrid),
        ("weighted_average", Method::WeightedAverage),
        ("depth_map", Method::DepthMap),
        ("exposure_fusion", Method::ExposureFusion),
    ])?;
    names(&[
        ("off", AlignmentMode::Off),
        ("translation", AlignmentMode::Translation),
        ("euclidean", AlignmentMode::Euclidean),
        ("similarity", AlignmentMode::Similarity),
        ("affine", AlignmentMode::Affine),
        ("breathing", AlignmentMode::Breathing),
    ])?;
    names(&[
        ("first", AlignmentReference::First),
        ("middle", AlignmentReference::Middle),
        ("previous", AlignmentReference::Previous),
    ])?;
    names(&[
        ("auto", DevicePreference::Auto),
        ("cpu", DevicePreference::Cpu),
        ("gpu", DevicePreference::Gpu),
    ])?;
    names(&[("8", BitDepth::Eight), ("16", BitDepth::Sixteen)])
}

/// Check valid and invalid ranges before filesystem access.
///
/// # Errors
/// Returns an incorrect acceptance or a missing diagnostic.
#[test]
fn settings_reject_invalid_values() -> Result<(), String> {
    let defaults = JobSettings::default();
    defaults.validate()?;
    JobSettings {
        max_scale_step: 0.01,
        slab_size: 40,
        slab_overlap: 4,
        ..defaults
    }
    .validate()?;
    JobSettings {
        device: DevicePreference::Gpu,
        gpu_id: Some(0),
        ..defaults
    }
    .validate()?;
    JobSettings {
        device: DevicePreference::Cpu,
        ..defaults
    }
    .validate()?;
    JobSettings {
        kernel_size: 63,
        pyramid_levels: 32,
        scale_factor: 128,
        slab_size: 4096,
        slab_overlap: 4095,
        quality: 100,
        ..defaults
    }
    .validate()?;
    invalid_settings(defaults)
}

/// Test all invalid bounds in isolation.
///
/// # Errors
/// Returns the invalid setting that reached execution.
fn invalid_settings(defaults: JobSettings) -> Result<(), String> {
    for (settings, flag) in [
        (
            JobSettings {
                kernel_size: 0,
                ..defaults
            },
            "--kernel-size",
        ),
        (
            JobSettings {
                kernel_size: 64,
                ..defaults
            },
            "--kernel-size",
        ),
        (
            JobSettings {
                pyramid_levels: 33,
                ..defaults
            },
            "--pyramid-levels",
        ),
        (
            JobSettings {
                scale_factor: 0,
                ..defaults
            },
            "--scale-factor",
        ),
        (
            JobSettings {
                quality: 101,
                ..defaults
            },
            "--quality",
        ),
        (
            JobSettings {
                max_scale_step: 0.0,
                ..defaults
            },
            "--max-scale-step",
        ),
        (
            JobSettings {
                max_scale_step: f32::NAN,
                ..defaults
            },
            "--max-scale-step",
        ),
        (
            JobSettings {
                max_scale_step: 1.0,
                ..defaults
            },
            "--max-scale-step",
        ),
        (
            JobSettings {
                slab_size: 1,
                ..defaults
            },
            "--slab-size",
        ),
        (
            JobSettings {
                slab_size: 4097,
                ..defaults
            },
            "--slab-size",
        ),
        (
            JobSettings {
                slab_size: 2,
                slab_overlap: 2,
                ..defaults
            },
            "--slab-overlap",
        ),
        (
            JobSettings {
                device: DevicePreference::Cpu,
                gpu_id: Some(0),
                ..defaults
            },
            "--gpu-id",
        ),
    ] {
        if !settings
            .validate()
            .err()
            .is_some_and(|error| error.contains(flag))
        {
            return Err(format!("expected diagnostic for {flag} in {settings:?}"));
        }
    }
    Ok(())
}
