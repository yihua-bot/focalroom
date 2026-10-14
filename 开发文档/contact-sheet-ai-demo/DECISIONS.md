# 工程决策

## v1.0 M5 Beta 工程准备验收（2026-10-14）

Keychain 经系统 security 命令集成（无新依赖），解析顺序 env（开发显式）→ 钥匙串（用户存储），GUI 提供存入/移除；写入瞬间进程参数可见为已知权衡，PRIVACY 如实声明。性能以可复现基线测试固化并以宽松守卫防退化：Release 200 张导入 2.9s（70 张/秒）、决策 p95 23ms、投影 3.4ms，均优于规格目标（≤120s / <150ms）；Debug 数字保留对照。诊断命令明示脱敏边界；Beta 旅程 E2E 以真实后端串起完整剧本（含 AG-11 旅程级纠正），作为真实用户 Beta 前的自动化底座。

发布流程项如实未做：签名/公证（无证书）、DMG、真实用户试用。本轮自动化：43 Rust（keychain 2）、24 单测、16 预览、3 集成（beta-journey 52.2s）、release 构建与 perf 基线、typecheck/lint/严格 Clippy/fmt 通过。**六阶段工程实现至此全部落地，停止等待整体评审。** 见 [ADR 0008](docs/ADR/0008-v1-m5-beta-readiness.md)、[测试报告](docs/TEST-REPORT.md)、[文件清单](docs/V1-M5-FILES.md)、[隐私说明](docs/PRIVACY.md)。下文保留各轮历史决定。

## v1.0 M4 偏好记忆验收（2026-10-14）

L2 证据 dedup_key=command_id：幂等回放命中同一回执不增行，撤销/重做不产生（也不伪造反）证据；Agent 草案在独立通道，结构性排除于证据之外。L3 假设为确定性 2-gram 聚合（≥2 条独立证据才生成、默认 project_local），纠正即 corrected+停用检索且同表述不复活（AG-11 实测）。检索按确认状态/证据数/时间排序，假设与证据 ID 绑定返回；ContextBuilder 每轮限量注入 3 条。FTS5 未引入：v1 假设基数小，过滤+排序与 LIKE 语义等价（ADR 0007），量增后作性能优化。证据不提供删除入口（append-only 审计，留产品评审）。

过程修正：测试 receipt 快照 revision 滞后一步属既定行为（gui 辅助改回读权威状态）；幂等重放断言改为证据计数语义。本轮自动化：41 Rust（agent_memory 4）、24 单测、16 预览、2 集成、typecheck/lint/严格 Clippy/fmt 通过。**当前停止 M4 等待评审，M5 未启动。** 见 [ADR 0007](docs/ADR/0007-v1-m4-preference-memory.md)、[测试报告](docs/TEST-REPORT.md)、[文件清单](docs/V1-M4-FILES.md)、[计划](docs/PLAN-V1-M4.md)。下文保留各轮历史决定。

## v1.0 M3 真实视觉与授权验收（2026-10-14）

OpenAIProvider 走 Responses API tool calling（reqwest/rustls），store:false 由本地 checkpoint 保存并重发多轮 function_call/output；视觉观察是工具执行器内的独立子请求，预取持读事务、网络在写事务外，避免长事务持锁。授权三态（本次任务/本项目/禁止），配额按**真实传输张数**扣减——幻觉 photo_id 的观察被整体丢弃但传输已发生、配额仍扣，隐私语义优先；观察缓存键 hash+model+prompt+kind，命中零上传。API key 仅环境变量进内存、不落库不进前端（Keychain 留 M5，ADR 0006）；费用为保守估算（1 美分/图 + 1 美分/千 token）非账单。provider 由后端按 key 决定，checkpoint 校验 providerId/promptVersion 防跨 provider 续跑。

复验过程一次集成失败定位为 target-v1 残留 M2 旧 harness（不识别 consentId）；按报告命令重建后全绿，前端载荷与 Rust 校验确认一致。本轮自动化：37 Rust（agent_vision 4 含 AG-04 零外发/授权上传/缓存/幻觉/触顶）、24 单测（授权卡 3）、16 预览、2 集成、typecheck/lint/严格 Clippy/fmt/契约通过；**真实模型 E2E 无 key 未跑，不宣称通过**。**当前停止 M3 等待评审，M4 未启动。** 见 [ADR 0006](docs/ADR/0006-v1-m3-vision-consent.md)、[测试报告](docs/TEST-REPORT.md)、[文件清单](docs/V1-M3-FILES.md)、[计划](docs/PLAN-V1-M3.md)。下文保留各轮历史决定。

## v1.0 M2 受控 Mock Runtime 验收（2026-10-14）

Agent 通道只经 ToolRegistry（12 工具白名单 + 工程 schema 子集校验）与 PolicyGate（项目隔离、photo_id scope、draft 归属）；不存在 shell/SQL/网络能力，费用恒 0、云端图像 0。Mock Provider 每轮依据真实工具结果重选下一工具，AG-01/02/03 以双套观察数据的工具轨迹断言，不用固定流程冒充自主。任务请求/工具调用持久化幂等回执，outcome 先落库后执行；中断恢复重放检查点，不重复草案。草案审批走唯一 CommandHandler：全量采纳替换序列、局部采纳只改所选并保留其余人工状态、rebase 仅更新基线需重新预览。文本预算为保守 UTF-8 上界，真实 token/成本属 M3。

复验发现 integration 首败源于旧 target 残留 m1-harness 二进制（无 agent 命令），带 `integration-harness` feature 重建后全绿；正式复现统一使用 `CARGO_TARGET_DIR="target-v1"`。本轮自动化：33 Rust（agent_runtime 13）、21 单测、16 预览 E2E、2 集成旅程、typecheck/lint/严格 Clippy/fmt/契约检查通过；原生证据沿用同日上一轮 M2 Review 截图与快照，未重跑原生 .app。**当前停止 M2 等待评审，M3 未启动。** 见 [ADR 0005](docs/ADR/0005-v1-m2-mock-runtime.md)、[测试报告](docs/TEST-REPORT.md)、[文件清单](docs/V1-M2-FILES.md)。下文保留各轮历史决定。

## v1.0 M1 命令与恢复验收（2026-10-14）

唯一 CommandHandler 负责正式写入。14 类 GUI 命令从机器契约生成，后端注入身份；幂等别名、完整命令、补偿目标和回执持久化，授权先于回放。Undo/Redo 追加补偿事件，比较只逻辑撤回；命名版本恢复是可撤销编辑。版本保存/项目新建/资源导入不占人工撤销栈，旧日志不伪造追溯撤销。

0003 仅增加六张 M1 表，保留旧 migration；临时旧库验证包含 WAL 的一致性备份与恢复。最终原生使用新 M1 Review identifier，原用户 DB 和 M0 Review 保持原状。十二表重启一致、200 源 hash 不变；重启后重做追加一条补偿，不复制比较行。CSV/JSON 默认不输出已知资源路径字段，人工输入仍原文保留。

DRY 以单一契约生成双方命令/事件；SRP 分离事务、编辑、状态、资源和审计；DIP 保持 Repository 边界；KISS/YAGNI 不引入 AI 空模块或新依赖。Tauri 点号事件名问题已通过映射修正并用最终原生包验收。**当前停止 M1 等待评审，M2 未启动。** 见 [ADR 0004](docs/ADR/0004-v1-m1-command-history.md)、[测试报告](docs/TEST-REPORT.md)、[文件清单](docs/V1-M1-FILES.md)。下文保留各轮历史决定。

## v1.0 M0 GUI 验收（2026-10-13）

多选、搜索、明确 K/X/U、人工 0–5 星、Loupe、同步/独立比较、面板折叠与快捷键隔离已实现。评分使用新增 0002 兼容 migration，原 0001 保留。批量事务校验完整选区，重复状态 no-op 不改变序列。共享 PhotoViewport 避免两套缩放逻辑，100% 明确针对缓存预览。

原生验收使用独立 Review identifier；200→25 完整退出重启、六表一致、200 个源 SHA256 不变。旧用户应用数据库当时未升级。M1 当时尚未实现，随后用户评审通过并授权继续。见 [ADR 0003](docs/ADR/0003-v1-m0-gui.md)、[历史测试报告](docs/V1-M0-TEST-REPORT.md)。

## v1.0 最终需求基线（2026-10-13）

主规格为 `planning-v1.0/docs/FINAL-IMPLEMENTATION-SPEC.md`，v0.2 与下文作为历史。现有工程已位于 `开发文档/contact-sheet-ai-demo/`。复用已有 GUI/本地核心，先完成新版 M0 测试图 25 张离线旅程及 GUI 缺口，再补 M1 命令/幂等/版本/撤销；M2 使用动态 ModelProvider → 工具 → 反馈循环，不做固定 pipeline。新包自检仅证明新库 24 表与静态资料，未证明应用 Agent 已实现。草案 items/schema、旧库 paused CHECK 等差异及兼容升级原则见 [ADR 0002](docs/ADR/0002-adopt-v10.md) 和 [接入记录](docs/V1.0-ADOPTION.md)。本轮没有修改运行代码或实际数据库。

## v0.2 需求基线更新（2026-10-13）

用户提交新版产品需求。原样归档到 `planning-v0.2/`，以后按 DEVELOPMENT-SPEC v0.2 评估实现。现有 GUI/导入/缓存/事务继续复用，先补新 M0 的统一命令边界与新 M1 的命令幂等、版本、撤销、多选及导出隐私；M2 改为 Agent Runtime + 工具 + 两份持久化草案，旧 AIRun/AISuggestion 计划停止沿用。新 0001 仅为 fresh schema，不能覆写/直接升级既有 DB。包内重新初始化 M0 的文字是新项目模板，不能据此清空工程或撤销已有评审。具体见 [ADR 0001](docs/ADR/0001-adopt-v02.md)、[需求接入记录](docs/V0.2-ADOPTION.md)。下文保留 v0.1 历史。

## M1 原生拖动修正（2026-10-13）

用户要求继续开发，先完成 M1 剩余验收。序列独立把手使用立即激活的 PointerSensor，移除 6px 距离阈值，确保只有一次有效移动时也能更新位移。继续使用现有 dnd-kit 键盘/取消/排序和 Repository 持久化接口，不新增拖动状态机或依赖。原生鼠标排序与完整重启通过；关闭 Tauri 文件拖放处理器的试验未解决问题，已恢复原配置。临时事件诊断代码已移除。真实用户图库尚未提供，严格停留在 M1，未提前创建 M2 表或服务。证据见 [M1-DRAG-TEST-REPORT.md](docs/M1-DRAG-TEST-REPORT.md)。

## M1 更新（2026-10-13）

用户已评审通过 M0 并授权继续。当前实现仅到 M1，不进入 AI 里程碑。

- 桌面默认使用 `TauriRepository`，浏览器与组件测试仍显式使用内存示例。`ProjectRepository` 保持 UI 调用边界，IPC 返回值通过 Zod 校验。
- SQLite 使用 sqlx 迁移、WAL、外键和单写连接。人工状态、序列投影、备注与 append-only 事件在同一事务中提交；revision 检查拒绝旧窗口覆盖。重启时只迁移应用新建的本地数据库。
- M1 迁移仅建立本地核心表，不提前建立 AI 服务。比较事件支持 `preferred_photo_id=NULL` 表达“不确定”；与原 SQL 草案只允许明确偏好的定义相比，这是执行规范要求的补充。
- 项目删除实现为归档/恢复，保留元数据、人工事件和原图。用户需在 UI 确认归档，没有永久清除入口。
- 文件选择在 Rust 使用系统 Dialog，生成绑定项目的一次性授权 token；前端不能传任意路径给导入，也不能执行 SQL。资源协议只在运行时授权本应用的缓存目录。
- 导入任务先返回 ID，再后台扫描目录；扫描逐项检查取消标记。每张照片独立提交，解码阶段取消在当前解码返回时生效，未提交照片不进入项目，已完成部分保留。失败逐张显示，可以重试；重复判断使用 SHA256，不将相似照片自动删除。
- Rust image 解码 JPEG/PNG/WebP/TIFF 并处理 EXIF orientation；macOS HEIC/RAW 使用固定 `/usr/bin/sips` 作为平台回退，参数不经 shell。已验证 HEIC 与 Canon CR2 sRAW 样本，其他 RAW 按平台支持返回逐张失败。
- 原图不复制到项目、不改写；320px 网格缓存与最长边 1400px 预览仅缩小、不放大小图。解码内存限制 256 MiB，文件大小上限 512 MiB。
- 100 张以上网格采用 TanStack Virtual，密度与检查器收起仍可使用。备注草稿按照片保留在 UI 中，保存后写入 SQLite。
- 导出从持久化快照生成 JSON/CSV/接触印相 HTML；只创建新文件，拒绝覆盖任意已有文件，返回导出 SHA256。HTML 只嵌入小尺寸缓存，CSV 防公式注入。
- 集成测试通过测试专用 stdio Rust harness 驱动真实 Store/ImagePipeline，并以 loopback 桥接 Playwright。该 harness 受 Cargo feature 限制，生产应用不编译或启动测试 HTTP 服务；系统文件选择器在测试中使用固定夹具替代，未据此宣称原生对话框已验收。
- M0 UI 继续保持独立组件；新增服务没有引入云同步、登录、模型、AI Provider 或个人审美推断。

---

日期：2026-10-13（Asia/Shanghai）。当前验收范围：M0，完成后停止等待 UI/UX 评审。

1. **工程目录**：原仓库仅有 Demo，没有既有 React/Rust 工程，也没有 Git 仓库。独立工程放在 `between-frames/`；保留原 Demo 与文档，不创建提交或推送。
2. **阶段优先级**：以执行指令的 M0 定义和用户当前要求为范围。照片导入、SQLite、真实项目存档、导出和生产事件事务属于 M1；AI Provider/MockProvider 属于 M2。AIPanel / PreferencePanel / ExportDialog 在 M0 仅交付可评审 UI，人工事件是本次内存预览记录。
3. **业务与 UI 分离**：`ProjectRepository` 为调用边界，M0 只注入 `MockRepository`。TanStack Query 保持仓库快照，Zustand 仅管理 UI 状态。避免未来更换持久化时重写视觉组件（DIP/SRP）。
4. **布局与构图**：沿用 Demo 的暗房三栏、品牌米金、Georgia/Songti SC 标题与入选底带；照片默认 `object-fit: contain`，不沿用 Demo 静默裁切。比较只记录偏好、不自动保留或淘汰（规格优先）。
5. **组件基建**：Tailwind v4 + shadcn 风格 Button（CVA/clsx/tailwind-merge）+ Radix Dialog 提供焦点陷阱与 Esc。应用排版用语义样式与统一 tokens，避免为 M0 引入整套未使用控件（KISS/YAGNI/DRY）。
6. **路由**：M0 只需要项目列表和单项目编辑，使用 Hash + React `useSyncExternalStore`，支持后退与静态桌面包；无需服务端路由或额外路由框架。
7. **权限**：Tauri 只授权 `core:default`，不启用文件、opener、shell 或 HTTP 插件；生产 CSP 仅允许本地资源与 Tauri IPC。后续导入授权将在 M1 按实际业务添加。
8. **素材引用**：Vite 引用父目录 `assets/` 并在构建时打包；开发服务器显式授权工程和该素材目录。图片保持外部资源，不以内嵌 base64 进入业务数据。
9. **未提前实现**：SQL migrations、图像流水线、模型、个人偏好推断、撤销/重做、序列历史和虚拟列表均留在对应后续阶段。M0 28 张图使用原生 lazy loading。
10. **格式支持冲突**：执行指令 M1 提到 HEIC/RAW，规格书则将它们标为可选平台解码或后续功能。M0 不实现导入；进入 M1 时应根据解码器可用性明确支持范围，不能宣称所有格式均支持。
11. **工具链**：使用官方 Tauri create-tauri-app React/TS 模板，依赖实际版本由 `package-lock.json` 和 `Cargo.lock` 固定。官方参考：[项目初始化](https://v2.tauri.app/start/create-project/)、[最小权限](https://v2.tauri.app/security/capabilities/)、[Vite 配置](https://vite.dev/guide/)。
