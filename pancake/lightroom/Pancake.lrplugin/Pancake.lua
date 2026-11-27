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
    if ok and type(path) == 'string' and path:sub(1, 1) == '/' then
        local kind = LrFileUtils.exists(path)
        if kind == 'file' then return { path = path } end
        -- Older versions stored the enclosing bundle instead of the executable.
        if kind == 'directory' and path:sub(-4) == '.app'
            and LrFileUtils.exists(LrPathUtils.child(path, 'Contents/MacOS/pancake-gui')) == 'file' then
            return { path = path, bundle = true }
        end
    end
    error('Install the plug-in again from Pancake Stacker Preferences to reconnect the app.', 0)
end

-- Each path is a single literal shell argument, including embedded apostrophes.
local function quote(path)
    return "'" .. path:gsub("'", "'\\''") .. "'"
end

local function launch(app, paths, edited)
    local target = quote(app.path)
    local args = { target }
    if app.bundle then
        args = { '/usr/bin/open', '-n' }
        if edited then args[#args + 1] = '-W' end
        args[#args + 1] = '-a'
        args[#args + 1] = target
        args[#args + 1] = '--args'
    end
    if edited then args[#args + 1] = '--lightroom-renders' end
    for _, path in ipairs(paths) do args[#args + 1] = quote(path) end
    local command = table.concat(args, ' ')
    if not app.bundle and not edited then
        -- Check execute permission before detaching and release Lightroom's standard streams.
        command = '/bin/test -x ' .. target .. ' && (' .. command .. ' </dev/null >/dev/null 2>&1 &)'
    end
    if LrTasks.execute(command) ~= 0 then
        error('Could not open Pancake Stacker. Check that it opens from its installed location.', 0)
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
            error('Selected photos share an original. Use Pancake Stacker (With Lightroom Edits) for virtual copies.', 0)
        end
        seen[path] = true
        paths[#paths + 1] = path
    end
    return paths
end

-- Both Lightroom entry points render the same full-size TIFFs.
function M.configurePreset(settings)
    settings.LR_export_destinationType = 'tempFolder'
    settings.LR_export_useSubfolder = false
    settings.LR_export_postProcessing = 'doNothing'
    settings.LR_reimportExportedPhoto = false
    settings.LR_format = 'TIFF'
    settings.LR_export_bitDepth = 16
    settings.LR_tiff_compressionMethod = 'compressionMethod_ZIP'
    settings.LR_export_colorSpace = 'sRGB'
    settings.LR_size_doConstrain = false
    settings.LR_outputSharpeningOn = false
    settings.LR_useWatermark = false
    settings.LR_renamingTokensOn = true
    settings.LR_tokens = '{{naming_sequenceNumber_5Digits}}-{{image_name}}'
    settings.LR_initialSequenceNumber = 1
    settings.LR_collisionHandling = 'rename'
end

local function collectRenders(count, progress, renditions)
    local ok, paths = LrTasks.pcall(function()
        if progress:isCanceled() then return end
        local files = {}
        for _, rendition in renditions() do
            local success, path = rendition:waitForRender()
            if progress:isCanceled() then return end
            if not success then error(path or 'Lightroom could not render a photo.', 0) end
            if not path:lower():match('%.tiff?$') then
                error('Select still photos to render as TIFFs.', 0)
            end
            files[#files + 1] = path
        end
        if progress:isCanceled() then return end
        if #files ~= count then error('Lightroom could not render the full selection.', 0) end
        return files
    end)
    progress:done()
    if not ok then error(paths, 0) end
    return paths
end

local function openEdited(app, photos)
    local folder = LrPathUtils.child(LrPathUtils.getStandardFilePath('temp'), 'Pancake-' .. LrUUID.generateUUID())
    local created, reason = LrFileUtils.createDirectory(folder)
    if not created then error(reason or 'Could not create a folder for temporary TIFFs.', 0) end
    local ok, message = LrTasks.pcall(function()
        local progress = LrProgressScope { title = 'Rendering TIFFs for Pancake Stacker' }
        local paths = collectRenders(#photos, progress, function()
            local settings = {}
            M.configurePreset(settings)
            settings.LR_export_destinationType = 'specificFolder'
            settings.LR_export_destinationPathPrefix = folder
            local session = LrExportSession { photosToExport = photos, exportSettings = settings }
            return session:renditions { progressScope = progress, stopIfCanceled = true }
        end)
        -- Pancake reads inputs on demand, so keep the TIFFs until this instance exits.
        if paths then launch(app, paths, true) end
    end)
    local removed, cleanupError = LrFileUtils.delete(folder)
    if not ok then error(message, 0) end
    if not removed then error(cleanupError or 'Could not remove the temporary TIFFs.', 0) end
end

local function appForSelection(count)
    if not MAC_ENV then error('The Pancake Stacker plug-in requires macOS.', 0) end
    if count < 2 then error('Select at least two photos to stack.', 0) end
    return installedApp()
end

local function reportFailure(action)
    local ok, message = LrTasks.pcall(action)
    if not ok then LrDialogs.message('Could not open photos in Pancake Stacker', tostring(message), 'critical') end
end

function M.openSelected(edited)
    LrTasks.startAsyncTask(function()
        reportFailure(function()
            local photos = LrApplication.activeCatalog():getTargetPhotos()
            local app = appForSelection(#photos)
            if edited then
                openEdited(app, photos)
            else
                launch(app, originalPaths(photos), false)
            end
        end)
    end)
end

function M.processPreset(_, context)
    reportFailure(function()
        local session = context.exportSession
        local count = session:countRenditions()
        local app = appForSelection(count)
        if context.propertyTable.withEdits then
            local progress = context:configureProgress { title = 'Rendering TIFFs for Pancake Stacker' }
            local paths = collectRenders(count, progress, function()
                return context:renditions { stopIfCanceled = true }
            end)
            -- Lightroom owns these files and can clean up after the callback ends.
            if paths then launch(app, paths, true) end
        else
            local photos = {}
            for _, rendition in session:renditions() do
                photos[#photos + 1] = rendition.photo
                rendition:skipRender()
            end
            if #photos ~= count then error('Lightroom could not load the full selection.', 0) end
            launch(app, originalPaths(photos), false)
        end
    end)
end

return M
