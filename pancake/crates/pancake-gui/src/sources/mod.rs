//! Deterministic imported source collection.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// Keep the accepted source count within the shared job request limit.
const MAX_FRAMES: usize = 4_096;

/// Limit per-import directory traversal and path allocation.
const MAX_ENTRIES: usize = 16_384;

/// Canonical source paths in processing order.
#[derive(Debug, Default)]
pub struct Sources {
    /// Replaced only after the new selection is validated.
    files: Vec<PathBuf>,
}

impl Sources {
    /// Source order visible in the sidebar and chosen for a stack.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.files
    }

    /// Add a user selection transactionally and collapse filesystem aliases.
    ///
    /// # Errors
    ///
    /// Returns an inaccessible, unsupported or non-file path, or a limit error.
    pub fn add(&mut self, paths: &[PathBuf]) -> Result<(), String> {
        if paths.len() > MAX_ENTRIES {
            return Err("The selection has too many paths.".into());
        }
        let mut files: BTreeSet<_> = self.files.iter().cloned().collect();
        for path in paths {
            if !supported(path) {
                return Err(format!("Unsupported image file: {}", path.display()));
            }
            let canonical = path
                .canonicalize()
                .map_err(|error| describe(path, &error))?;
            if !canonical
                .metadata()
                .map_err(|error| describe(path, &error))?
                .is_file()
            {
                return Err(format!("Expected a source file: {}", path.display()));
            }
            let _inserted = files.insert(canonical);
            if files.len() > MAX_FRAMES {
                return Err(format!(
                    "A workspace supports at most {MAX_FRAMES} sources."
                ));
            }
        }
        self.files = files.into_iter().collect();
        Ok(())
    }

    /// Remove every source after a workspace transition.
    pub fn clear(&mut self) {
        self.files.clear();
    }

    /// Remove one displayed entry by its ordinal.
    ///
    /// # Errors
    ///
    /// Rejects an index no longer present in the current collection.
    pub fn remove(&mut self, index: usize) -> Result<(), String> {
        if index >= self.files.len() {
            return Err("The source selection changed.".into());
        }
        let _removed = self.files.remove(index);
        Ok(())
    }
}

/// List regular supported files in a user's selected folder.
///
/// Nested folders are accepted up to the traversal limit. Skip hidden entries
/// and symlink traversal. Order is lexical across the entire input tree.
///
/// # Errors
///
/// Returns an inaccessible folder or entry, or an exceeded traversal limit.
pub fn folder_files(folder: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![folder.to_path_buf()];
    let mut files = BTreeSet::new();
    let mut budget = std::iter::repeat_n((), MAX_ENTRIES);
    while let Some(path) = pending.pop() {
        for child in fs::read_dir(&path).map_err(|error| describe(&path, &error))? {
            budget
                .next()
                .ok_or("Too many entries in the selected folder.")?;
            let child = child.map_err(|error| describe(&path, &error))?;
            if child.file_name().as_encoded_bytes().starts_with(b".") {
                continue;
            }
            let kind = child
                .file_type()
                .map_err(|error| describe(&child.path(), &error))?;
            let path = child.path();
            if kind.is_dir() {
                pending.push(path);
            } else if !kind.is_symlink()
                && supported(&path)
                && child
                    .metadata()
                    .map_err(|error| describe(&path, &error))?
                    .is_file()
            {
                let _inserted = files.insert(path);
            }
        }
    }
    Ok(files.into_iter().collect())
}

/// Recognize app-supported source suffixes in a directory import.
fn supported(path: &Path) -> bool {
    path.extension()
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(|suffix| {
            matches!(
                suffix.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "tif" | "tiff" | "bmp" | "exr" | "npy" | "dng" | "cr3"
            )
        })
}

/// Add the implicated path to an I/O error without lossy path matching.
fn describe(path: &Path, error: &std::io::Error) -> String {
    format!("Could not read {}: {error}", path.display())
}
