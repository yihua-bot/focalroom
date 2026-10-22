local LrApplication = import 'LrApplication'
local LrDialogs = import 'LrDialogs'
local LrFileUtils = import 'LrFileUtils'
local LrPathUtils = import 'LrPathUtils'
local LrShell = import 'LrShell'
local LrTasks = import 'LrTasks'
local LrUUID = import 'LrUUID'
local Json = require 'Json'

local Service = {
    supportedPlatforms = { 'Macintosh' },
    allowFileFormats = { 'JPEG', 'TIFF', 'PNG' },
    canExportVideo = false,
    hideSections = { 'exportLocation', 'postProcessing' },
    exportPresetFields = {},
}
local function shellQuote(value)
    return "'" .. value:gsub("'", "'\\''") .. "'"
end
local function write(path, value)
    local file, message = io.open(path, 'wb')
    if not file then error(message) end
    local ok, err = file:write(value)
    file:close()
    if not ok then error(err) end
end
function Service.updateExportSettings(settings)
    -- 只写入独立临时目录，绝不采用“覆盖已有照片”的导出设置。
    local staging = LrPathUtils.child(LrPathUtils.getStandardFilePath('temp'), 'FocalRoom-' .. LrUUID.generateUUID())
    LrFileUtils.createAllDirectories(staging)
    settings.LR_export_destinationType = 'specificFolder'
    settings.LR_export_destinationPathPrefix = staging
    settings.LR_export_useSubfolder = false
    settings.LR_collisionHandling = 'rename'
    settings.LR_reimportExportedPhoto = false
end
function Service.sectionsForTopOfDialog(f)
    return { { title = 'FocalRoom', f:static_text {
        title = '导出完成后选择保存位置，再到 FocalRoom 项目中导入调色结果。\n原照片和 Lightroom 中的调整保持不变。每次最多 500 张。',
        fill_horizontal = 1,
    } } }
end
function Service.processRenderedPhotos(_, context)
    if context.exportSession:countRenditions() > 500 then
        LrDialogs.message('FocalRoom', '每次最多导出 500 张，请缩小选区。', 'warning')
        return
    end
    local destinations = LrDialogs.runOpenPanel {
        title = '选择 FocalRoom 调色结果保存位置', canChooseFiles = false,
        canChooseDirectories = true, canCreateDirectories = true, allowsMultipleSelection = false,
    }
    if not destinations then return end
    local helper = LrPathUtils.child(_PLUGIN.path, 'bin/FocalRoomBridge')
    if not LrFileUtils.exists(helper) then
        LrDialogs.message('FocalRoom', '插件文件不完整，请从官网重新下载完整安装包。', 'critical')
        return
    end
    local entries, renderedPaths, failures = {}, {}, {}
    local progress = context:configureProgress { title = '准备 FocalRoom 照片' }
    for _, rendition in context:renditions { stopIfCanceled = true } do
        local success, path = rendition:waitForRender()
        if success then
            local photo = rendition.photo
            entries[#entries + 1] = {
                sourcePath = photo:getRawMetadata('path'), renderPath = path,
                externalPhotoId = tostring(photo.localIdentifier), externalVariantId = tostring(photo.localIdentifier),
            }
            renderedPaths[#renderedPaths + 1] = path
        else failures[#failures + 1] = tostring(path) end
    end
    if progress:isCanceled() or #failures > 0 or #entries == 0 then
        LrDialogs.message('FocalRoom', '导出未完成，本次没有生成导入包。请检查照片是否可读取后重试。', 'warning')
        return
    end
    local temp = LrPathUtils.getStandardFilePath('temp')
    local id = LrUUID.generateUUID()
    local input = LrPathUtils.child(temp, 'focalroom-request-' .. id .. '.json')
    local result = LrPathUtils.child(temp, 'focalroom-result-' .. id .. '.json')
    local ok, errorMessage = LrTasks.pcall(function()
        write(input, Json.encode {
            provider = 'LIGHTROOM_CLASSIC', catalogId = LrApplication.activeCatalog():getPath(),
            destination = destinations[1], items = entries,
        })
        local exitCode = LrTasks.execute(shellQuote(helper) .. ' --request ' .. shellQuote(input) .. ' ' .. shellQuote(result))
        if exitCode ~= 0 then error('照片准备失败。请确认原片在线、导出格式为 JPEG / TIFF / PNG，且保存位置有足够空间。') end
    end)
    LrFileUtils.delete(input)
    LrFileUtils.delete(result)
    -- 只移除本次 Lightroom 生成的临时渲染，不删除任何原片。
    if ok then
        for _, path in ipairs(renderedPaths) do LrFileUtils.delete(path) end
        LrShell.revealInShell(destinations[1])
        LrDialogs.message('FocalRoom', '调色结果已保存到新的 FocalRoom 文件夹。\n\n在项目中选择「调色版本」→「导入调色结果」。旧版客户端可用「导入照片」作为独立照片导入。', 'info')
    else LrDialogs.message('FocalRoom', tostring(errorMessage), 'critical') end
    progress:done()
end
return Service
