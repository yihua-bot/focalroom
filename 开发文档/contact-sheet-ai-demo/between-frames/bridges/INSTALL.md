# FocalRoom · Lightroom Classic 与 Capture One

版本 1.0.0 · macOS 11 及以上 · Apple Silicon / Intel 通用组件。

## 先了解这两个下载

- Lightroom Classic：`.lrplugin` 导出插件。仅支持 Classic，不适用于云端版 Lightroom、Lightroom Mobile。
- Capture One：独立的 macOS 桥接应用，通过导出后的照片接入；不是 Capture One SDK 插件，不装进 Capture One 的 Plugins 目录。需要手动确认每张结果对应的原照片，不读取私有 Catalog，也不自动同步 Catalog / Session / 多 Variant。
- 此版本只增加调色结果，不回写原片、评分、选片或作品序列。不上传照片，不需要 AI 或账户密钥。
- 这两款外部编辑器尚未在开发机上安装，未完成真实软件版本兼容与 30 张往返验收。请先用副本验证少量照片；不要将此包视为完整双向同步。

## 安装 Lightroom Classic 插件

1. 解压 `FocalRoom-Lightroom-Classic-1.0.0-macos.zip`。
2. 把其中完整的 `FocalRoom.lrplugin` 文件夹放到你准备长期保留的位置。不要只复制 Lua 文件，也不要再套一层同名文件夹。
3. 打开 Lightroom Classic → 文件（File）→ 增效工具管理器（Plug-in Manager）→ 添加（Add）。
4. 选择 `FocalRoom.lrplugin` 文件夹，确认插件已启用，关闭管理器。
5. 选中照片 → 文件 → 导出（Export），在“导出到 / Export To”中选择 **FocalRoom**。
6. 设置 JPEG、PNG 或 TIFF，完成导出。建议先用 sRGB JPEG；需要保留较高位深可选 16 位 TIFF。每次最多 500 张；FocalRoom 接收的单张渲染不超过 512 MB。印刷色彩仍须另行核验。
7. 按提示选择保存位置。插件会新建 `FocalRoom-…` 文件夹，保留渲染副本和对应信息。
8. 到 FocalRoom 按下方“带回项目”步骤操作。原片必须在线，插件需要读取原片确认内容身份。不要更改或删除 Lightroom 临时渲染，等待完成提示。

卸载：在 Lightroom 增效工具管理器中选中 FocalRoom，点击移除。不会删除你的照片。

## 安装 Capture One 桥接应用

1. 解压 `FocalRoom-Capture-One-Bridge-1.0.0-macos.zip`。
2. 将 **FocalRoom Capture One Bridge.app** 拖入“应用程序”，打开一次。
3. 在 Capture One 完成调色，使用导出（Export）保存 JPEG、PNG 或单页 TIFF。等待导出完成。
4. 打开桥接应用，点击“添加导出的照片”，选择刚保存的照片。也可在访达中选择这些已导出的照片 → 打开方式 → FocalRoom Capture One Bridge。
5. 在列表中选中一行，点击“选择对应原照片”。选取与 FocalRoom 项目中一致的 RAW 或其他原始文件。每次确认后自动选中下一行；全部对应完成后才能保存。
6. 点击“保存并带回 FocalRoom”，选择保存位置。桥接应用会新建结果文件夹，并在访达显示它。
7. 若 Capture One 提供导出后的“Open With / 打开方式”，可选择桥接应用接收**已渲染**照片。不要使用原图上的 **Open With** 代替导出：Capture One 官方说明该命令直接打开原片，不包含调色；也可使用 **Edit With** 并选择 JPEG/TIFF。
8. 每个外观方向请分批保存。该手动桥接不读取 Capture One 的 Variant ID，同一原片的结果在 FocalRoom 中作为同一手动调色分支的不同渲染保留。

卸载：退出后从“应用程序”移除桥接应用即可；已保存的结果与原片不受影响。

## 把调色结果带回项目

**新版客户端（显示“调色版本”入口）**：
1. 打开原项目，确保对应原照片已经导入该项目。
2. 选片工作台 → 调色版本 → 导入调色结果。
3. 选择插件生成的 **FocalRoom-… 文件夹本身**，不是它的上一级，也不是单张图片。
4. 查看新增的调色版本，点击缩略图放大。原照片、人工选片和作品序列不变。当前预览最长边 1400px，保留的渲染文件保持原始字节；这里不提供专业软打样。
5. 同一内容重复导入会跳过。照片内容与项目不对应、结果损坏或导出不完整时不会写入这一批版本。

**已发布但没有“调色版本”的旧版客户端**：
使用项目里的“导入照片 → 选择文件夹”，选择插件生成的结果文件夹。照片会作为**独立新照片**导入，不与原片合并，也不自动替换作品序列。升级到包含调色版本入口的客户端后，才支持按原片归属保存版本；不要把源码已接入当作旧安装包已更新。

## 常见问题

- **找不到插件**：Lightroom Classic 用“文件 → 增效工具管理器 → 添加”，不要复制到 Modules 后假设自动生效。Capture One 下载的是应用，不应在插件管理器中添加。
- **当前项目没有原照片**：切换到正确项目，导入当时使用的原片。不要用同名照片替代；软件按内容确认身份。
- **原照片已被其他软件改写**：用最初导入项目的版本重新导出。不要关闭完整性检查或改写对应信息。
- **无法读取或过大**：导出单页 TIFF、PNG 或 JPEG；渲染小于 512 MB。RAW 用于对应原片，不能当作已调色结果。
- **系统阻止打开**：仅使用官网版本。若仍被阻止，记录 macOS 版本和提示并联系作者；不要关闭 Gatekeeper 或执行移除隔离属性的命令。
- **颜色不同**：请核对两边显示与导出色彩配置；当前普通预览不承诺印刷级一致性。

帮助：https://focroom.cc/plugins/ · sliqiang85@gmail.com

## English quick installation

Lightroom Classic: unzip, keep the entire **FocalRoom.lrplugin** folder, then **File → Plug-in Manager → Add**. Export selected photos using **Export To: FocalRoom**. Choose JPEG, PNG or single-page TIFF, then choose a save location.

Capture One: move **FocalRoom Capture One Bridge.app** to Applications. Export rendered JPEG, PNG or TIFF files from Capture One, add them in the bridge, and explicitly select the matching original for each row. Save the new batch folder. This is an export bridge, not a Capture One SDK plug-in; automatic Catalog/Session/Variant synchronization is not included.

FocalRoom: open the original project → **Color versions → Import edited photos** and select the generated batch folder. If your installed release does not have this entry, use its existing **Import photos** action; this creates separate photos rather than linked versions. Originals and the formal selection remain unchanged. Native Lightroom/Capture One compatibility has not been verified on a real installation yet. Start with copies of a few photographs.

Full instructions in 中文 / English / 日本語 / 한국어: https://focroom.cc/plugins/
