//! Verify imported aliases and a real recursive filesystem layout.

use std::error::Error;
use std::fs;

use super::{Sources, folder_files};

/// Preserve recognized nested files, excluding hidden or unsupported items.
///
/// # Errors
///
/// Returns setup failures or unexpected source paths.
#[test]
fn imports_nested_source_files_once() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let folder = root.path();
    let nested = folder.join("raw");
    let hidden = folder.join(".cache");
    fs::create_dir_all(&nested)?;
    fs::create_dir_all(&hidden)?;
    fs::write(folder.join("README.txt"), [])?;
    fs::write(folder.join(".discard.png"), [])?;
    fs::write(hidden.join("cached.png"), [])?;
    let first = folder.join("tête 焦点.PNG");
    let second = nested.join("ά.CR3");
    fs::write(&first, [])?;
    fs::write(&second, [])?;

    let mut sources = Sources::default();
    sources.add(&[first.clone(), folder.join(".").join("tête 焦点.PNG")])?;
    sources.add(&folder_files(folder)?)?;
    let expected = [second.canonicalize()?, first.canonicalize()?];
    if sources.paths() != expected {
        return Err(format!("expected {expected:?}, found {:?}", sources.paths()).into());
    }
    Ok(())
}

/// A failed batch cannot remove existing sources or add a partial selection.
///
/// # Errors
///
/// Returns setup failures or unexpected mutations to the catalog.
#[test]
fn retains_sources_after_unreadable_selection() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let first = root.path().join("original.png");
    let second = root.path().join("new.tif");
    fs::write(&first, [])?;
    fs::write(&second, [])?;
    let mut sources = Sources::default();
    sources.add(std::slice::from_ref(&first))?;
    let result = sources.add(&[second, root.path().join("missing.tif")]);
    if result.is_ok() || sources.paths() != [first.canonicalize()?] {
        return Err("failed transaction changed source catalog".into());
    }
    Ok(())
}
