# 序间 BETWEEN FRAMES

## 新方向：v2.0（2026-10-15）

用户提供了 `BetweenFrames-v2.0-Implementation-Kit`，方向升级为专业摄影 Agent、创作工作区、多人协作与后续私密专业服务。方向差异和阶段映射见 [V2.0-ADOPTION](docs/V2.0-ADOPTION.md)。**M0 工程/契约切片已本地验收**：npm workspaces、Desktop/Web 独立入口、共享 UI、TS/Rust 契约与权限骨架、CI 配置；83 Rust、134 单测、17 浏览器 E2E、5 后端旅程通过，独立原生包导入/选片/退出重启通过。M0 整体仍 partial/待架构与 UI/UX 评审；云服务、v2 生产迁移、远端 CI 均未验收，原用户库未升级。见 [M0 报告](docs/V2-M0-TEST-REPORT.md) 和 [启动说明](between-frames/V2-M0-README.md)。旧版 M0–M5 完成记录不代表新版阶段已通过。

**v2 评审修复（2026-10-15）**：八项缺陷修复与回归完成，110 Rust、142 单测、16 预览、1 Web、6 条真实后端旅程通过；新增 0017 仅用于临时库兼容验证，原用户库未升级。见 [修复报告](docs/V2-REVIEW-FIXES-TEST-REPORT.md)。撤回“v2 工程侧完成”结论，当时 [验收总账](docs/V2-COMPLIANCE.md) 为 16 pass / 8 partial，云共享、Creator 隔离及真实支付仍需继续开发。

**M1 的 500 张 GUI 切片（2026-10-15）**：500 张混合格式实际文件完成页面选片、序列、版本恢复、导出与新 Rust 进程重启，源 hash 前后一致；补齐取消/失败/重连验收，110 Rust、142 单测、25 E2E 通过。复现：`npm run test:m1-gui`。当前 [总账](docs/V2-COMPLIANCE.md) 为 17 pass / 7 partial；M1 整体仍 partial，原生选择器、原图 1:1/色彩与低端机/内存优化尚未关闭。见 [M1 报告](docs/V2-M1-500-GUI-TEST-REPORT.md)。

**M1 原图与内存切片（2026-10-15，当前）**：JPEG/PNG 按需原图 1:1 / Retina、同步比较、精简 GUI 历史、独立原生选择器/保存/退出重启已验收；113 Rust、146 单测、26 E2E 通过。40 次编辑后投影减少 91.1%，JS Heap 从基线 220.8 降至 74.4MiB。49 表和原图 hash 重启前后一致。M1 仍 partial，其他原图格式、色彩、原生 500 图/低端性能待补；原用户库未升级。详见 [当前 M1 报告](docs/V2-M1-INSPECTION-TEST-REPORT.md)。

**M1 接续评审修复（2026-10-15，当前）**：RAW 检视改为有界派生缓存并修正来源标识，幂等回执恢复首次完整结果，增长基准改按实际 1250 命令和回执增量计数；122 Rust 功能测试、1 增长基准、147 单测、27 独立 E2E 通过。旧 29 条 E2E 与 63.7MB/500 命令数字撤回。新原生标识完成边界图补验，未升级原用户库；M1 仍 partial。见 [当前修复报告](docs/V2-M1-REVIEW-FIXES-TEST-REPORT.md)。

**M1 原生 500 图与灯箱修复（2026-10-15，当前）**：独立 release 经真实系统文件夹/保存面板完成 500 图导入、重复跳过、25 图编辑/序列/版本与导出、完整 Quit/重启；50 表和 500 源 hash 不变。修复胶片条 CSS 未加载导致的布局溢出；本轮 120 Rust 默认、147 单测、4 M1 GUI 通过。素材是 1200×800 派生测试图，宿主主进程采样不代表 WKWebView 总内存；RAW 细节、专业色彩/高像素素材与低端硬件仍待补，M1 保持 partial。见 [最新 M1 报告](docs/V2-M1-NATIVE-500-TEST-REPORT.md)。

## v1.0 需求与验收历史（2026-10-13 起）

最终实施需求已原样纳入 [`planning-v1.0/`](planning-v1.0/README.md)，后续以 [`FINAL-IMPLEMENTATION-SPEC`](planning-v1.0/docs/FINAL-IMPLEMENTATION-SPEC.md) 为准。新要求明确动态工具闭环、可暂停任务、后端预算、检查点恢复、上传 consent 和 15 条 Agent 验收；需求包不等于已完成软件。

现有工程复用，先按新 M0 补齐/复验 GUI 与 200 张测试图→25 张序列的离线重启，再完成 M1 统一命令、版本、Undo/Redo 和幂等。M2 才实现动态 Mock Agent，M3 接真实视觉，M4 记忆，M5 Beta。新初始 SQL 与当前数据库不兼容，升级需专用兼容 migration。

当前 **v1.0 M0–M5 工程实现全部完成，停止等待整体评审**：M5 补齐 Keychain 密钥存储（env→钥匙串解析链）、脱敏诊断命令、性能基线（Release：200 张导入 2.9s、决策 p95 23ms，均优于规格目标）、Beta 完整旅程 E2E（意图→选片→委托→草案→假设纠正→采纳→重启一致）与隐私文档。43 Rust 测试、24 前端单测、16 预览 E2E、3 真实后端集成旅程全部通过。**v1.1 已完成**：版本对比（任意两版 diff）、新建项目模板、项目标签（编辑/筛选，独立元数据通道）、沉浸审片（灯箱内决策评分 + 胶片条导航）。
**v1.2 已完成**：印刷参考导出（A4 横版分页 HTML，打印即 PDF）、元数据侧车导出（每照片一 JSON + manifest）、册页跨图标记（spread/full 可撤销命令，印刷分页尊重标记）。
**本轮评审修复（2026-10-14）**：修复真实工具协议、视觉任务 IPC、桌面新导出入口、上传前预算预留、单次授权绑定与记忆纠正复活，补强导出/重启旅程。59 Rust、44 单测、16 预览与 3 后端集成通过；钥匙串写删未执行。原生选择器、实际 PDF 和退出重启已补验，Safari 页脚多分页已修复并跑相关回归；真实模型/计价仍待补验。见 [评审修复报告](docs/REVIEW-FIXES-TEST-REPORT.md) 与 [原生补验报告](docs/NATIVE-REVIEW-FIXES-REPORT.md)。本轮独立应用为 `between-frames/src-tauri/target-v1/debug/bundle/macos/Between Frames Fix Review.app`。
**M4 本地记忆控制增量（2026-10-15）**：新增假设确认/移除/恢复、证据排除/恢复、原文与反例查看、项目内 FTS5 搜索和任务记忆版本校验。记忆变更暂停执行中的任务，旧任务需重新委托；正式选片与 Undo 栈不变。全量验证 67 Rust、50 单测、16 预览、4 后端旅程通过，后续边界补验见 [记忆控制报告](docs/MEMORY-CONTROLS-TEST-REPORT.md)。跨项目与物理擦除未实现；审计与备份仍保留；原用户数据库未升级。解锁后已完成原生界面、旧任务拒绝/重新委托与完整重启补验：35 表快照一致，28 源 hash 不变。独立包：`between-frames/src-tauri/target-v1/debug/bundle/macos/Between Frames Memory Review.app`。
**M5 本地存储诊断增量（2026-10-15）**：新增可查看、刷新和主动复制的存储诊断；桌面与 harness 共用 Rust 投影，包含数据库/WAL/共享内存、缓存、迁移备份占用。计数失败与不完整统计明确显示，无新增迁移或依赖。72 Rust、56 单测、16 预览、5 真实后端旅程及原生补验通过，35 表和 28 源 hash 不变。详见 [M5 存储诊断报告](docs/STORAGE-DIAGNOSTICS-TEST-REPORT.md)。最新独立构建仍为 `Between Frames Memory Review.app`（55.34MiB，包含本轮诊断），原用户库未升级。
**规格符合性终审**：[V1.0-COMPLIANCE](docs/V1.0-COMPLIANCE.md)——P0 十二条与 AG-01–15 全矩阵逐条核对，可自动化范围内全部满足，无 Mock 假通过项。
**待外部条件**：签名/公证需开发者证书、20 位用户 Beta 需真人、真实模型 E2E 需 API Key。

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
