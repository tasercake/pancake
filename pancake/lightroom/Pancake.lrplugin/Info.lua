return {
    LrSdkVersion = 6.0,
    LrSdkMinimumVersion = 6.0,
    LrToolkitIdentifier = 'me.penukonda.pancake.lightroom',
    LrPluginName = 'Pancake Stacker',
    VERSION = { major = 0, minor = 1, revision = 1 },
    LrExportServiceProvider = {
        title = 'Pancake Stacker',
        file = 'ExportServiceProvider.lua',
        builtInPresetsDir = 'presets',
    },
    LrExportMenuItems = {
        { title = 'Pancake Stacker (Originals)', file = 'OpenOriginals.lua', enabledWhen = 'photosSelected' },
        { title = 'Pancake Stacker (With Lightroom Edits)', file = 'OpenEdited.lua', enabledWhen = 'photosSelected' },
    },
}
