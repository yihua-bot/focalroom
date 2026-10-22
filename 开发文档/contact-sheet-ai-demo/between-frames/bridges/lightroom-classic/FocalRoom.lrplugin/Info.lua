-- Lightroom Classic 正式插件入口；不修改原片或 Catalog 内容。
return {
    LrSdkVersion = 9.0,
    LrSdkMinimumVersion = 9.0,
    LrToolkitIdentifier = 'cc.focroom.lightroom.export',
    LrPluginName = 'FocalRoom',
    LrPluginInfoUrl = 'https://focroom.cc/plugins/',
    VERSION = { major = 1, minor = 0, revision = 0, build = 1 },
    LrExportServiceProvider = {
        title = 'FocalRoom',
        file = 'ExportService.lua',
    },
}
