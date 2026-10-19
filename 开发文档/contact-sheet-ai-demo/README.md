# FocalRoom

**最新开发接续（2026-10-19）**：[详细交接与剩余任务](docs/DEVELOPER-HANDOFF-2026-10-19.md)。API/Web及0014/0015、真实邮件登录/收件已验；官方AI生产仍关闭，正式安装包/公证/官网下载及M0–M5其余验收未全部完成。以下旧阶段记录保留历史，不作当前实施顺序。

**使用文档**：[详细操作说明书](docs/FOCALROOM-USER-MANUAL.md) · [浏览器阅读版](docs/FOCALROOM-USER-MANUAL.html)。覆盖桌面选片、Agent、记忆、导出、共享迁移与 Web 多人评片，按当前界面说明功能边界。

**当前接续：M4/M5 共享协作（2026-10-17）**。按用户要求推进到 M5：新增可运行的 NestJS/PostgreSQL 权威服务、桌面真实迁移向导、Web 登录/邀请/私有预览/批注/草案修订/逐项审批/历史版本、实时 Presence/重连/撤权和受限评论 Task 工具。共享不会授予 AI 上传或作者记忆权限。可在 `between-frames` 运行 `npm run review:shared` 启动独立本地评审；全部证据、测试、失败记录和边界见 [M4/M5 验收报告](docs/V2-M4-M5-SHARED-AUTHORITY-TEST-REPORT.md)、[部署说明](between-frames/ops/shared/README.md) 与 [剩余计划](docs/V2-REMAINING-PLAN.md)。M3 专业质量、M5 真人双人项目和真实生产部署仍待完成，未开展 M6/M7。下方阶段记录保留历史，当前状态以本段及链接报告为准。

产品最终名称为 **FocalRoom**（原 Between Frames / 序间）。品牌更新沿用现有应用标识、内部包名与数据库位置；旧规格和验收报告中的旧名称保留历史含义。

名称、透明图案、favicon 与应用图标已接入，见 [品牌更新记录](docs/FOCALROOM-BRAND-UPDATE.md)。原图直接像素抠图，去除全部文字与白底；180 单测、17 浏览器回归与独立桌面打包通过，用户视觉评审待确认。

**最新接续：M3 同端点模型分工（2026-10-17）**：策展主循环与图像观察可配置不同型号，能力文件分别冻结；图像授权按实际观察型号校验，任务恢复绑定两个角色，缓存和检索跟随观察来源，预算仍统一累计。默认复用原模型，未增加跨服务 fallback。190 桌面 Rust +13 本地云、183 前端、16预览/6后端/1Web、独立双角色界面与 debug 打包通过；252 源摘要与90起始PNG一致。本轮无真实 Provider/用户照片请求或用户库操作，成本优势与摄影质量未测，M3 仍 partial。见 [验收报告](docs/V2-M3-MODEL-ROLES-TEST-REPORT.md)、[配置](docs/MODEL-CAPABILITIES.md) 和 [剩余计划](docs/V2-REMAINING-PLAN.md)。

**最新接续：M3 实际服务图像授权（2026-10-17）**：授权绑定实际端点与型号，更换服务不能沿用旧同意，旧通用正向授权不会自动升级。新增主动管理/取消/全服务禁止入口，管理授权不误启动任务；HTTP 重定向不会自动转发预览。无新迁移或依赖；M3 仍 partial，跨供应商 fallback 执行、模型分工与专业质量仍待完成。见 [本轮报告](docs/V2-M3-PROVIDER-CONSENT-TEST-REPORT.md)、[剩余计划](docs/V2-REMAINING-PLAN.md)。

全量 exit=0：181 桌面 Rust +13 本地云、180 单测、16 预览/6 后端及严格检查通过；截图 QA 后前端与两个 E2E 套件再通过，独立 debug 构建通过。本轮无真实服务/用户照片请求，Prompt 仍 m3-openai-8 / m2-mock-2。下文为历史切片。


**最新接续：M3 本地作者记忆接线（2026-10-17）**：Memory Studio 可明确授权离线任务引用其他活跃 LOCAL 项目已确认的相关假设，附来源和证据；可编辑列表保持当前项目独立。授权撤回或来源变更后旧任务需重新委托，云模型和共享项目不参与。本轮复用现有命令/回执/检查点，无新迁移或依赖；M3 仍 partial，完整 Creator 隔离、云 Provider 专属授权、模型分工和专业质量仍待完成。见 [本轮报告](docs/V2-M3-AUTHOR-MEMORY-TEST-REPORT.md)、[剩余计划](docs/V2-REMAINING-PLAN.md)。

最终各项验证：174 桌面 Rust +13 本地云、173 单测、16 预览/6 后端，独立 debug 构建通过。首轮新增 UI 测试的同步状态断言失败，修正为等待持久化后，完整后端套件 6/6 通过，原始失败 trace 保留。Prompt m3-openai-8 / m2-mock-2；无本轮真实模型回归。以下为历史切片。


**最新接续：M3 持久化增量向量索引（2026-10-17）**：已有缓存事实的本机向量可跨连接重开复用，只更新变化照片；当前模型/来源/任务范围不符会拒绝复用。每项目有界文件缓存、损坏重建和清空联动已实现，真实中英文验证了冷启动3条、更新1条时复用2条/生成1条。没有新增图片上传、依赖或迁移；M3仍 partial，详细结果与局限见 [本轮报告](docs/V2-M3-PERSISTENT-INDEX-TEST-REPORT.md)。下文“向量不落盘”为上一切片历史。

当前各项验证：165桌面Rust +13云、169单测、16预览/6后端，3项显式原生测试与debug构建通过；后端首轮受打包刷新开发页干扰，顺序重跑6/6通过，原始失败记录保留。

**最新接续：M3 本地语义候选检索（2026-10-17）**：新增真实 Apple 本机中英文句向量检索，仅使用任务范围内已有缓存事实，保留原关键词工具、明确来源及不可用回退。中文640/英文512维原生测试和 Agent 工具链已实测；语义相似度不代表审美或事实置信度。补现有云模块 Viewer 只读与严格检查回归，无新迁移/上传/生产操作。M3 仍 partial，持久化增量索引、模型分工与质量评测继续推进；完整结果和剩余问题见 [本轮报告](docs/V2-M3-LOCAL-SEMANTIC-TEST-REPORT.md) 与 [剩余计划](docs/V2-REMAINING-PLAN.md)。下文云状态和计数保留历史，当前生产状态须独立核验。

本轮全量通过：160桌面Rust +13云、169单测、16预览/6后端；额外2项原生测试、独立debug构建及弱链接检查通过。DeepSeek Prompt7复验尚未执行（旧凭据图片无法识别、0请求），不作为通过项。

**最新接续：M3 模型能力配置（2026-10-16）**：未知模型/代理不再默认具备视觉或工具能力；可信配置、组合请求门槛和能力变更后的恢复拒绝已接入，界面错误与拒绝图像路径可操作。151 桌面 Rust + 11 云、169 Vitest、16 预览/6 后端通过；独立 debug 应用构建通过。DeepSeek 无图回归 3 轮/4 工具、13,907 实际 token、3 美分估算，正式项目不变。M3 整体仍 partial，见 [本轮报告](docs/V2-M3-CAPABILITIES-TEST-REPORT.md) 和 [配置说明](docs/MODEL-CAPABILITIES.md)。

**最新接续：M3 实际模型与缓存事实检索（2026-10-16）**：路由/检查点使用实际配置并拒绝恢复时静默换端点，缓存按端点隔离，支持范围内缓存事实关键词检索且不新增上传。144 桌面 Rust + 11 云、164 Vitest、16 预览/6 后端全通过；DeepSeek 新 Prompt 文件名任务 3 轮/4 工具、13,792 实际 token、3 美分估算、0 图片。独立桌面包构建通过；真实摄影质量仍待评测。见 [本轮报告](docs/V2-M3-ROUTING-SEARCH-TEST-REPORT.md)。

**最新接续：AI 模拟评审（2026-10-16）**：按用户授权，现有 LUMIX 候选完成 2 条模拟作者/编辑评审的录入、导出与分来源汇总；0 条真人，正式作品与审美记忆不变。新增来源标记、旧评语未核实处理、冻结批次 --reviews 导入；15 项定向评测测试通过。见 [本轮报告](docs/V2-M3-SIMULATED-REVIEW-TEST-REPORT.md)。模拟支持工程继续，真实质量门槛保持单列。

**最新接续：已有云切片修复（2026-10-16）**：仓库已有独立 axum/SQLite 本地 API，现修复单 grant 撤销误伤、密码/会话、注册事务、预览边界和首次启动，134 桌面 Rust + 11 云测试、159 Vitest、16 预览/6 后端及严格检查通过。腾讯云尚无部署、Postgres 权威尚未实现；M3 质量和 M4 整体仍 partial。见 [本轮报告](docs/V2-M4-LOCAL-BOUNDARIES-TEST-REPORT.md)。下方“没有 API 代码”为旧状态，不能误解为没有本地实现。

## 新方向：v2.0（2026-10-15）

**最新 M3 增量（2026-10-16）**：用户授权 LUMIX 真实照片，80 张中抽样观察 12 张并形成 8 张待评审草案，原图 hash 和正式项目不变；保留前次失败，尚无真人/竞品质量结论。新增预算收敛、版本化费率和离线评测；134 Rust、159 Vitest、16 预览、6 后端通过。M3 仍 partial，腾讯云目前没有 Between Frames API，不能宣布全计划完成。见 [M3 报告](docs/V2-M3-EVALUATION-TEST-REPORT.md) 与 [剩余计划](docs/V2-REMAINING-PLAN.md)。

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
