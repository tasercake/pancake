//! Bounded conversion of path arguments to a deterministic stack input list.

use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Maximum unique source files in a job.
pub(super) const MAX_INPUT_COUNT: u32 = 4096;

/// Maximum path entries examined across explicit arguments and glob results.
const MAX_INPUT_ENTRIES: usize = 16_384;

/// Resolve literals before considering glob metacharacters.
///
/// Canonicalization collapses path aliases while preserving differently named
/// files that happen to share contents or hard-link storage.
pub(super) fn resolve(arguments: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = BTreeSet::new();
    let mut entries = limited_entries();
    for argument in arguments {
        let () = entries.next().ok_or("too many input path entries")?;
        match fs::metadata(argument) {
            Ok(metadata) if metadata.is_file() => insert(&mut files, argument)?,
            Ok(_) => return Err(format!("input {} is not a file", argument.display())),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                expand(argument, &mut files, &mut entries)?;
            }
            Err(error) => return Err(path_error(argument, &error)),
        }
    }
    if files.len() < 2 {
        return Err(format!(
            "at least two distinct input files are required (found {})",
            files.len()
        ));
    }
    Ok(files.into_iter().collect())
}

/// Stop processing repeated arguments or glob results after the entry limit.
fn limited_entries() -> impl Iterator<Item = ()> {
    std::iter::repeat_n((), MAX_INPUT_ENTRIES)
}

/// Resolve a missing literal as a shell-like wildcard when applicable.
fn expand(
    argument: &Path,
    files: &mut BTreeSet<PathBuf>,
    entries: &mut impl Iterator<Item = ()>,
) -> Result<(), String> {
    let pattern = argument
        .to_str()
        .filter(|pattern| pattern.contains(['*', '?', '[']))
        .ok_or_else(|| format!("input {} does not exist", argument.display()))?;
    let mut matches = glob::glob_with(
        pattern,
        glob::MatchOptions {
            require_literal_leading_dot: true,
            require_literal_separator: true,
            ..glob::MatchOptions::new()
        },
    )
    .map_err(|error| format!("invalid input pattern {pattern}: {error}"))?
    .peekable();
    if matches.peek().is_none() {
        return Err(format!("input pattern {pattern} matched no files"));
    }
    for matched in matches {
        let () = entries.next().ok_or("too many input path entries")?;
        let path =
            matched.map_err(|error| format!("cannot read input pattern {pattern}: {error}"))?;
        let metadata = fs::metadata(&path).map_err(|error| path_error(&path, &error))?;
        if !metadata.is_file() {
            return Err(format!("input {} is not a file", path.display()));
        }
        insert(files, &path)?;
    }
    Ok(())
}

/// Store one canonical path and enforce the retained-file limit.
fn insert(files: &mut BTreeSet<PathBuf>, path: &Path) -> Result<(), String> {
    let file = path
        .canonicalize()
        .map_err(|error| path_error(path, &error))?;
    let _added = files.insert(file);
    if files.len() > usize::try_from(MAX_INPUT_COUNT).map_err(|error| error.to_string())? {
        return Err(format!(
            "stack exceeds {MAX_INPUT_COUNT} distinct input files"
        ));
    }
    Ok(())
}

/// Report which path was inaccessible.
fn path_error(path: &Path, error: &std::io::Error) -> String {
    format!("cannot access input {}: {error}", path.display())
}
