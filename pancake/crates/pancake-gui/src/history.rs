//! Desktop folder history scoped to one application profile.

use std::fs::{self, File};
use std::io::{self, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

/// Keep the most recent folders at the top of a short, usable menu.
const MAX_FOLDERS: usize = 12;
/// Limit unsupported profile contents before allocating for JSON parsing.
const MAX_BYTES: usize = 65_536;
/// Store history separately from preferences so concurrent updates are independent.
const HISTORY: &str = "recent-folders.json";

/// Ordered, canonical source directories in the active profile.
#[derive(Debug)]
pub struct RecentFolders {
    /// Profile directory resolved by the application's preference store.
    profile: PathBuf,
    /// Most recently opened folder first.
    entries: Vec<PathBuf>,
}

impl RecentFolders {
    /// Load a committed folder history or start empty.
    ///
    /// # Errors
    ///
    /// Returns profile read or validation errors. Does not alter the profile.
    pub fn open(profile: &Path) -> io::Result<Self> {
        Ok(Self {
            profile: profile.to_path_buf(),
            entries: read(profile)?,
        })
    }

    /// Current folders in most-recent-first order.
    #[must_use]
    pub fn entries(&self) -> &[PathBuf] {
        &self.entries
    }

    /// Remember a directory that supplied successfully opened source images.
    ///
    /// # Errors
    ///
    /// Returns inaccessible-folder or profile-write errors. A failed write
    /// preserves the instance's prior history.
    pub fn remember(&mut self, folder: &Path) -> io::Result<()> {
        let canonical = folder.canonicalize()?;
        if !canonical.is_dir() {
            return Err(io::Error::new(
                ErrorKind::NotADirectory,
                "Recent folders must be directories.",
            ));
        }
        self.update(|entries| {
            std::iter::once(canonical.clone())
                .chain(entries.into_iter().filter(|entry| entry != &canonical))
                .take(MAX_FOLDERS)
                .collect()
        })
    }

    /// Persist an empty history for this profile.
    ///
    /// # Errors
    ///
    /// Returns profile-write errors without changing in-memory history.
    pub fn clear(&mut self) -> io::Result<()> {
        self.update(|_| Vec::new())
    }

    /// Serialize read-modify-write between windows sharing this profile.
    ///
    /// # Errors
    ///
    /// Returns profile access, serialization, or persistence errors.
    fn update(&mut self, change: impl FnOnce(Vec<PathBuf>) -> Vec<PathBuf>) -> io::Result<()> {
        fs::create_dir_all(&self.profile)?;
        let lock = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.profile.join("recent-folders.lock"))?;
        lock.lock()?;
        let next = change(read(&self.profile)?);
        let bytes = serde_json::to_vec(&next)?;
        if bytes.len() > MAX_BYTES {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Recent folders exceed 64 KiB.",
            ));
        }
        let mut pending = tempfile::NamedTempFile::new_in(&self.profile)?;
        pending.write_all(&bytes)?;
        pending.as_file().sync_all()?;
        let _published = pending.persist(self.profile.join(HISTORY))?;
        self.entries = next;
        Ok(())
    }
}

/// Read only an absolute, unique, bounded collection of paths.
///
/// # Errors
///
/// Returns read and invalid-profile errors.
fn read(profile: &Path) -> io::Result<Vec<PathBuf>> {
    let file = match File::open(profile.join(HISTORY)) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    let _count = file.take(65_537).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "Recent folders exceed 64 KiB.",
        ));
    }
    let entries: Vec<PathBuf> = serde_json::from_slice(&bytes)?;
    let unique = entries.iter().collect::<std::collections::BTreeSet<_>>();
    if entries.len() > MAX_FOLDERS
        || unique.len() != entries.len()
        || entries.iter().any(|entry| !entry.is_absolute())
    {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "Invalid recent folder list.",
        ));
    }
    Ok(entries)
}

/// Contracts for persistence across application launches.
#[cfg(test)]
mod tests {
    use super::{HISTORY, RecentFolders};
    use std::{fs, io};

    /// A successful folder visit moves that canonical directory to the front.
    ///
    /// # Errors
    ///
    /// Returns a profile I/O error or an unexpected persisted state.
    #[test]
    fn remember_and_clear_across_launches() -> io::Result<()> {
        let sandbox = tempfile::tempdir()?;
        let profile = sandbox.path().join("profile");
        let a = sandbox.path().join("输入 café");
        let b = sandbox.path().join("🌄");
        fs::create_dir(&a)?;
        fs::create_dir(&b)?;
        let mut history = RecentFolders::open(&profile)?;
        check(history.entries().is_empty(), "new profile starts clean")?;
        history.remember(&a)?;
        history.remember(&b)?;
        history.remember(&a.join("."))?;
        let mut reopened = RecentFolders::open(&profile)?;
        check(
            reopened.entries() == [a.canonicalize()?, b.canonicalize()?],
            "MRU order and canonical identity survive process restart",
        )?;
        reopened.clear()?;
        check(
            RecentFolders::open(&profile)?.entries().is_empty(),
            "explicit clear survives restart",
        )
    }

    /// A failed write keeps the previously committed in-memory history.
    ///
    /// # Errors
    ///
    /// Returns an I/O error or an unexpected change to committed history.
    #[test]
    fn write_failure_preserves_the_current_history() -> io::Result<()> {
        let sandbox = tempfile::tempdir()?;
        let profile = sandbox.path().join("profile");
        let first = sandbox.path().join("first");
        let second = sandbox.path().join("second");
        fs::create_dir(&first)?;
        fs::create_dir(&second)?;
        let mut history = RecentFolders::open(&profile)?;
        history.remember(&first)?;
        fs::remove_file(profile.join("recent-folders.json"))?;
        fs::remove_file(profile.join("recent-folders.lock"))?;
        fs::remove_dir(&profile)?;
        fs::write(&profile, b"blocked")?;
        check(
            history.remember(&second).is_err(),
            "a plain file has no configuration directory",
        )?;
        check(
            history.clear().is_err(),
            "clear also requires a successful write",
        )?;
        check(
            history.entries() == [first.canonicalize()?],
            "only committed history remains in memory",
        )
    }

    /// A profile cannot inject an unbounded or relative folder target.
    ///
    /// # Errors
    ///
    /// Returns an I/O error if the fixture fails or the history is altered.
    #[test]
    fn malformed_and_unbounded_lists_fail_without_editing_the_profile() -> io::Result<()> {
        let sandbox = tempfile::tempdir()?;
        let profile = sandbox.path();
        let data = profile.join("recent-folders.json");
        for bad in [
            b"not a list".to_vec(),
            b"[\"relative\"]".to_vec(),
            vec![b' '; 65_537],
        ] {
            fs::write(&data, &bad)?;
            check(
                RecentFolders::open(profile).is_err(),
                "bad serialized folder list must be diagnosed",
            )?;
            check(
                fs::read(&data)? == bad,
                "reading a malformed profile is nonmutating",
            )?;
        }
        Ok(())
    }

    /// Adding many folders keeps the newest bounded list.
    ///
    /// # Errors
    ///
    /// Returns a profile I/O error or an unexpected set of retained folders.
    #[test]
    fn retains_only_the_newest_folders() -> io::Result<()> {
        let sandbox = tempfile::tempdir()?;
        let mut history = RecentFolders::open(&sandbox.path().join("profile"))?;
        let mut created = Vec::new();
        for index in 0_u8..20 {
            let directory = sandbox.path().join(format!("folder-{index}"));
            fs::create_dir(&directory)?;
            history.remember(&directory)?;
            created.push(directory.canonicalize()?);
        }
        let expected: Vec<_> = created.into_iter().rev().take(12).collect();
        check(
            history.entries() == expected,
            "the MRU list has twelve entries",
        )
    }

    /// Only a directory can replace the latest folder, even when a file exists.
    ///
    /// # Errors
    /// Returns a fixture failure or an unexpected history update.
    #[test]
    fn rejects_nonfolders_without_mutating_committed_history() -> io::Result<()> {
        let sandbox = tempfile::tempdir()?;
        let profile = sandbox.path().join("profile");
        let first = sandbox.path().join("first");
        let image = first.join("image.tif");
        fs::create_dir(&first)?;
        fs::write(&image, b"existing source file")?;
        let mut history = RecentFolders::open(&profile)?;
        history.remember(&first)?;
        let expected = [first.canonicalize()?];
        let committed = fs::read(profile.join(HISTORY))?;

        for (candidate, expected_error) in [
            (image, io::ErrorKind::NotADirectory),
            (
                sandbox.path().join("removed folder"),
                io::ErrorKind::NotFound,
            ),
        ] {
            let rejected = history.remember(&candidate);
            check(
                rejected.is_err_and(|error| error.kind() == expected_error),
                "an invalid folder visit must report the applicable storage error",
            )?;
            check(
                history.entries() == expected,
                "invalid folder changed the active MRU",
            )?;
            check(
                fs::read(profile.join(HISTORY))? == committed
                    && RecentFolders::open(&profile)?.entries() == expected,
                "invalid folder changed the committed MRU",
            )?;
        }
        Ok(())
    }

    /// Report a failed contract without a panic-only path in a fallible test.
    ///
    /// # Errors
    ///
    /// Returns the test's diagnostic when the predicate is false.
    fn check(condition: bool, message: &'static str) -> io::Result<()> {
        if condition {
            Ok(())
        } else {
            Err(io::Error::other(message))
        }
    }
}
