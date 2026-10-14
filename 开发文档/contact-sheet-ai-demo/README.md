# 序间 BETWEEN FRAMES

## 当前需求基线：v1.0（2026-10-13）

最终实施需求已原样纳入 [`planning-v1.0/`](planning-v1.0/README.md)，后续以 [`FINAL-IMPLEMENTATION-SPEC`](planning-v1.0/docs/FINAL-IMPLEMENTATION-SPEC.md) 为准。新要求明确动态工具闭环、可暂停任务、后端预算、检查点恢复、上传 consent 和 15 条 Agent 验收；需求包不等于已完成软件。

现有工程复用，先按新 M0 补齐/复验 GUI 与 200 张测试图→25 张序列的离线重启，再完成 M1 统一命令、版本、Undo/Redo 和幂等。M2 才实现动态 Mock Agent，M3 接真实视觉，M4 记忆，M5 Beta。新初始 SQL 与当前数据库不兼容，升级需专用兼容 migration。

当前 **v1.0 M0–M5 工程实现全部完成，停止等待整体评审**：M5 补齐 Keychain 密钥存储（env→钥匙串解析链）、脱敏诊断命令、性能基线（Release：200 张导入 2.9s、决策 p95 23ms，均优于规格目标）、Beta 完整旅程 E2E（意图→选片→委托→草案→假设纠正→采纳→重启一致）与隐私文档。43 Rust 测试、24 前端单测、16 预览 E2E、3 真实后端集成旅程全部通过。**待外部条件**：签名/公证需开发者证书、20 位用户 Beta 需真人、真实模型 E2E 需 API Key。

Beta 试用包（unsigned debug，arm64）：`between-frames/src-tauri/target-v1/debug/bundle/macos/Between Frames M5 Review.app`（独立 identifier，不影响既有数据）；试用指引见 [BETA-GUIDE](docs/BETA-GUIDE.md)，隐私边界见 [PRIVACY](docs/PRIVACY.md)。契约检查已覆盖 GUI 与 Agent 两类契约（`npm run contracts:check`）。

本机评审应用：`between-frames/src-tauri/target-v1/debug/bundle/macos/Between Frames M1 Review.app`，使用新的 `studio.betweenframes.m1review` 数据目录，用户原应用和 M0 Review DB 未升级。浏览器预览为 `http://localhost:1420/`，真实存档使用桌面版。详见 [`测试与剩余问题`](docs/TEST-REPORT.md)、[`文件变更`](docs/V1-M1-FILES.md)、[`ADR 0004`](docs/ADR/0004-v1-m1-command-history.md)、[`M0 历史报告`](docs/V1-M0-TEST-REPORT.md)。记忆/上下文补充进入 [`后续约束`](docs/MEMORY-CONTEXT-ADDENDUM.md)，不提前实现。

需求接入的历史差异见 [`V1.0-ADOPTION`](docs/V1.0-ADOPTION.md)、[`ADR 0002`](docs/ADR/0002-adopt-v10.md)。当前工程目录已位于 `开发文档/contact-sheet-ai-demo/`。

## v0.2 历史需求接入（已被 v1.0 替代）

当时的 GUI × Photography Agent 需求已原样纳入 [`planning-v0.2/`](planning-v0.2/README.md)。[`DEVELOPMENT-SPEC v0.2`](planning-v0.2/docs/DEVELOPMENT-SPEC.md)、旧规格、Demo 与验收记录保留为历史和视觉参考。

现有工程仍是 v0.1 的 M0/M1 实现，**尚未满足新版 M1**。下一步先补统一 DomainCommandService、命令幂等、序列版本、Undo/Redo、批量编辑与导出隐私，再实现新版 M2 的受控 Mock Agent 和两份持久化草案。新 SQL 不能直接应用到当前数据库，升级使用新增兼容 migration。

需求差异、契约缺口及验收覆盖见 [`docs/V0.2-ADOPTION.md`](docs/V0.2-ADOPTION.md)，架构决定见 [`ADR 0001`](docs/ADR/0001-adopt-v02.md)。本次需求接入没有改动现有应用代码或项目数据。

## 已有 v0.1 M1 本地核心（2026-10-13）

桌面工程位于 [`between-frames/`](between-frames/README.md)：Tauri v2 + React + TypeScript + Vite + Rust/SQLite。M0 已评审通过；M1 接入只读导入、缓存、人工决策与序列持久化、比较、事件与导出。原有 Demo 保留。

```bash
cd "between-frames"
npm ci
CARGO_TARGET_DIR="target-v1" npm run tauri dev -- --config "tauri.review.conf.json"
# 真实本地编辑使用桌面版；npm run dev 仍为浏览器示例预览
```

桌面数据在重启后保留，原图不被改写；浏览器示例仍在刷新后重置。当时验收记录见 [`历史报告`](docs/V0.1-M1-TEST-REPORT.md)，当前边界见 [`docs/KNOWN_ISSUES.md`](docs/KNOWN_ISSUES.md)。

---

**从照片，到作品。** 面向摄影创作者的 AI 选片、序列编排和审美记忆工作台。当前仓库是一套**可操作的交互 Demo + 完整工程需求包**，不是已经完成的生产级 AI 软件。

## 交付物

| 文件 | 作用 |
|---|---|
| `demo-standalone.html` | **单文件版**，直接在浏览器打开，包含全部内嵌示例图片。推荐先体验这个。 |
| `index.html` + `assets/` | 可维护源代码版 Demo，需要用本地静态文件服务打开。 |
| `docs/PRODUCT-AND-ENGINEERING-SPEC.md` | 产品定义、页面、UX、设计系统、技术栈、数据模型、AI 工作流、验收标准。 |
| `docs/AGENT-EXECUTION-PROMPT.md` | 可直接复制给编码 Agent 的分阶段执行命令。 |
| `docs/schema.sql` | 生产版 SQLite 初始数据结构草案（作为 migrations 基础）。 |
| `docs/AI-OUTPUT.schema.json` | AI 编辑提案的严格 JSON 结果模型。 |
| `screenshot-initial.png` | 初始工作台 UI 截图（仅预览）。 |

## 最快使用

1. 下载并打开 `demo-standalone.html`。在 Chrome / Edge / Safari 中体验。
2. 点击 `模拟 AI 初选`，查看与人工入选状态分离的推荐候选。可单张保留或点击「采纳剩余建议」。这里仅为确定性规则模拟，不是视觉 AI。
3. 点击照片，使用 `K` 保留、`X` 淘汰、左右箭头切换，或右侧按钮进行操作。
4. 进入「序列」，拖动照片重新编排。
5. 使用「两图对比」，记录偏好；右侧「审美记忆」查看决策事件。
6. 点击导出，输出 JSON / CSV / 打印 Contact Sheet。
7. 导入自己的 JPEG / PNG / WebP / TIFF（Demo 只保障常见浏览器能预览的格式；TIFF 能否预览取决于浏览器）。

### 本地缓存说明

- Demo 元数据优先保存在浏览器 localStorage；用户导入图片使用 IndexedDB 持久化。具体持久化行为依赖浏览器安全上下文和本地数据权限；隐私模式、受限 `file://` 环境可能不可用。
- 如直接打开 HTML 后图片导入无法持久化，使用文件服务器运行源码版：

```bash
cd contact-sheet-ai-demo
python3 -m http.server 8080
# 浏览器打开 http://localhost:8080
```

- 不会联网，不会调用 AI API，不会上传原始图片。本地浏览器数据清理后，导入的图片可能丢失。生产软件须迁移到 Tauri + SQLite + app cache。
- standalone 体积较大是因为 28 张演示图片作为 data URL 内嵌，属于预期情况。

## 给开发 Agent 的指令

**将整个文件夹提供给 Agent**，然后把 `docs/AGENT-EXECUTION-PROMPT.md` 的全文发送给 Agent。不要只发 screenshot，也不要要求 Agent 把 Demo 的单文件 JS 当作生产代码继续堆功能。正确做法是根据视觉与行为参考拆为组件化 React/Tauri 应用。

## 当前 Demo 已实现 / 未实现

| 功能 | 状态 |
|---|---|
| Contact Sheet、多状态选片、项目意图编辑 | 可操作 |
| 28 张本地演示图片 | 可操作 |
| 自有图像导入（浏览器存储） | 可操作，环境可能限制 |
| 模拟 AI 初选 | 可操作、**假数据规则演示** |
| 作品序列、拖拽排序、两图比较 | 可操作 |
| 人工决策记录/备注、基础审美记忆时间线 | 可操作 |
| JSON/CSV/打印 | 可操作 |
| 多模态模型真实视觉理解 | **未实现** |
| 真实跨项目的审美偏好推断 | **未实现** |
| 生产桌面后端、Rust、SQLite | **未实现**，已给出开发规格 |
| 真实摄影书排版与 PDF 输出 | 第二阶段 |

## 重要的产品逻辑差别

演示版点击「模拟 AI 初选」只产生独立的「AI 建议」状态，不自动改变最终选片结果。用户逐张保留或显式点击「采纳剩余建议」后，才修改作品状态。

正式产品还需将 AI suggestions 和 human decisions 分库表保存、支持建议版本、复核与撤销；见规格书。

## 素材说明

演示图像由 `scikit-image` 随包样例素材裁切、色彩处理而得，仅用于产品功能演示。准备商用前请更换为自有拍摄或取得明确授权的摄影作品，不应默认这些素材适合商业再分发。
