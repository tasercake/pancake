//! Install the embedded Lightroom plug-in and connect it to the app bundle.

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

/// Files copied to Lightroom's Modules folder. Publish the manifest last.
const PAYLOAD: &[(&str, &[u8])] = &[
    (
        "Pancake.lua",
        include_bytes!("../../../lightroom/Pancake.lrplugin/Pancake.lua"),
    ),
    (
        "OpenOriginals.lua",
        include_bytes!("../../../lightroom/Pancake.lrplugin/OpenOriginals.lua"),
    ),
    (
        "OpenEdited.lua",
        include_bytes!("../../../lightroom/Pancake.lrplugin/OpenEdited.lua"),
    ),
    (
        "ExportServiceProvider.lua",
        include_bytes!("../../../lightroom/Pancake.lrplugin/ExportServiceProvider.lua"),
    ),
    (
        "presets/Pancake (Originals).lrtemplate",
        include_bytes!(
            "../../../lightroom/Pancake.lrplugin/presets/Pancake (Originals).lrtemplate"
        ),
    ),
    (
        "presets/Pancake (With Lightroom Edits).lrtemplate",
        include_bytes!(
            "../../../lightroom/Pancake.lrplugin/presets/Pancake (With Lightroom Edits).lrtemplate"
        ),
    ),
    (
        "Info.lua",
        include_bytes!("../../../lightroom/Pancake.lrplugin/Info.lua"),
    ),
];

/// One user-level installation connected to an absolute executable path.
#[derive(Debug)]
pub(super) struct Installation {
    /// Executable path read as literal data by Lua.
    executable: String,
    /// Lightroom plug-in directory.
    plugin: PathBuf,
}

impl Installation {
    /// Resolve the executable path, even when launched through a symlink.
    /// # Errors
    /// Returns filesystem errors or an error if the executable path cannot be encoded.
    pub(super) fn new(executable: &Path, modules: &Path) -> io::Result<Self> {
        let executable = executable.canonicalize()?;
        let executable = executable.into_os_string().into_string().map_err(|_| {
            io::Error::other("The Pancake Stacker executable path is not valid UTF-8.")
        })?;
        Ok(Self {
            executable,
            plugin: modules.join("Pancake.lrplugin"),
        })
    }

    /// Check the complete installed payload and its connection to this app.
    pub(super) fn is_current(&self) -> bool {
        self.files().all(|(name, data)| {
            fs::read(self.plugin.join(name)).is_ok_and(|installed| installed == data)
        })
    }

    /// Write regular copies, replacing each destination only after its write succeeds.
    /// # Errors
    /// Returns a filesystem error when the plug-in cannot be written.
    pub(super) fn write(&self) -> io::Result<()> {
        for directory in [&self.plugin, &self.plugin.join("presets")] {
            fs::create_dir_all(directory)?;
            if directory.symlink_metadata()?.is_symlink() {
                return Err(io::Error::other(format!(
                    "The Lightroom plug-in folder is a symbolic link: {}",
                    directory.display(),
                )));
            }
        }
        for (name, data) in self.files() {
            let target = self.plugin.join(name);
            let directory = target
                .parent()
                .ok_or_else(|| io::Error::other("Missing Lightroom plug-in destination folder"))?;
            let mut pending = tempfile::NamedTempFile::new_in(directory)?;
            pending.write_all(data)?;
            let _committed = pending.persist(target)?;
        }
        Ok(())
    }

    /// Emit the connection and plug-in payload in publish order.
    fn files(&self) -> impl Iterator<Item = (&str, &[u8])> {
        std::iter::once(("Connection.txt", self.executable.as_bytes()))
            .chain(PAYLOAD.iter().copied())
    }
}
