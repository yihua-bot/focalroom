-- FocalRoom Export Filter for Lightroom Classic
-- 将选中照片导出并附带 .focalroom-manifest.json，供 FocalRoom watcher 检测回流。
--
-- 安装：将此目录放到 Lightroom Classic 的 Plug-in Extras 或 Modules 目录。
-- 使用：在 Lightroom 中选择照片 → 文件 → 导出 → FocalRoom Round-trip

local LrFileUtils = import 'LrFileUtils'
local LrPathUtils = import 'LrPathUtils'
local LrDialogs = import 'LrDialogs'
local LrExportSession = import 'LrExportSession'
local LrTasks = import 'LrTasks'
local LrDate = import 'LrDate'

local FocalRoomExport = {}

-- 生成 manifest JSON
local function generateManifest(exportContext)
    local exportSession = exportContext.exportSession
    local catalog = import 'LrApplication'.activeCatalog()
    local catalogPhotoIds = {}

    local manifest = {
        schemaVersion = 3,
        provider = "LIGHTROOM_CLASSIC",
        issuedAt = LrDate.currentTime().iso8601(),
        items = {}
    }

    -- 生成 transferBatchId（简化 UUID）
    local uuid = string.format("%08x-%08x-%08x-%08x",
        math.random(0, 0x7FFFFFFF), math.random(0, 0x7FFFFFFF),
        math.random(0, 0x7FFFFFFF), math.random(0, 0x7FFFFFFF))
    manifest.transferBatchId = uuid

    -- Catalog 指纹
    manifest.catalogFingerprint = catalog:getProperty('application', 'catalogPath') or 'unknown'

    for _, photo in ipairs(exportSession.exportedPhotos) do
        local photoId = photo:getRawMetadata('uuid') or photo.localIdentifier
        table.insert(manifest.items, {
            externalPhotoId = tostring(photoId),
            renderFilename = photo:getFormattedMetadata('fileName') or 'unknown',
            -- FocalRoom 需要的 assetId 和 sourceSha256 由 FocalRoom 端通过 XMP 传入
            -- 如果没有 XMP 标记，FocalRoom watcher 会走手动匹配流程
        })
    end

    return manifest
end

-- 导出过滤器定义
function FocalRoomExport.processRenderedPhotos(exportContext, filterContext)
    local exportSession = exportContext.exportSession
    local manifest = generateManifest(exportContext)

    -- 在导出目录写 manifest
    local exportPath = exportContext.propertyTable.LR_export_destinationPathPrefix
    if exportPath then
        local manifestPath = LrPathUtils.addChild(exportPath, '.focalroom-manifest.json')

        -- 序列化 JSON（简化实现，无依赖）
        local json = require 'JSON'
        local jsonString = json.encode(manifest)
        LrFileUtils.writeFile(manifestPath, jsonString)

        LrDialogs.message('FocalRoom Round-trip',
            '导出完成。manifest 已写入:\n' .. manifestPath ..
            '\n\nFocalRoom 将自动检测并导入调色结果。')
    end
end

-- 导出对话框部分（简化——用户选择导出目录即可）
function FocalRoomExport.sectionsForBottomOfDialog(fViewFactory, propertyTable)
    return {
        {
            title = "FocalRoom Round-trip",
            synopsis = bind 'focalroomSynopsis',
            fViewFactory:row {
                fViewFactory:static_text {
                    title = "导出照片到指定目录，FocalRoom 将自动检测回流。",
                },
            },
        },
    }
end

return FocalRoomExport
