# v2 M1：有界 CR2 内嵌 JPEG 检视

实现与测试：2026-10-15；收尾核对：2026-10-16。范围见 [ADR 0029](ADR/0029-bounded-cr2-embedded-preview.md)。**CR2 支持子集及原生显示通过，M1 整体仍 partial；本报告不代表完整 RAW 解码、专业色彩或后续云协作已完成。**

## 本轮实现

- `cr2_preview.rs` 只读 CR2 v2 小端 IFD0 的单条 JPEG，检查目录、偏移、长度、重复字段、JPEG 头与尺寸；原文件不超过 512MiB，JPEG 不超过 4MiB / 1600 万像素。具体限制为每请求边界，不能当作应用整体内存上限。
- 使用内嵌 JPEG 本身的像素流；无 EXIF 时补充方向段，已有 EXIF 冲突时回退。八种方向有独立四象限像素预期回归；原 JPEG/ICC 段不重编码。
- IPC 使用 `original` / `derived_preview` / `embedded_preview` 判别类型。新增内嵌类型只返回有界 JPEG data URL，不提供 RAW 文件路径，不创建持久检视缓存。前端限制 MIME、字符和总长度；CSP 仅在图像来源中允许 data，不放宽 script/connect。
- 原生界面显示“RAW 内嵌 JPEG 预览 · 非 RAW 全分辨率”，按当前 JPEG 像素计算 100%。原生复验发现来源标签被图片遮挡，已补定位与层级并重建复验。
- 其他 RAW、复杂条带/目录、不支持或损坏的预览仍走校验后的 1400px 派生缓存；既有原图授权、归档/所属校验、文件大小/指纹检查保留。未新增依赖或迁移，未改 AI 预览和上传范围。

格式依据：[ExifTool 主实现](https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Exif.pm) 将 CR2 IFD0 的 0x111 / 0x117 分别映射为预览偏移与长度；实现仅覆盖这个明确子集，不泛化到全部 RAW。

## 实际测试

本轮默认 Rust 集 **125 passed**（增加 5 项 CR2 回归），另外显式 HEIC/CR2 样本 **2 passed**，合计 127 项不同功能测试。默认集中的 2 性能、2 真实格式样本为 ignored，钥匙串写删回环过滤；真实模型与生产支付请求未由本轮执行。最后对变更模块 5 项、检视套件 10 项再次通过，不重复计数。

前端 **149 passed / 19 文件**，含内嵌来源/Retina/失败回退与 IPC 非 JPEG、远端 URL、过大回执拒绝；契约、类型、Lint、fmt、严格 Clippy、Desktop/Web workspace 构建通过。M1 GUI **4 条不同旅程通过**，覆盖 500 图、异常/重连、Retina 原图及派生 RAW 回归；重复运行不算 8 条，也没有新增独立 E2E 数。

本轮执行期间工作区出现其他开发者的新提交（真实模型、签名及资源接入）。上述计数对应本轮实际运行源码；**未声称对后来所有提交完成全量复验**。那些增量需按其自己的证据核实，尤其真实资源存在不等于生产功能完成。

## 原生验收

复用 schema 17 的独立应用 `studio.betweenframes.m1reviewfixes`，原用户库及旧应用标识未启动。真实系统选择器导入本机既有 CR2 样本的临时副本，新增 1、失败 0。

样本 6,800,865 字节；读取偏移 49,416、长度 **481,778** 的 JPEG，实际 **1936×1288**。样本来源元数据未在本轮独立归档，因此只作为单个格式样本，不能代表多相机或摄影师测试集。来源指纹与预览指纹见 [camera-fixture.json](evidence/v2-m1-cr2-preview/camera-fixture.json)。

原生 WKWebView 在最终 release 中成功显示该 JPEG，完整来源标签可见；适合屏幕与 100% 切换通过。旧 PID 34997 确认退出，重启 PID **36215**。**50 张表**内容摘要一致、CR2 原文件 SHA-256 不变、检视前后 JPEG 缓存清单一致、Agent task 为 0。上一轮 500 张源文件哈希、25 张序列及人工决策也保持一致。[只读核对结果](evidence/v2-m1-cr2-preview/after-restart.json)、[复验脚本](evidence/v2-m1-cr2-preview/verify-native.py)。

原生截图在 CUA 会话内实际检查，没有本地原生 PNG 归档。浏览器回归截图另存于 `screenshots/v2-m1-cr2-preview/`，不能混称原生截图。历史截图保留。

## 剩余

更多 CR2 布局及其他 RAW 的受控检视、真实方向/ICC/高位深相机矩阵、专业显示准确性、WKWebView 总资源与低端设备仍待验证。当前 JPEG 头部/范围检查不等于完整熵数据验证，WebView 解码失败会明确回退。未实现 Lightroom 式 RAW 显影。

新增的真实模型测试只能作为接通与协议证据，不能替代 v2 要求的至少 30 项质量任务、摄影专家和用户评估。云认证、共享权威与真正多人协作仍是后续主要工程。本轮结果不改变 24 场景总账中“本地测试通过不能替代云端验收”的边界。
