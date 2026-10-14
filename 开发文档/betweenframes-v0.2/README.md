# BETWEEN FRAMES / 序间 — v0.2 GUI × Photography Agent 开发交付包

本包是**开发规格与测试契约**，不是一套已经实现的 Tauri 应用。`reference/` 内的 HTML 是上一版可打开的演示（模拟 AI），用于保留视觉与交互风格；生产开发应以 `docs/DEVELOPMENT-SPEC.md` 为准。

## 先读什么

- `docs/DEVELOPMENT-SPEC.md` — 完整产品、UI/UX、Agent/GUI 共用领域层、技术栈、状态机、隐私和里程碑。
- `AGENT-EXECUTION.md` — 可直接喂给编码 Agent 的开发指令。
- `contracts/agent-tool-contracts.json` — 12 个 Agent 工具的 strict 参数契约与权限分级。
- `contracts/command-envelope.schema.json` — 12 种领域命令的接口示例契约。
- `migrations/0001_init.sql` — 初始 SQLite schema、外键与跨项目防护触发器。
- `tests/ACCEPTANCE-CASES.json` — 10 条最关键端到端验收用例。
- `reference/legacy-interactive-demo.html` — 上一版 GUI Demo（非真实模型）。
- `reference/legacy-ui-preview.png` — 上一版工作台视觉参考。

## 最重要的产品决定

传统 GUI 与 Agent 双入口，**同一个 Domain Command Service**；Agent 仅在草案里编辑，不覆盖用户正式作品；用户随时以 GUI 接管、改选、撤销；图像本地优先。

## 文档优先级

`DEVELOPMENT-SPEC v0.2` > `AGENT-EXECUTION v0.2` > v0.2 机器可读 contracts / migration（若出现冲突，需修复契约并记录 ADR） > 旧版 Demo > 旧版 v0.1 文档。

## 已做的验证

本交付包中的 SQLite DDL 已通过 Python SQLite 解析与外键测试；JSON 文件可被解析；**未声称这里已经建成和打包完整 macOS 桌面应用，也未接入真实多模态 Agent**。实现由编码 Agent 按 M0–M5 执行。

## 放进已有工程

不要让编码 Agent 自动清空工程。把本包放在原工程根目录的一个 `planning-v0.2/` 文件夹并优先读取新规格；代码迁移/替换需要查看原仓库后才决定。
