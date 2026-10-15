# ADR 0029：按需提取有界 CR2 内嵌 JPEG

日期：2026-10-15。延续 ADR 0028 的原图只读与 RAW 有界回退，新增一个明确受限的来源类型；不将它视为 RAW 全分辨率解码或专业色彩验收。

## 格式与读取范围

首个支持子集为 CR2 v2 小端 TIFF 头、IFD0 中的一条连续 JPEG。依据 [ExifTool 的 EXIF 读取实现](https://raw.githubusercontent.com/exiftool/exiftool/master/lib/Image/ExifTool/Exif.pm)：CR2 IFD0 的 0x111 / 0x117 分别用于预览偏移和长度。实现只读取这个主目录，不遍历 RAW IFD、MakerNotes 或任意标签指针，不搜索全文件中的 JPEG 魔数。

边界为原文件 512MiB、目录最多 128 项、JPEG 最多 4MiB / 1600 万像素、解码器头部尺寸上限每边 16000、总像素字节上限 64MiB。像素尺寸必须与 IFD0 声明相等；偏移加法、文件边界、单值类型、重复字段、Compression=6 和 JPEG SOI/EOI 均检查。JPEG 头由现有 image crate 读取，不执行 RAW 传感器解码或系统转换。实际熵数据的显示解码仍由 WebView 完成，失败明确回退，不声称头部检查等于任意 JPEG 完整解码验证。

不支持的头/布局、越界或过大预览、损坏 JPEG 头、方向冲突，都沿用校验后的最长边 1400px 派生缓存。DNG/NEF/ARW/CR3/RAF 等没有新增解码能力，`.raw` 也统一走 RAW 回退分支。原图内容指纹、所属项目/归档校验保留；成功提取后再次比对源指纹，源文件始终只读。

## 方向与字节保留

继承 CR2 IFD0 的 EXIF 方向 1–8。内嵌 JPEG 已有 EXIF 时只接受同方向，不重复插入或猜测冲突；没有 EXIF 且方向非 1 时添加 36 字节的最小 APP1 方向段。JPEG 像素流和原有 ICC 等段不重编码。

测试以四个不同颜色象限为独立预期，覆盖八种方向的尺寸与缓存角落像素，同时校验 JPEG 原有字节段保留。ICC 标记回归只证明字节保留，不冒充有效 profile 或显示器准确性；真实色彩矩阵仍待验证。

## IPC 和资源授权

`InspectionSource` 改为以 `kind` 区分的枚举：

- `original` / `derived_preview`：继续返回既有 `path`，JSON 结构兼容，精确授权对应文件。
- `embedded_preview`：只返回 `dataUrl`（JPEG Base64），不返回路径、不授权整个 RAW 文件，不创建检视缓存或改写数据库。

JPEG 的读取和内存载荷按每请求限制；按需返回使现有 1400px 缓存保持不变，也不引入需要新配额/淘汰策略的持久缓存。前端通过判别联合校验，仅接受 `data:image/jpeg;base64,` 和限定字符，整体不超过 5,600,000 字符。CSP 仅在 `img-src` 增加 `data:`，script/connect 规则保持原样。图像来源不存入 Photo 的缩略/云视觉预览字段，现有 AI 上传授权及 1400px 通道不扩大。

前端使用共享 `InspectionImage` 类型，显示“RAW 内嵌 JPEG 预览 · 非 RAW 全分辨率”；100% 以当前 JPEG 的图像像素和屏幕物理像素计算。派生预览仍保持原标识，不能与内嵌预览混同。视图关闭或翻片后沿用现有迟到响应防护。

## 范围和后续

本机单个 CR2 样本的主预览为 1936×1288 / 481,778 字节；只证明该布局可用，不证明多机型覆盖。样本沿用已有临时测试文件，来源信息不足以代表广泛摄影师素材集。

不新增依赖或迁移，复用 image、base64、唯一原图查询与共用 viewport。完整测试、原生结果与剩余项见 [本轮报告](../V2-M1-CR2-PREVIEW-TEST-REPORT.md)。RAW 全分辨率、更多容器、高像素/色彩与低端资源验收仍待后续 M1 工作。
