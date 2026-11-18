//! Language-neutral diagnostics written only after successful publication.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::config::JobConfig;
use crate::runtime::SelectedRuntime;

use super::registration::SourceMap;

/// Append a complete, factual observation of the selected and executed runtime.
///
/// # Errors
///
/// Returns invalid geometry/count, unsafe aliases, or event file I/O errors.
pub(super) fn completion(
    path: &Path,
    config: &JobConfig,
    runtime: &SelectedRuntime,
    transforms: &[SourceMap],
) -> Result<(), String> {
    if transforms.len() != config.inputs().len() {
        return Err("completion transform count differs from the source count".to_owned());
    }
    ensure_distinct_file(path, config)?;
    let geometry = config
        .inputs()
        .iter()
        .zip(transforms)
        .map(|(input, transform)| {
            input
                .to_str()
                .map(|name| (name, transform.original.matrix()))
                .ok_or("completion requires UTF-8 source paths")
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let sequential = transforms
        .iter()
        .map(|map| map.sequential.matrix())
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "event": "stack_complete",
        "completed": true,
        "actual_backend": runtime.backend().as_str(),
        "runtime": runtime.runtime_name(),
        "device": runtime.device_name(),
        "device_fingerprint": runtime.fingerprint(),
        "device_pci_address": runtime.device_pci_address(),
        "physical_gpu": runtime.physical_gpu(),
        "fallback_reason": runtime.fallback_reason(),
        "alignment_transforms": geometry,
        "sequential_transforms": sequential,
    });
    let mut line = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    line.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| {
            format!(
                "cannot open completion observer {}: {error}",
                path.display()
            )
        })?;
    file.write_all(&line).map_err(|error| {
        format!(
            "cannot write completion observer {}: {error}",
            path.display()
        )
    })
}

/// Refuse observer paths aliasing data the job must keep unchanged.
///
/// # Errors
///
/// Returns file-identity inspection errors or rejects an existing nonregular file.
fn ensure_distinct_file(path: &Path, config: &JobConfig) -> Result<(), String> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err("completion observer path must be a regular file".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    }
    for protected in config
        .inputs()
        .iter()
        .map(std::path::PathBuf::as_path)
        .chain(config.output())
    {
        if same_file::is_same_file(path, protected).map_err(|error| error.to_string())? {
            return Err("completion observer aliases an input or export".to_owned());
        }
    }
    Ok(())
}
