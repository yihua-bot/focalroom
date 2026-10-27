# G2 原型评审包：原生色彩变换与瓦片金字塔（FR3-014 / FR3-015）

日期：2026-10-27。范围：`d87b3a6`→`4930f0e` 六个提交交付的原型与验证证据。
用途：请技术负责人（用户）决策是否批准接入产品管线；批准后按 §4 方案实施。
边界：设备校准、多屏一致、软打样与真实 100MP 压力测试保持 DEVICE 门槛，本文不含这些结论。

## 1. 交付物

### FR3-014 原生色彩变换（`color_engine.rs::native_transform`，macOS-gated）

| API | 输入/输出 | 位图格式 | 提交 |
|---|---|---|---|
| `transform_rgb8` | RGB8 → RGB8（就地） | 32bpp RGBX（SkipLast=5） | `d87b3a6` |
| `transform_rgb16` | RGB16 → RGB16（就地） | 64bpp RGBX16（5\|0x1000 小端） | `2018c17` |
| `transform_rgb32f` | f32 → f32（就地） | 128bpp RGBA float（256\|5\|0x2000） | `4371242` |

实现：`CGColorSpaceCreateWithICCProfile` 原始 FFI + CoreGraphics 位图上下文绘制；
ICC 字节无效返回 `COLOR_PROFILE_INVALID`，缓冲不匹配返回 `VALIDATION_ERROR`。

### FR3-015 大图管线（`image_pipeline.rs`）

| API | 职责 | 提交 |
|---|---|---|
| `tile_pyramid::{level_count, downsample_2x, tile_pixels, build_levels}` | 多级金字塔；缓存键含源 SHA + 级别 + 坐标 + 算法版本；总内存 ≤ 原图 × 4/3 | `7f8363c` |
| `tile_pyramid::build_levels_with_cancel` | 起点/每级回调取消；`CANCELLED` 结构化错误，无部分结果 | `30564b1` |
| `scaled_decode::decode_rgba_scaled` | ImageIO 缩略图按长边解码；不为缩小视图加载全分辨率像素；预乘 RGBA8 | `4930f0e` |

## 2. 验证证据

**色彩参考值（sRGB → Display P3，D65，独立色度学推导 + ColorSync 输出互证）**

| 像素 | 预期 | 依据 |
|---|---|---|
| 红 (255,0,0) | 234, 51, 35 | 初等色度计算；±3 |
| 绿 (0,255,0) | 117, 251, 76 | 同上；±3 |
| 蓝 (0,0,255) | 0, 0, **245** | 蓝原色同色度但最大亮度更低（Y 0.0722 对 0.0793，线性比≈0.9106），sRGB 编码≈245——并非直觉的 255 |
| 白 | 不变 ±1 | 双方同为 D65 |

三精度交叉验证：RGB16 ÷257 与 float ×255 必须复现 RGB8 参考值（±2）。
回归：color_engine 18 项、tile_pyramid 8 项、scaled_decode 3 项；全量 Rust 352+ 通过；
每次提交 CI 全绿（最新 run 40439363737 @ `4930f0e`）。

**平台事实（实现中确认并已写入注释）**：macOS 位图上下文不支持 24bpp 紧凑 RGB 与直连 alpha
（kCGImageAlphaLast）；字节序常量位于 CGBitmapInfo 12–14 位（0x4000 是 32Big，16Little 是 0x1000）。

## 3. 未验证项（诚实边界）

- 全部夹具为生成图像；无真实 100MP/16-bit TIFF 压力、无性能计时。
- 显示器侧 ICC（CGDisplayCopyColorSpace）尚未原型化——变换的"目标色彩空间"目前只能显式传入。
- 未接入任何产品路径；原型仅供评审。

## 4. 建议接入方案（需批准）

1. **导入预览**：`image_pipeline::prepare` 生成 1400px 预览时，对高分辨率源先走
   `decode_rgba_scaled(source, 2800)` 再编码——避免整图解码峰值内存（收益随源分辨率上升）。
2. **画布/网格缩略**：大图节点瓦片化，按 `(source_sha, level, x, y, version)` 内容寻址缓存；
   视图只取可见瓦片；构建走 `build_levels_with_cancel` 挂 UI 取消。
3. **色彩**：检视视图渲染时应用 源 ICC → 显示 ICC 变换；显示 ICC 采集需下一增量
   （CGDisplayCopyColorSpace 原型 + 变换后 provenance 记录）。
   预览/导出链路是否上变换按 §5 决策点 3。

## 5. 决策点

| # | 决策 | 选项 |
|---|---|---|
| 1 | 导入路径接入 scaled_decode？ | 批准 / 先做 100MP 真实文件性能对照再定 |
| 2 | 瓦片金字塔接入目标 | 画布网格 / 照片检查器 / 两者 |
| 3 | 色彩变换应用点 | 仅检视图（渲染时）/ 连导出渲染一并（影响产物管线，需冻结 provenance 契约） |
| 4 | 下一增量优先级 | 显示 ICC 采集原型 / 100MP 压力夹具 / 两者并行 |

## 6. 相关记录

- 会话切片清单与证据索引：`current-state-v3.json`（快照基线 d4d3e70；六提交后待下次刷新）
- CI：全部绿；最近 run 40439363737
- 依赖：core-foundation 0.10 / core-graphics 0.24 / foreign-types 0.5（仅 macOS，Cargo 已锁定）
