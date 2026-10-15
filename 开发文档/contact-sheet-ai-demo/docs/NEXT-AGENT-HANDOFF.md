# Between Frames：接手开发交接说明

当前入口：[M1 原生 500 图报告](V2-M1-NATIVE-500-TEST-REPORT.md)。生成 PNG/JPEG 图库的原生功能切片已补验，50 表/500 源 hash 重启一致，胶片条 CSS 已修复。本轮重跑 120 Rust 默认、147 前端、4 M1 GUI；累计基线的额外格式样本与其他 E2E 未重跑。下一步优先 B（真正 RAW 细节与方向/色彩矩阵）及 C（WKWebView 总资源、长期/低端性能），不再把原生 500 图功能当作未做。M1 整体仍 partial，不能进入新 M2。以下旧数字和旧配置说明为历史交接基线，以当前报告及本段为准。

更新：2026-10-15。本文件交接现有工作，不表示下列待办已经实现或验收。

## 可直接交给下一位 AI 的任务

请接手 Between Frames 当前工作区，继续补齐 **v2 M1**。先完整阅读下列入口并检查实际工作区差异，保留所有已有未提交和未跟踪文件。第一轮优先推进 **受控 RAW 细节检视与方向/色彩样本矩阵**，并补 **WKWebView 总资源和低端性能证据**；基于实测修复可复现问题、补充有意义的回归并更新报告。不要进入新的 M2/AI 服务开发。不要升级原用户库、覆盖历史迁移、清理工作区或提交推送。每轮报告实际通过项、证据与剩余问题，环境不足的部分保持 partial，不用浏览器 shim、生成图或软件 GPU 数据冒充原生、真实素材或低端硬件验收。

## 工作目录与阅读入口

真正应用根目录：

```text
/Users/yihua/Documents/film/开发文档/contact-sheet-ai-demo/between-frames
```

以下相对路径均从该目录出发：

1. `AGENTS.md`、`../README.md`、`../docs/TEST-REPORT.md`。
2. `../docs/V2-M1-NATIVE-500-TEST-REPORT.md`：当前结果与剩余项；`V2-M1-REVIEW-FIXES-TEST-REPORT.md` / ADR 0028：不能回退的三项修复；`V2-M1-INSPECTION-TEST-REPORT.md` 保留历史。
3. `../docs/ADR/0025-m1-original-inspection-and-gui-projection.md`。
4. `../docs/V2-M1-500-GUI-TEST-REPORT.md`、ADR 0024：500 图基线和测试方法。
5. `../docs/V2-REVIEW-FIXES-TEST-REPORT.md`、ADR 0023：已有评审修复，禁止回退。
6. `../docs/V2-COMPLIANCE.md`、`../docs/V2.0-ADOPTION.md`：区分当前总账与历史差距快照。
7. 最新需求包 `/Users/yihua/Documents/film/BetweenFrames-v2.0-Implementation-Kit/` 中的 `docs/FINAL-PRODUCT-ENGINEERING-SPEC.md`、`docs/EXECUTION-PLAN.md` 和验收矩阵；按 M1 范围落实。旧 v1 M0–M5 完成记录不能替代新版里程碑验收。

## 历史交接基线（以当前入口为准）

- **v2 M1 仍 partial**，总账 **17 pass / 7 partial**；不是“v2 已完成”。
- 最近通过：**113 Rust、146 前端单测、26 E2E**；契约、类型、Lint、fmt、严格 Clippy、Desktop/Web 和独立原生构建通过。性能基线 1 项 ignored，钥匙串写删回环 1 项跳过。
- 500 图浏览器 GUI 连接真实 release Rust/SQLite，完成导入、虚拟滚动、决策/评分/备注、25 图序列、Undo/Redo、版本恢复、导出和不同 PID 的进程恢复；500 个源 hash 不变。**系统选择器在这组自动化里仍模拟。**
- 500 图素材是 250 PNG + 250 JPEG、1200×800，从仓库 28 张参考图派生；不是独立摄影师原片，也不是大型 RAW 集。
- JPEG/PNG 原图检视已实现：按项目/照片 ID 查所属关系和 SHA-256，限 64 MiB / 3200 万像素；点击后精确授权文件，适合屏幕继续使用缓存。Retina 下原图 100% 是物理屏幕像素 1:1。其他格式明确回退，不能把 1400px 缓存叫原图。
- GUI 的 `compact: true` 仅去掉返回副本中各历史事件的完整前后状态/上下文；数据库审计、持久化幂等回执、Undo/Redo、导出未删减。40 次编辑时返回 JSON 从 5,718,837 降至 510,106 UTF-8 字节。
- M4/10 核/24GiB 的浏览器采样：40 次编辑后 JS Heap 74.4MiB（旧基线 220.8）；最终 Renderer RSS 464.8MiB；按键→下一帧 p95 66.7ms。不是峰值、泄漏排除结论或原生/低端机性能证明；GPU 为 SwiftShader，硬件利用率未获得。
- 原生 **2 张 2400×1600 生成 JPEG/PNG** 已通过真实文件选择、文件夹选择、重复跳过、原图/同步比较、JSON 保存与完整 Quit/重启；49 表内容与源 hash 不变，Agent task 为 0。**原生 500 图功能现已补验，完整 WebView/低端性能仍待验收。**
- 上述实现和证据有大量未提交/未跟踪文件。不要只按 HEAD 判断进度；不得 `git reset`、`git clean` 或覆盖整目录。是否提交由用户另行决定。

## 后续顺序与验收标准

### A：原生 500 图功能已补验；保留复现方法

本节 1–4 的生成图库功能流程已完成，见当前报告；资源只采样到主进程，WKWebView 总资源和低端机仍待验收。修复后的应用标识为 `studio.betweenframes.m1reviewfixes`，配置 `tauri.m1-review-fixes.conf.json`，不要使用下方旧标识。

1. 使用独立评审应用和测试图库，经真实系统文件夹选择器导入 500 张。保留上轮基线，新增独立证据目录。
2. 在原生 WKWebView 检查首/中/尾虚拟网格、快速决策、批量选片/评分、灯箱、原图/双图对比、序列排序、Undo/Redo、版本恢复和导出；完整退出重启后核对状态。
3. 记录构建方式、机器、素材真实尺寸/总量、导入耗时、进程 CPU/RSS、可获得的交互延迟。原生指标若无法直接取得，要写清采样方法和限制，不能套用 Chromium CDP 数值。
4. 操作前后逐张比对 SHA-256；导出回执核对实际文件；重启确认旧进程退出、新 PID、业务数据一致。只移动测试副本来验证失联/重连。
5. 没有摄影师素材或低端设备时，可先完成生成图库的原生验收和瓶颈修复；把真实高像素素材/低端设备标为待外部验证。

### B：补齐其他格式的原图检视

- 按需求确认 TIFF、HEIC、RAW 的支持矩阵，补安全的本地解码与按需资源读取。禁止直接放开任意路径或把整个原图目录加入 asset scope。
- 超过现有检视上限时先设计受控解码/分块/缓存方案，不能仅提高常量导致双图比较无界分配。
- 验证 EXIF 方向、实际解码尺寸、失败回退、取消/翻片迟到响应和原文件不变。RAW 若只能取嵌入预览，明确标注，不能声称是全分辨率解码。
- 生成的本地解码缓存与云视觉资源保持不同用途；不得把新原图能力接入 AI 上传。

### C：色彩、长时间性能与 M1 收口

- 建立带来源说明的 ICC/方向/高位深样本；明确缓存与原图的色彩处理边界。系统 WebView 能显示图片不等于 ICC/HDR/16 位或校准显示器验收通过。
- 实测长期编辑与大项目的 SQLite/回执增长。若确有瓶颈，优先研究只读投影/按需历史，保护完整审计、旧幂等重放和 Undo/Redo；不擅自删除历史或重写旧迁移。
- 低端设备缺失是外部验收条件；不要伪造达标。更新 M1 报告/总账，逐项说明关闭项与未关闭项，经用户评审再进入新的 M2。

## 关键文件

| 范围 | 文件 |
|---|---|
| 原图所属校验/格式和上限 | `src-tauri/src/photo_inspection.rs` |
| Tauri 精确文件授权/精简返回副本 | `src-tauri/src/commands.rs`、`src-tauri/src/lib.rs` |
| 图像导入/方向/1400px 缓存 | `src-tauri/src/image_pipeline.rs` |
| GUI 仓库与延迟原图能力 | `src/services/TauriRepository.ts`、`src/domain/models.ts` |
| 显示/缩放/异步响应 | `src/components/shared/PhotoViewport.tsx`、`Lightbox.tsx`、`CompareModal.tsx` |
| 完整项目/历史/持久化回执 | `src-tauri/src/db.rs`、`command_handler.rs`、`command_audit.rs` |
| 500 图与 Retina GUI 自动化 | `e2e/m1-500.spec.ts`、`playwright.m1.config.ts` |
| 测试素材/真实后端驱动 | `src-tauri/src/harness_fixtures.rs`、`src-tauri/src/bin/m1-harness.rs`、`scripts/integration-server.mjs` |
| 新增回归 | `src-tauri/tests/photo_inspection.rs`、`src/components/shared/PhotoViewport.test.tsx`、`src/services/TauriRepository.test.ts` |
| 独立原生配置 | `src-tauri/tauri.m1-review-fixes.conf.json` |

## 验证命令与环境

在应用根目录执行；选择与改动相匹配的验证。交付里程碑前完成必需回归，记录实际输出。

```bash
npm run contracts:check
npm run typecheck
npm run lint
npm test
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
npm run test:e2e
npm run test:web
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
BF_M1_ACCEPTANCE_SLICE="v2-m1-next" npm run test:m1-gui
npm run build:workspaces
CARGO_TARGET_DIR="target-v1" npm run tauri -- build --config "src-tauri/tauri.m1-review-fixes.conf.json" --bundles app
```

- `test:integration` 与 `test:m1-gui` 共用 1420/1422，必须串行。改 Rust 后先重建对应 debug/release harness。
- 1424 是 Web 示例预览，本地测试可复用现有服务，不要杀掉用户的预览。它使用 MockRepository，不能证明原生能力或桌面原图读取。
- 当前原生评审标识为 `studio.betweenframes.m1reviewfixes`（`studio.betweenframes.m1inspection` 为旧标识，不启动），数据库位于其独立 app data；原用户库和旧 `studio.betweenframes.v2foundation` 等评审库未升级，禁止用新二进制启动那些标识。
- 新评审库已 schema 17。旧迁移 0001–0017 不可修改；若确需新迁移，先满足项目与用户的确认要求，使用临时旧库验证备份/升级/回滚，不能直接尝试用户库。
- 自动化显式禁用真实 API Key；不写删钥匙串、不联系生产模型、不伪造 GPU 或真实摄影师验收。
- 最新证据在 `../docs/evidence/v2-m1-native-500/`，浏览器回归截图在 `screenshots/v2-m1-native-500-regression/`；原生截图仅在 CUA 会话内检查，不能声称存在本地 PNG。含结果、原始日志、50 表摘要、500 源 hash 和源码 SHA-256。保留旧证据与历史截图。

## 每轮交付

可运行的实现及适当测试；新增 ADR（涉及架构时）；实际功能/性能验收结果；独立日志/截图；更新 `TEST-REPORT.md`、`V2-COMPLIANCE.md` 和 `AGENTS.md`。明确仍未验证的格式、素材、设备、色彩及外部服务条件，不把“可构建”写成“已验收”。
