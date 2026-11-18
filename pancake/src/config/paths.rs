//! Bounded conversion of path arguments to a deterministic stack input list.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// Maximum unique source files in a job.
pub(super) const MAX_INPUT_COUNT: u32 = 4096;

/// Maximum number of explicit paths and scanned directory entries in a job.
const MAX_INPUT_ENTRIES: usize = 16_384;

/// Resolve literal paths before considering glob metacharacters.
///
/// Canonicalization collapses path aliases while preserving differently named
/// files that happen to share contents or hard-link storage.
///
/// # Errors
/// Returns inaccessible paths, malformed patterns, or exceeded input limits.
pub(super) fn resolve(arguments: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut files = BTreeSet::new();
    let mut entries = std::iter::repeat_n((), MAX_INPUT_ENTRIES);
    for argument in arguments {
        consume_entry(&mut entries)?;
        match argument.canonicalize() {
            Ok(file) => insert(&mut files, file)?,
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

/// Consume a unit of budget before doing any work on the entry.
///
/// # Errors
/// Returns an error when the scan budget is exhausted.
fn consume_entry(entries: &mut impl Iterator<Item = ()>) -> Result<(), String> {
    entries
        .next()
        .ok_or_else(|| "too many input path entries".into())
}

/// Identify metacharacters supported in a path component.
fn is_pattern(text: &str) -> bool {
    text.contains(['*', '?', '['])
}

/// Resolve a missing literal as a shell-style wildcard when applicable.
///
/// Expand each component separately. A wildcard selects entries within one
/// directory level; there is no unbounded recursive descent.
///
/// # Errors
/// Returns unresolvable or invalid patterns, invalid files, or exceeded limits.
fn expand(
    argument: &Path,
    files: &mut BTreeSet<PathBuf>,
    entries: &mut impl Iterator<Item = ()>,
) -> Result<(), String> {
    let pattern = argument
        .to_str()
        .filter(|pattern| is_pattern(pattern))
        .ok_or_else(|| format!("input {} does not exist", argument.display()))?;
    let mut candidates = vec![PathBuf::new()];
    for component in argument.components() {
        let text = component.as_os_str();
        if let Some(segment) = text.to_str().filter(|segment| is_pattern(segment)) {
            let matcher = glob::Pattern::new(segment)
                .map_err(|error| format!("invalid input pattern {pattern}: {error}"))?;
            candidates = expand_component(&candidates, &matcher, entries)?;
        } else {
            for candidate in &mut candidates {
                candidate.push(text);
            }
        }
    }
    let mut matched = false;
    for candidate in candidates {
        match candidate.canonicalize() {
            Ok(path) => insert(files, path)?,
            Err(error) if missing_component(&error) && !candidate.is_symlink() => {
                continue;
            }
            Err(error) => return Err(path_error(&candidate, &error)),
        }
        matched = true;
    }
    if !matched {
        return Err(format!("input pattern {pattern} matched no files"));
    }
    Ok(())
}

/// Scan one component without collecting an entire directory listing.
///
/// # Errors
/// Returns unexpected directory access errors or an exhausted scan budget.
fn expand_component(
    paths: &[PathBuf],
    matcher: &glob::Pattern,
    entries: &mut impl Iterator<Item = ()>,
) -> Result<Vec<PathBuf>, String> {
    let mut next_paths = Vec::new();
    for prefix in paths {
        let directory = if prefix.as_os_str().is_empty() {
            Path::new(".")
        } else {
            prefix.as_path()
        };
        let listing = match fs::read_dir(directory) {
            Ok(listing) => listing,
            Err(error) if missing_component(&error) => continue,
            Err(error) => return Err(path_error(directory, &error)),
        };
        let listing = listing.map(|entry| entry.map(|entry| (entry.file_name(), entry.path())));
        scan_entries(listing, directory, matcher, entries, &mut next_paths)?;
    }
    Ok(next_paths)
}

/// Inspect each streaming directory entry once and preserve directory read errors.
///
/// # Errors
/// Returns entry read or scan budget errors.
fn scan_entries(
    listing: impl Iterator<Item = std::io::Result<(OsString, PathBuf)>>,
    directory: &Path,
    matcher: &glob::Pattern,
    entries: &mut impl Iterator<Item = ()>,
    next_paths: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let options = glob::MatchOptions {
        require_literal_leading_dot: true,
        ..glob::MatchOptions::new()
    };
    for entry in listing {
        consume_entry(entries)?;
        let (name, path) = entry.map_err(|error| path_error(directory, &error))?;
        if matcher.matches_path_with(Path::new(&name), options) {
            next_paths.push(path);
        }
    }
    Ok(())
}

/// Missing components simply remove that wildcard prefix from consideration.
fn missing_component(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::NotFound | ErrorKind::NotADirectory)
}

/// Store one canonical regular file and enforce the retained-file limit.
///
/// # Errors
/// Returns non-file paths or input count errors.
fn insert(files: &mut BTreeSet<PathBuf>, file: PathBuf) -> Result<(), String> {
    if !file.is_file() {
        return Err(format!(
            "input {} is not an accessible file",
            file.display()
        ));
    }
    let _added = files.insert(file);
    if u32::try_from(files.len()).unwrap_or(u32::MAX) > MAX_INPUT_COUNT {
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

/// An interrupted directory stream must identify the directory and return an error.
///
/// # Errors
/// Returns a parser or missing diagnostic error.
#[cfg(test)]
#[test]
fn failed_directory_read_keeps_its_path() -> Result<(), String> {
    let directory = Path::new("failed-directory");
    let matcher = glob::Pattern::new("*.png").map_err(|error| error.to_string())?;
    let error = std::io::Error::other("directory read interrupted");
    let mut output = Vec::new();
    let outcome = scan_entries(
        std::iter::once(Err(error)),
        directory,
        &matcher,
        &mut std::iter::once(()),
        &mut output,
    );
    let expected = Err("cannot access input failed-directory: directory read interrupted".into());
    if outcome != expected || !output.is_empty() {
        return Err(format!("directory iterator error was lost: {outcome:?}"));
    }
    Ok(())
}
