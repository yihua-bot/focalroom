# v2 M1：原图检视、GUI 内存与原生恢复验收

当前结果见 [M1 评审修复报告](V2-M1-REVIEW-FIXES-TEST-REPORT.md) 与 ADR 0028。本文件保留历史切片；第二轮的 RAW 嵌入预览、当前快照重放和每 500 命令基准声明已修订，不能当作当前验收保证。

日期：2026-10-15。沿用 33e5ff9 之后的未提交修复和 ADR 0024。**本轮补齐 JPEG/PNG 原图检视、精简 GUI 历史载荷，以及原生选择器/保存/退出重启验收；M1 整体仍 partial，不进入新的 M2。** 决策见 [ADR 0025](ADR/0025-m1-original-inspection-and-gui-projection.md)。

## 实现结果

- 灯箱和同步对比增加“100% 原图”，按需校验所属项目、文件指纹和检视上限后读取原文件。Retina 下按物理屏幕像素显示，适合屏幕继续使用缓存。预览模式仍明确标为“100% 预览”。
- 原图当前支持 JPEG / PNG，不超过 64 MiB / 3200 万像素；失联、内容改变、格式不支持、超限或解码失败均有明确提示并回退。原图只读、不复制、不重编码，未扩大 AI 图像入口。
- GUI 请求精简历史投影，省略每个事件携带的完整前后状态与上下文。事件列表/数量、当前状态、版本和撤销游标保留；数据库审计、持久化幂等回执及导出保持完整。没有修改旧迁移或新增依赖。
- 共用 PhotoViewport 通过可选加载能力访问原图，复用缩放/平移与错误处理，避免把 Tauri 调用散入 UI；原图回调仅持有项目/照片 ID。遵循 KISS/DRY，未引入额外图片框架或新的状态存储。

## 功能验收

| 场景 | 实际断言 | 结果 |
|---|---|---|
| 500 张完整旅程 | 250 PNG + 250 JPEG，导入、虚拟网格、40 次决策、批量 25 张、评分备注、序列、三轮 Undo/Redo、版本恢复、导出、新 Rust 进程恢复 | pass |
| 原图与缓存区分 | 独立 2400×1600 JPEG/PNG；适合屏幕 naturalWidth 1400，100% 原图 naturalWidth 2400；初次点击前没有原图资源请求 | pass |
| Retina 几何 | devicePixelRatio=2，2400 像素原图显示 CSS 宽度 1200；两图同步切换到原图 | pass |
| 失败与恢复 | 临时原图移走后提示失联，仍显示 1400px 缓存；重连后实际文件 hash 与入库值一致；异步迟到响应被忽略 | pass |
| 原图保护 | 完整旅程 500 个源 SHA-256 前后一致；两张高分辨率图经检视、比较、重连后 hash 不变 | pass |
| 权限和格式 | 跨项目/未知照片/归档项目拒绝；已改变内容拒绝；TIFF 显示不支持原图检视；像素超限拒绝 | pass |
| 历史与幂等 | 精简副本不改变 Store 完整快照；同 envelope 重放原始完整回执；实际 GUI Undo/Redo 和导出继续通过 | pass |
| 异常导入 | 取消时已提交 10/500 张，进程恢复投影相等；重复跳过 1 张，损坏/编码错误/600MiB 文件各自失败；重试不新增照片 | pass |

数据为仓库素材派生的自动化测试图和生成的梯度图；不替代独立摄影师真实高像素素材。细节见 [原图检视结果](evidence/v2-m1-inspection/original-inspection.json)、[异常结果](evidence/v2-m1-inspection/exception-results.json)。

## 性能实测

Apple M4 / 10 核 / 24GiB、Chromium 156、Vite dev 前端 + release Rust/SQLite。沿用上轮 500 张 1200×800 文件与同一编辑旅程，最终复测期间没有并行编译；trace 关闭，未强制 GC。不是原生 WKWebView 或低端机基准，不是峰值内存。

| 指标 | ADR 0024 基线 | 本轮最终采样 |
|---|---:|---:|
| 40 次编辑后 GUI 项目投影 | 完整历史 | UTF-8 5,718,837 → 510,106 字节，减少 **91.1%** |
| 40 次编辑后 JS Heap | 220.8 MiB | **74.4 MiB** |
| 最终 Renderer RSS | 757.1 MiB | **464.8 MiB** |
| JS Heap 三阶段采样 | 32.4 → 220.8 → 414.2 MiB | 31.3 → 74.4 → 28.3 MiB |
| 按键到下一帧 p95 | 133.3ms | **66.7ms** |
| IPC p95 | 127.6ms | **60.4ms** |
| 500 张导入 | 9.41s | 9.90s |
| Rust RSS（退出前 / 新进程） | 324112 / 113520 KiB | 203440 / 147904 KiB |

投影字节在本轮同一时刻分别请求完整/精简接口测量。JS Heap 三阶段为滚动后、40 次编辑后、重启并导出后；GC 与进程缓存会造成波动，不能据此宣称泄漏已排除或全场景内存达标。GPU 仍为 SwiftShader 软件设备，利用率为 null；没有伪造硬件 GPU 指标。[完整机器数据](evidence/v2-m1-inspection/gui-metrics.json)。

## 原生验收

release 构建的 `Between Frames M1 Inspection.app` 使用新标识 `studio.betweenframes.m1inspection` 和新数据库（schema 17），未打开或迁移旧评审应用/原用户数据库。通过 CUA 实际操作系统对话框和 WKWebView：

1. 单文件选择导入 JPEG：新增 1、失败 0；文件夹选择导入：新增 PNG 1、重复 JPEG 1、失败 0。
2. 两张均为 2400×1600；JPEG/PNG 100% 原图可见，同步对比两边均显示原图，人工保留 JPEG 并评 4 星，记录一次“不确定”比较。
3. 系统 Save 对话框写入新的 JSON 文件；回执 SHA-256 与实际文件相等，2 张照片 / 1 张序列，默认无本机路径。
4. 使用应用 Quit 完整退出，PID **98093** 消失；再次启动得到 PID **98492**。项目、4 星、选片及 3 次可撤销编辑可见；**49 张数据表逐表内容摘要完全相等**，源图 hash 不变、Agent task 为 0。新进程也能重新授权并显示原图。

原生截图已在会话中实际检查，未伪称已归档本地图片。原生结果与逐表摘要见 [native-results.json](evidence/v2-m1-inspection/native-results.json)。本轮原生使用 2 张生成图，不宣称完成原生 500 张性能验收。

## 回归与复现

- Rust **113 项通过**，性能基线 1 项 ignored，钥匙串写删回环 1 项跳过。
- 前端 **146 项 / 19 文件通过**；E2E **26 条独立旅程通过**：16 预览、1 Web、6 既有真实后端、3 M1 GUI。
- 契约、TypeScript、ESLint、fmt、严格 Clippy（含 integration-harness）、Desktop/Web workspace 构建、独立原生 release 应用构建通过。
- 首轮前端测试清理误移除了共享 ResizeObserver mock，修正测试清理后全量通过。Web 测试原先与 1424 用户预览冲突，本地允许复用现有服务，CI 仍要求独占；重跑通过。测试覆盖的历史截图已还原，本轮证据放入独立目录。
- 汇总与原始输出：[results.json](evidence/v2-m1-inspection/results.json)。[本轮截图目录](screenshots/v2-m1-inspection/)。

```bash
# 在 between-frames 执行；1420/1422 的两组测试必须串行。
BF_M1_ACCEPTANCE_SLICE="v2-m1-inspection" npm run test:m1-gui
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
npm run typecheck && npm run lint && npm test
npm run test:e2e && npm run test:web
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
# 构建独立评审包，不启动旧应用：
CARGO_TARGET_DIR="target-v1" npm run tauri -- build --config "src-tauri/tauri.m1-inspection.conf.json" --bundles app
```

## 收口增量（2026-10-15 第二轮）

### B 项：TIFF / HEIC / RAW 原图检视（已交付）

- **TIFF**：直通检视（image crate 校验格式，WKWebView 原生渲染全分辨率），上限 64MiB/3200 万像素不变。
- **HEIC**：ftyp 盒 brand 魔数校验（heic/heix/hevc/hevx/mif1/msf1）后直通 WKWebView 原生渲染；真实样本（52KB HEIC）验证。
- **RAW（cr2/cr3/nef/arw/dng/orf/rw2/raf）**：不做全尺寸解码——经 `/usr/bin/sips` 提取**相机内嵌 JPEG 预览**（CR2 实测 1936×1288），按内容 hash 缓存复用；检视返回 `kind: "embedded_preview"`，前端显示「RAW 嵌入预览 · 非全分辨率」标注。原 RAW 文件 hash 不变。文件上限 512MiB（不解码像素仅抽预览）。
- 前端三层（repository zod / PhotoViewport 标注 / data-source 属性）+ 检视测试 7 项全过；伪造 RAW 在导入即失败。

### C 项：存储增长实测与 O(n²) 修复（ADR 0026）

500 命令基线实测发现回执嵌套导致 **7.99GB 膨胀**；修复后 **63.7MB**（126 倍缩减）、编辑吞吐 2→28 命令/秒。顺带修复重放分支 FK bug。重放语义精确化为核心字段一致 + 状态不变双断言。

### C 项续：色彩边界实测（ADR 0027）

- **16 位 PNG**：全流程可用，缓存降 8 位（行为锁定）。
- **ICC JPEG**：用系统 Display P3 profile 生成真实 ICC 样本；缩略/预览缓存**不保留 ICC**（宽色域预览可能偏色），**原图检视保留 ICC 字节**（直通 WKWebView 系统色彩管理）。
- **不是 ICC 感知管线**——预览与原图的色彩差异边界已通过测试锁定，不冒充通过；校准显示器/HDR 验收待外部设备。

### A 项：原生 500 图验收侦察（未完成，如实报告）

- 原生 release app 启动验证：窗口 1440×887 在屏、CGWindowList 可查、菜单栏可达。
- **阻塞**：本会话无屏幕录制权限（Vision OCR 全空）+ Tauri 窗口不发布 AXWindow（System Events windows=0）+ 坐标点击无视觉反馈闭环——原生 UI 之旅无法可靠自动化，不冒充验收。
- 已备妥：500 张素材（/tmp/bf-native-500，250 PNG+250 JPEG，SHA-256 基线）、Swift 点击工具（/tmp/click）、进程监控。原生验收待有视觉通道的操作者执行。

## 当前剩余问题与停止点

以 ADR 0028 / 最新修复报告为准：M1 仍 partial。RAW 当前只有有界派生预览，真正 RAW 细节/嵌入 JPEG、原生 500 图、独立真实高像素素材、专业色彩与低端设备仍待验收；长期事件读取/大量版本的增长需另外评估。旧回执重放和增长统计问题已修复，不再列为待修复缺陷。
