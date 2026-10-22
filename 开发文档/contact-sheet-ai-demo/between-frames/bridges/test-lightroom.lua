-- SDK 合约夹具：真实 Lua 5.1 执行。不能代替实际 Lightroom 加载验收。
local plugin, temp, helper, source, render, destination = unpack(arg)
package.path = plugin .. '/?.lua;' .. package.path
_PLUGIN = { path = plugin }
local messages, mode, executions = {}, 'ok', 0
local function quote(s) return "'" .. s:gsub("'", "'\\''") .. "'" end
local modules = {
    LrApplication = { activeCatalog = function() return { getPath = function() return '/catalog/test.lrcat' end } end },
    LrDialogs = { message = function(_, text) messages[#messages + 1] = text end,
        runOpenPanel = function() if mode ~= 'cancel' then return { destination } end end },
    LrFileUtils = { exists = function() return true end, createAllDirectories = function() end,
        delete = function(path) if path ~= render then os.remove(path) end end },
    LrPathUtils = { child = function(a, b) return a .. '/' .. b end, getStandardFilePath = function() return temp end },
    LrShell = { revealInShell = function() end },
    LrTasks = { pcall = pcall, execute = function(command)
        executions = executions + 1
        -- 测试时只替换已知 helper 路径；其他路径仍走生产引用转义逻辑。
        local prefix = quote(plugin .. '/bin/FocalRoomBridge')
        assert(command:sub(1, #prefix) == prefix)
        command = quote(helper) .. command:sub(#prefix + 1)
        return os.execute(command)
    end },
    LrUUID = { generateUUID = function() return 'fixture-request' end },
}
import = function(name) assert(modules[name], name); return modules[name] end
local info = dofile(plugin .. '/Info.lua')
assert(info.LrToolkitIdentifier == 'cc.focroom.lightroom.export')
assert(info.LrExportServiceProvider.file == 'ExportService.lua')
local service = dofile(plugin .. '/ExportService.lua')
local settings = { LR_collisionHandling = 'overwrite' }
service.updateExportSettings(settings)
assert(settings.LR_collisionHandling == 'rename' and settings.LR_reimportExportedPhoto == false)
local function context()
    local emitted = false
    return {
        exportSession = { countRenditions = function() return mode == 'too-many' and 501 or 1 end },
        configureProgress = function() return { isCanceled = function() return false end, done = function() end } end,
        renditions = function() return function()
            if emitted then return nil end
            emitted = true
            return 1, { photo = { localIdentifier = 42, getRawMetadata = function(_, key) assert(key == 'path'); return source end },
                waitForRender = function() return mode ~= 'failed', render end }
        end end,
    }
end
service.processRenderedPhotos(nil, context())
assert(executions == 1 and #messages == 1, 'valid export must execute the native helper')
assert(messages[1]:find('调色结果已保存', 1, true), messages[1])
for _, current in ipairs({ 'cancel', 'failed', 'too-many' }) do
    mode = current; service.processRenderedPhotos(nil, context())
    assert(executions == 1, 'invalid/cancelled exports must not execute helper: ' .. mode)
end
print('Lightroom Lua 5.1: entry, settings, real helper export, cancel, render failure, photo limit passed')
