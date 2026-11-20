local LrApplication = import 'LrApplication'
local LrDialogs = import 'LrDialogs'
local LrExportSession = import 'LrExportSession'
local LrFileUtils = import 'LrFileUtils'
local LrPathUtils = import 'LrPathUtils'
local LrProgressScope = import 'LrProgressScope'
local LrTasks = import 'LrTasks'
local LrUUID = import 'LrUUID'

local M = {}

local function installedApp()
    local ok, path = LrTasks.pcall(LrFileUtils.readFile, LrPathUtils.child(_PLUGIN.path, 'Connection.txt'))
    if not ok or type(path) ~= 'string' or path:sub(1, 1) ~= '/' or path:sub(-4) ~= '.app'
        or LrFileUtils.exists(LrPathUtils.child(path, 'Contents/MacOS/pancake-gui')) ~= 'file' then
        error('Install the plug-in again from Pancake Preferences to reconnect the app.', 0)
    end
    return path
end

-- Each path is a single literal shell argument, including embedded apostrophes.
local function quote(path)
    return "'" .. path:gsub("'", "'\\''") .. "'"
end

local function launch(app, paths, edited)
    local args = { '/usr/bin/open', '-n' }
    if edited then args[#args + 1] = '-W' end
    args[#args + 1] = '-a'
    args[#args + 1] = quote(app)
    args[#args + 1] = '--args'
    if edited then args[#args + 1] = '--lightroom-renders' end
    for _, path in ipairs(paths) do args[#args + 1] = quote(path) end
    if LrTasks.execute(table.concat(args, ' ')) ~= 0 then
        error('Could not open Pancake. Check that the app opens from Finder.', 0)
    end
end

local function originalPaths(photos)
    local paths, seen = {}, {}
    for _, photo in ipairs(photos) do
        local path = photo:getRawMetadata('path')
        if not path or LrFileUtils.exists(path) ~= 'file' then
            error('An original is offline. Reconnect it in Lightroom and try again.', 0)
        end
        if seen[path] then
            error('Selected photos share an original. Use Pancake (With Lightroom Edits) for virtual copies.', 0)
        end
        seen[path] = true
        paths[#paths + 1] = path
    end
    return paths
end

local function render(photos, folder, progress)
    local session = LrExportSession {
        photosToExport = photos,
        exportSettings = {
            LR_export_destinationType = 'specificFolder',
            LR_export_destinationPathPrefix = folder,
            LR_export_useSubfolder = false,
            LR_export_postProcessing = 'doNothing',
            LR_reimportExportedPhoto = false,
            LR_format = 'TIFF',
            LR_export_bitDepth = 16,
            LR_tiff_compressionMethod = 'compressionMethod_ZIP',
            LR_export_colorSpace = 'sRGB',
            LR_size_doConstrain = false,
            LR_outputSharpeningOn = false,
            LR_useWatermark = false,
            LR_renamingTokensOn = true,
            LR_tokens = '{{naming_sequenceNumber_5Digits}}-{{image_name}}',
            LR_initialSequenceNumber = 1,
            LR_collisionHandling = 'rename',
        },
    }
    local paths = {}
    for _, rendition in session:renditions { progressScope = progress, stopIfCanceled = true } do
        local success, path = rendition:waitForRender()
        if progress:isCanceled() then return end
        if not success then error(path or 'Lightroom could not render a photo.', 0) end
        if not path:lower():match('%.tiff?$') then
            error('Select still photos to render as TIFFs.', 0)
        end
        paths[#paths + 1] = path
    end
    if progress:isCanceled() then return end
    if #paths ~= #photos then error('Lightroom could not render the full selection.', 0) end
    return paths
end

local function openEdited(app, photos)
    local folder = LrPathUtils.child(LrPathUtils.getStandardFilePath('temp'), 'Pancake-' .. LrUUID.generateUUID())
    local created, reason = LrFileUtils.createDirectory(folder)
    if not created then error(reason or 'Could not create a folder for temporary TIFFs.', 0) end
    local ok, message = LrTasks.pcall(function()
        local progress = LrProgressScope { title = 'Rendering TIFFs for Pancake' }
        local rendered, paths = LrTasks.pcall(function()
            if not progress:isCanceled() then return render(photos, folder, progress) end
        end)
        progress:done()
        if not rendered then error(paths, 0) end
        -- Pancake reads inputs on demand, so keep the TIFFs until this instance exits.
        if paths then launch(app, paths, true) end
    end)
    local removed, cleanupError = LrFileUtils.delete(folder)
    if not ok then error(message, 0) end
    if not removed then error(cleanupError or 'Could not remove the temporary TIFFs.', 0) end
end

function M.openSelected(edited)
    LrTasks.startAsyncTask(function()
        local ok, message = LrTasks.pcall(function()
            if not MAC_ENV then error('The Pancake plug-in requires macOS.', 0) end
            local photos = LrApplication.activeCatalog():getTargetPhotos()
            if #photos < 2 then error('Select at least two photos to stack.', 0) end
            local app = installedApp()
            if edited then
                openEdited(app, photos)
            else
                launch(app, originalPaths(photos), false)
            end
        end)
        if not ok then LrDialogs.message('Pancake export failed', tostring(message), 'critical') end
    end)
end

return M
