-- Run with Lua 5.1 or LuaJIT from pancake/. Stub only the Lightroom SDK boundary.
local plugin = 'lightroom/Pancake.lrplugin/'
local info = assert(loadfile(plugin .. 'Info.lua'),
    'Pancake must register both Lightroom export commands')()
local commands = assert(info.LrExportMenuItems)
assert(#commands == 2, 'Expected originals and edited commands')
local service = assert(info.LrExportServiceProvider, 'Pancake must register context-menu export presets')

local photoPath = "/Photos/first's photo.dng"
local renderPath = "/tmp/Pancake-UUID/00001-first's photo.tif"
local app = "/Applications/Pancake's Copy.app"
local secondPhotoPath = '/Photos/second.dng'
local secondRenderPath = '/tmp/Pancake-UUID/00002-second.tif'
local targetPhotos = {}
for _, path in ipairs { photoPath, secondPhotoPath } do
    targetPhotos[#targetPhotos + 1] = {
        getRawMetadata = function(_, key) assert(key == 'path'); return path end,
    }
end

local events, failure, cancel, appMissing, renderFailure, launchFailure, offline, partialRender
local function record(event) events[#events + 1] = event end
local sdk = {
    LrApplication = {
        activeCatalog = function() return { getTargetPhotos = function() return targetPhotos end } end,
    },
    LrDialogs = {
        message = function(_, details) failure = details end,
    },
    LrExportSession = function(params)
        assert(#params.photosToExport == 2)
        assert(params.exportSettings.LR_format == 'TIFF')
        assert(params.exportSettings.LR_export_bitDepth == 16)
        assert(params.exportSettings.LR_export_colorSpace == 'sRGB')
        assert(params.exportSettings.LR_tiff_compressionMethod == 'compressionMethod_ZIP')
        assert(not params.exportSettings.LR_size_doConstrain)
        assert(params.exportSettings.LR_export_destinationPathPrefix == '/tmp/Pancake-UUID')
        return {
            renditions = function()
                local renders = {}
                for _, path in ipairs { renderPath, secondRenderPath } do
                    renders[#renders + 1] = {
                        waitForRender = function()
                            record('render')
                            if renderFailure then return false, 'Render failure' end
                            return true, path
                        end,
                    }
                end
                if partialRender then renders[2] = nil end
                return ipairs(renders)
            end,
        }
    end,
    LrFileUtils = {
        exists = function(path)
            if path == app .. '/Contents/MacOS/pancake-gui' and appMissing then return false end
            if path == photoPath and offline then return false end
            return 'file'
        end,
        readFile = function(path) assert(path == '/Installed/Pancake.lrplugin/Connection.txt'); return app end,
        createDirectory = function(path) assert(path == '/tmp/Pancake-UUID'); record('mkdir'); return true end,
        delete = function(path) assert(path == '/tmp/Pancake-UUID'); record('delete'); return true end,
    },
    LrPathUtils = {
        child = function(parent, leaf) return parent .. '/' .. leaf end,
        getStandardFilePath = function(kind) assert(kind == 'temp'); return '/tmp' end,
    },
    LrProgressScope = function()
        return { isCanceled = function() return cancel end, done = function() record('done') end }
    end,
    LrTasks = {
        pcall = pcall,
        startAsyncTask = function(task) task() end,
        execute = function(command)
            -- Let the real POSIX shell parse every argument, including apostrophes.
            local args = assert(io.popen("printf '%s\\n' " .. command)):read('*a')
            record(args)
            return launchFailure and 1 or 0
        end,
    },
    LrUUID = { generateUUID = function() return 'UUID' end },
}

function import(name) return assert(sdk[name], name) end
MAC_ENV = true
_PLUGIN = { path = '/Installed/Pancake.lrplugin' }
package.path = plugin .. '?.lua;' .. package.path

local function runPreset(menu)
    events, failure = {}, nil
    local env = {}
    local preset = assert(loadfile(plugin .. service.builtInPresetsDir .. '/' .. commands[menu].title .. '.lrtemplate'))
    setfenv(preset, env)()
    assert(env.s.title == commands[menu].title)
    local settings = env.s.value
    assert(settings.exportServiceProvider == info.LrToolkitIdentifier)
    local edited = menu == 2
    assert(settings[info.LrToolkitIdentifier .. '_withEdits'] == edited)
    local provider = dofile(plugin .. service.file)
    local properties = { withEdits = edited }
    provider.updateExportSettings(properties)
    assert(properties.LR_export_destinationType == 'tempFolder')
    assert(properties.LR_format == 'TIFF')
    assert(properties.LR_export_colorSpace == 'sRGB')
    assert(properties.LR_export_bitDepth == 16)
    assert(not properties.LR_size_doConstrain)
    local renditions = {}
    for i, path in ipairs { renderPath, secondRenderPath } do
        renditions[i] = {
            photo = targetPhotos[i],
            skipRender = function() record('skip') end,
            waitForRender = function()
                record('render')
                if renderFailure then return false, 'Render failure' end
                return true, path
            end,
        }
    end
    if partialRender then renditions[2] = nil end
    provider.processRenderedPhotos(nil, {
        propertyTable = properties,
        exportSession = {
            countRenditions = function() return #targetPhotos end,
            renditions = function() assert(not edited); return ipairs(renditions) end,
        },
        configureProgress = function() return sdk.LrProgressScope() end,
        renditions = function(_, args)
            assert(edited and args.stopIfCanceled)
            return ipairs(renditions)
        end,
    })
end

local function run(menu)
    events, failure = {}, nil
    dofile(plugin .. commands[menu].file)
end

run(1)
assert(not failure, failure)
assert(#events == 1 and events[1] == table.concat({
    '/usr/bin/open', '-n', '-a', app, '--args', photoPath, secondPhotoPath, '',
}, '\n'), 'Originals must open without rendering')

run(2)
assert(not failure, failure)
assert(table.concat(events, '|') == 'mkdir|render|render|done|' .. table.concat({
    '/usr/bin/open', '-n', '-W', '-a', app, '--args', '--lightroom-renders',
    renderPath, secondRenderPath, '',
}, '\n') .. '|delete', 'TIFFs must live until the new Pancake process closes')

renderFailure = true
run(2)
assert(failure:find('Render failure', 1, true))
assert(table.concat(events, '|') == 'mkdir|render|done|delete')
renderFailure = false

cancel = true
run(2)
assert(not failure, failure)
assert(table.concat(events, '|') == 'mkdir|done|delete')
cancel = false

launchFailure = true
run(2)
assert(failure:find('Could not open Pancake', 1, true))
assert(events[#events] == 'delete')
launchFailure = false

appMissing = true
run(1)
assert(failure:find('Pancake Preferences', 1, true))
assert(#events == 0)
appMissing = false

partialRender = true
run(2)
assert(failure:find('full selection', 1, true))
assert(table.concat(events, '|') == 'mkdir|render|done|delete')
partialRender = false

offline = true
run(1)
assert(failure:find('offline', 1, true))
assert(#events == 0)
offline = false

targetPhotos = { targetPhotos[1], targetPhotos[1] }
run(1)
assert(failure:find('virtual copies', 1, true))
assert(#events == 0)

targetPhotos = { targetPhotos[1] }
run(1)
assert(failure:find('at least two', 1, true))
assert(#events == 0)

targetPhotos = {
    { getRawMetadata = function() return photoPath end },
    { getRawMetadata = function() return secondPhotoPath end },
}
runPreset(1)
assert(not failure, failure)
assert(table.concat(events, '|') == 'skip|skip|' .. table.concat({
    '/usr/bin/open', '-n', '-a', app, '--args', photoPath, secondPhotoPath, '',
}, '\n'), 'Originals preset must pass originals directly')

runPreset(2)
assert(not failure, failure)
assert(table.concat(events, '|') == 'render|render|done|' .. table.concat({
    '/usr/bin/open', '-n', '-W', '-a', app, '--args', '--lightroom-renders',
    renderPath, secondRenderPath, '',
}, '\n'), 'Edited preset must keep Lightroom renditions until Pancake exits')

renderFailure = true
runPreset(2)
assert(failure:find('Render failure', 1, true))
assert(table.concat(events, '|') == 'render|done')
renderFailure = false

partialRender = true
runPreset(2)
assert(failure:find('full selection', 1, true))
assert(table.concat(events, '|') == 'render|done')
partialRender = false

cancel = true
runPreset(2)
assert(not failure, failure)
assert(table.concat(events, '|') == 'done')

print('Lightroom export tests passed')
