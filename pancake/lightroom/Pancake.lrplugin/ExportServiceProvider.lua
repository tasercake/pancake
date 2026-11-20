local Pancake = require 'Pancake'

return {
    canExportVideo = false,
    allowFileFormats = { 'TIFF' },
    allowColorSpaces = { 'sRGB' },
    showSections = {},
    exportPresetFields = { { key = 'withEdits', default = false } },
    startDialog = Pancake.configurePreset,
    updateExportSettings = Pancake.configurePreset,
    processRenderedPhotos = Pancake.processPreset,
}
