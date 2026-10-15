# ADR 0027：M1 色彩处理边界（实测锁定）

日期：2026-10-15。不冒充通过——以下为可复现的行为事实，供产品决策与用户理解。

## 实测边界（tests/photo_inspection.rs 的独立 PNG/ICC 回归）

| 场景 | 管线行为 | 产品含义 |
|---|---|---|
| 16 位 PNG | 全流程可用（导入/解码/缓存/检视），缩略/预览缓存降为 8 位 RGB | 检视缓存有精度损失；原图检视（直通文件）不经管线，保留原位深 |
| ICC JPEG（宽色域） | 解码为 8 位 RGB；**缩略/预览缓存不保留 ICC** | 预览色彩可能与原图有偏差（宽色域图片饱和度/色域压缩）；**原图检视保留 ICC 字节**（原文件直通 WKWebView，由系统色彩管理） |
| EXIF 方向 | 已有回归覆盖（v1 即支持 `jpeg_exif_orientation_is_applied_to_cache`） | 无变化 |

## 边界声明

- **不是 ICC 感知管线**：image crate 解码丢弃 ICC；预览 JPEG 按 sRGB-less RGB 写出。宽色域（Display P3/Adobe RGB）图片的缩略/预览可能偏色。
- **原图检视保留原文件字节**：直通 WKWebView 后由系统解码/显示。本轮未测量渲染输出或显示器色彩，不能仅凭 ICC 字节存在保证显示准确。
- **16 位精度在缓存层丢失**：14 位以上编辑（如严重暗部提升）的精度损失在预览层不可见。原图检视不受影响。
- **校准显示器/HDR 验收**：需要外部设备，本轮未验证。WKWebView 能显示图片不等于 ICC/HDR/校准验收通过。

## 测试锁定

后续 ADR 0028 将测试拆成 `png_16bit_quantization_preserves_original_bytes` 与 `icc_metadata_is_preserved_in_original_and_absent_in_preview`。前者实际解码确认源 PNG 为 Rgb16、缓存为 Rgb8；后者使用系统 Display P3 profile 和正确长度的 JPEG APP2 段，断言缓存无 ICC、原图字节不变。ICC 分支不再静默跳过。这些断言不证明 16 位渲染或色彩准确性，EXIF 方向仍由既有独立回归覆盖。
