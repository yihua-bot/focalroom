# BETWEEN FRAMES v0.2：给编码 Agent 的实施指令

> 适用于 Codex、Claude Code 等具备文件系统、终端、浏览器/桌面测试工具的编码 Agent。本文件是开发任务，不是让模型直接向最终用户输出代码片段。

## 你的角色与第一优先级

你是工程负责人 + 产品工程师。**开发可安装、可脱网使用的 macOS 摄影创作软件**，不是仅画一个网页 Demo，也不是聊天助手包一层相册界面。保持传统 GUI 的核心价值，同时把 Agent 作为同一工作台可用的受控执行者。

## 必读（按顺序）

1. `README.md`：包含资产引用及运行/交付说明。
2. `docs/DEVELOPMENT-SPEC.md`：v0.2 唯一产品与架构规格。
3. `contracts/agent-tool-contracts.json`、`contracts/command-envelope.schema.json`。
4. `migrations/0001_init.sql`、`tests/ACCEPTANCE-CASES.json`。
5. `reference/legacy-interactive-demo.html`、`reference/legacy-ui-preview.png`：旧视觉、布局与部分交互的参考，**其中 AI 为 Mock，数据结构不再作为依据**。

## 不可违背的工程红线

- **保留 GUI**：Contact Sheet / Inspector / Compare / Sequence / Export 必须脱网可用。禁止把主界面改为聊天页。
- **一个事实来源**：React 不能直接改数据库；GUI 与 Agent 使用同一个 `DomainCommandService`，任何写动作必须能被事件日志追踪。
- **Agent 没有正式写权限**：Tool Registry 仅允许只读、草案写、请求反馈及完成任务。`ApplyDraft` 只能由用户确认触发，不能注册模型工具。
- **默认不上传图片**：用户明确批准前，无法对任何外部视觉模型发图；不传 GPS、原始路径、原始 TIFF/RAW。
- **不触碰原图**：禁止移动、覆盖、裁切写回、删除源文件。只在 App 缓存里生成缩略图/预览。
- **真实结果与 Mock 区分**：MockProvider 生成的结果必须标注模拟；没有真实 AI Key 时所有 GUI 和 Mock 测试仍要成功。
- **正确处理冲突**：模型开始后人类可以继续编辑。草案 base revision 过时必须被阻止应用。
- **禁止虚报测试**：如果没有 macOS 工具链或真实模型 Key，明确指出哪些测试没法完成，仍交付已经实现且可验证的部分。

## 提交顺序

### 第一轮只做 M0

1. 检查当前代码和资产，不清空原项目。不确定处写 `docs/DECISIONS.md`。
2. 初始化 Tauri 2 + React + TypeScript + Vite + Tailwind CSS；保留可复用的 UI primitives。
3. 实现稳定的 AppShell、ProjectList、ContactSheetGrid、Inspector、Lightbox、Compare、SequenceBoard、AgentPanel 布局。
4. 实现 `DomainCommand` TypeScript/Rust DTO 与 `execute_gui_command` MockRepository 入口。不要在按钮回调直接变更正式领域数据。
5. 统一暗房视觉 tokens、缩略图比例、选中和 keep/reject 的视觉区分。AI 面板可折叠，默认不抢中心区域。
6. 安装基础自动化测试，运行可执行的 lint/typecheck/unit/UI smoke，提交真实截图（1920×1080 + 1280×800）。
7. 更新 `docs/TEST-REPORT.md` 和 `docs/KNOWN-ISSUES.md`，**停止并请求人类验收 M0**。不要擅自推进到 M1。

### 获得 M0 验收后：执行 M1

实现真实 SQLite 及 migrations、只读图库导入、缓存、GUI keep/reject/undecided、双图偏好、序列排序、版本、事件与 undo、CSV/JSON/HTML 导出。让 200 张真实 JPEG 在**无网络**状态下完成完整任务；重启后状态不丢失。

### M1 验收后：执行 M2

实现 Agent Runtime 状态机、Tool Registry/Policy Gate、工具调用审计、Mock Agent 调用实际后端工具创建两份可编辑草案；实现 GUI 草案审核、用户批准、部分采纳、撤销、过时冲突。保证 Agent 工具中无 `ApplyDraft`。

### M2 验收后：执行 M3

加入真实视觉 Provider Adapter、函数调用循环、受控预览上传、授权、并发和费用预算；从 100–300 张照片产生可审阅草案。模型工具调用结果必须通过后端真实 schema/权限验证。

### M3 验收后：执行 M4/M5

审美证据与跨项目偏好检索，纠正、删除、来源追踪，最后做打包、安全、性能与摄影师实际使用测试。允许收窄功能，但不得牺牲数据安全和一致性。

## 开发执行策略

- 每个里程碑最多拆成 5–10 个明确任务，先写代码再测试，避免一直写规划。
- 前后端 DTO 保持一种权威定义（Rust serde DTO + TS 导出或契约测试）。
- 后端 `CommandService` 在 SQLite transaction 中写实体与 append-only 事件；所有调用都有 `commandId` 保证幂等。
- Agent Runtime 统一在 Rust 侧；前端只展示 task/steps、发起/取消/反馈任务，不允许 Agent 通过前端“偷偷点击按钮”。
- `MockProvider` 真的调用 ToolRegistry 修改草案表，绝非虚拟动画。
- 每个 `AgentTask` 持久化 `base_project_revision`，用户编辑产生新 revision 时草案标记 stale。
- 如果发现 spec 自身缺口，优先选择更安全、更小的可测试实现，在 ADR 写出假设和权衡，不要直接扩大范围。
- 真实测试必须有按错误码分类（`REVISION_CONFLICT`, `PERMISSION_DENIED`, `INVALID_PHOTO_ID`, `USER_CONSENT_REQUIRED`, `TOOL_BUDGET_EXCEEDED`）。

## 代码质量门槛

尽可能在目标环境运行：

```bash
pnpm typecheck
pnpm lint
pnpm test
pnpm test:e2e
pnpm tauri build
cd apps/desktop/src-tauri && cargo test
```

若新工程采用单包目录，则使用 README 声明的等价命令。不能因当前容器不是 macOS 就跳过所有可执行的 Web 和 Rust 测试。

## 每个里程碑回报格式

| 任务 ID | 完成内容 | 验证命令/交互 | PASS / FAIL / NOT RUN | 风险 |
|---|---|---|---|---|

然后附：项目目录树、代码改动摘要、截图路径、真实测试输出、遗留缺陷、对照 `tests/ACCEPTANCE-CASES.json` 的覆盖情况、需要用户验收的问题。对未完成的工作明确标注，不得把模拟实现说成真实 AI 结果。

## 立即开始

**现在只执行 M0**：在不覆盖旧 Demo 的前提下建立生产工程骨架，并实现兼容原 GUI 交互的可运行工作台。Agent 面板留足入口，但不要在 M0 偷做真实模型服务。完成 M0 的构建与测试后停止等验收。
