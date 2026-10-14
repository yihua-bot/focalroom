# v1.0 M2 文件清单

范围为本地 Mock Runtime、任务恢复、预算、草案审批与 ContextBuilder。工程路径相对仓库根目录。M3 真实 Provider/视觉/云端 consent、M4 记忆假设/FTS5 不在本轮。

| 文件 | 职责与修改 |
|---|---|
| `between-frames/contracts/agent-tool-contracts.json` | 12 个严格摄影工具；以 items 修订旧草案契约；M2 视觉工具明确仅对照元数据 |
| `between-frames/contracts/agent-task-request.schema.json` | 六类生命周期请求、必填预算、无客户端自报权限 |
| `between-frames/contracts/gui-command-request.schema.json`、`domain-envelope.schema.json` | 增加 ApplyDraft/RebaseDraft/UpdateDraft/DiscardDraft |
| `between-frames/contracts/m1-command-examples.json` | 从 14 扩为 18 个 GUI 命令示例 |
| `between-frames/contracts/app-events.json` | 增加合法 Tauri 任务/草案事件映射 |
| `between-frames/src/domain/commands.generated.ts`、`src-tauri/src/commands_generated.rs` | 重新生成 DTO/事件常量 |
| `between-frames/prompts/agent-system-m2.md` | 版本化 Mock 系统约束，不宣称真实视觉理解 |
| `between-frames/src-tauri/migrations/0004_mock_runtime.sql` | 九张 M2 表、唯一 call/receipt、追加步骤、任务/草案归属触发器 |
| `between-frames/src-tauri/src/agent/types.rs` | Task、Budget、Usage、ToolCall/Result、ModelOutcome、Draft/投影 DTO |
| `between-frames/src-tauri/src/agent/policy.rs` | 工具白名单、工程 schema 子集校验、独立可信 scope/预算 Gate |
| `between-frames/src-tauri/src/agent/lifecycle.rs` | 生命周期请求幂等、合法 FSM、人工反馈、启动中断标记 |
| `between-frames/src-tauri/src/agent/store.rs` | 权威任务加载、步骤/检查点、投影、过期标记；写入仅由 Handler 调用 |
| `between-frames/src-tauri/src/agent/context.rs` | 从安全检查点加载关联协议，scope/版本检查，分层上下文与保守计量 |
| `between-frames/src-tauri/src/agent/provider.rs` | ModelProvider Trait、根据实际工具结果重决策的本地 Mock |
| `between-frames/src-tauri/src/agent/runtime.rs` | 后台动态闭环、超时/有限重试、暂停/取消边界与调度竞态处理 |
| `between-frames/src-tauri/src/agent/execution.rs` | 模型尝试计量、outcome 先落库、工具效果/回执/检查点单事务 |
| `between-frames/src-tauri/src/agent/tools.rs` | 项目内分页/搜索、人工事实对照、草案生成/修订/核验/反馈/完成 |
| `between-frames/src-tauri/src/agent/drafts.rs` | 人工局部/全量采纳、rebase、排序/丢弃，拒绝过期或仍执行中的草案 |
| `between-frames/src-tauri/src/agent/mod.rs` | CommandHandler 的任务/受控工具入口及执行版本常量 |
| `between-frames/src-tauri/src/command_handler.rs`、`edit_commands.rs`、`command_audit.rs` | 人工审批沿用 M1 审计/回执/撤销事务；正式变化使旧任务过期 |
| `between-frames/src-tauri/src/db.rs` | M1→M2 一致性备份；保留旧 migration checksum |
| `between-frames/src-tauri/src/commands.rs`、`lib.rs` | 两个类型化 IPC、低敏事件、后台 Engine、启动恢复 |
| `between-frames/src-tauri/src/bin/m1-harness.rs` | 测试专用 M2 IPC/恢复桥；保留历史名称，不进正常产物 |
| `between-frames/src/domain/agent.ts`、`models.ts` | 前端任务/草案接口和预算默认值 |
| `between-frames/src/services/TauriAgentRepository.ts`、`TauriRepository.ts` | Zod 投影/回执校验，同 request ID 重试，人工草案命令 |
| `between-frames/src/features/ai-editor/AIPanel.tsx` | 显式发起、目标/预算、状态/步骤、暂停/恢复/取消/反馈/审核 |
| `between-frames/src/features/ai-editor/DraftReview.tsx` | 照片条目、全量影响、勾选局部采纳、排序、rebase 与明确替换确认 |
| `between-frames/src/features/ai-editor/AgentStatus.tsx` | 隐藏面板后仍可访问实际任务状态，不启动模型 |
| `between-frames/src/app/AppController.tsx`、`copy.ts`、`src/features/project/ProjectList.tsx`、`src/styles/app.css` | 挂载入口/投影刷新、真实 Mock 标签、视觉样式 |
| `between-frames/tauri.m2-review.conf.json` | 新 M2 Review identifier，避免升级旧用户/Review 数据库 |
| `between-frames/src-tauri/tests/agent_runtime.rs` | 12 个 M2 领域/安全/恢复场景，含 200→两版 25 张 |
| `between-frames/src-tauri/tests/command_history.rs`、`local_core.rs` | 18 类 GUI 契约及四个 migration 的旧库回归 |
| `between-frames/src/services/TauriAgentRepository.test.ts`、`src/test/App.test.tsx` | 生命周期重试/冲突/错误投影和浏览器边界 |
| `between-frames/e2e/agent-core.spec.ts` | 真实 Rust/SQLite M2 GUI、预算恢复、审批/撤销/重启与只读 hash |
| `between-frames/e2e/local-core.spec.ts`、`workspace.spec.ts` | 多项目测试归档定位与现有 GUI 回归 |
| `between-frames/playwright.config.ts`、`playwright.integration.config.ts` | 明确分离预览与真实后端旅程 |
| `docs/PLAN-V1-M2.md`、`docs/ADR/0005-v1-m2-mock-runtime.md` | 实施计划和契约/恢复决定 |
| `docs/TEST-REPORT.md`、`docs/KNOWN_ISSUES.md`、两份 README、`DECISIONS.md` | 当前状态、复现、限制；M1 报告另存历史 |
| `docs/V1-M2-NATIVE-*.json`、`docs/screenshots/v1-m2/`、`docs/screenshots/v1-m2-native/` | 实际中断、采纳、完整重启与 hash/截图证据 |

SRP：类型、策略、生命周期、事务执行、存储投影、上下文、Provider、编排与草案编辑各自负责单一边界。DIP：Runtime 依赖 ModelProvider，GUI 依赖 Repository；正式审批仍走 M1 Handler。DRY：复用命令生成、历史补偿、UI tokens 和图像引用；上下文只移除重复的工具参数副本，保留 call/result linkage。KISS/YAGNI：不加 SDK/网络工具/记忆空模块或新依赖，单 Agent 串行草案写入，按需新增九表。

没有执行 Git commit/push、删除源文件、全局配置修改或真实模型请求。原始需求包和 0001–0003 migration 保留。浏览器内存预览明确提示桌面运行，不硬编码候选冒充 Agent。
