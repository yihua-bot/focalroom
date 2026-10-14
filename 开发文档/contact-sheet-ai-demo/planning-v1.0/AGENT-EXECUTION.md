# Between Frames v1.0 — 给编码 Agent 的主执行指令

> 把本文件连同 `docs/FINAL-IMPLEMENTATION-SPEC.md` 放入仓库；编码 Agent 必须遵循开发基线而不是仅依据旧版 HTML Demo。

## 任务目标

实现一个 macOS 本地优先的 GUI-first、Agent-powered 摄影项目工作空间。GUI 不依赖 AI 即可完成照片导入、筛选、比较、作品排序、撤销、版本与导出；AI 是可调用工具、能自主多步执行、观察反馈并产生候选草案的 PhotographyEditorAgent。**严禁用固定 Pipeline 伪装 Agent**。

## 必须先读

1. `docs/FINAL-IMPLEMENTATION-SPEC.md`（优先级最高）；
2. `contracts/`、`migrations/`、`tests/`；
3. `reference/legacy-demo.html`（只做 GUI 参考，不继承模拟 AI 业务逻辑）。

## 强制施工顺序

- **M0**：GUI + 本地导入 + Contact Sheet + Compare + Sequence，离线可用。
- **M1**：Rust 统一 Domain Command、SQLite 事务、事件日志、版本、撤销、导出。
- **M2**：Tool Registry、PolicyGate、Agent Task Engine、MockProvider、动态工具循环、草案审批和恢复契约。
- **M3**：真实多模态工具调用、Provider adapter、图片授权与隐私保护、任务成本。
- **M4**：审美证据/假设/纠正、项目与跨项目记忆；证据检索。
- **M5**：性能、安装打包、质量门禁、用户 Beta 验证。

每阶段先写一个简短 PLAN（文件、主要改动、测试）；实施完成必须运行真实命令并汇报：修改文件清单、测试输出、可复现步骤、截图、阻塞问题、ADR。完成本阶段后停下等待产品负责人确认。

## 真 Agent 最小验收（M2/M3）

1. 模型支持动态 tool calls，不是固定工具调用顺序。
2. 实现至少两次“模型→工具→工具结果→模型重决策”的闭环；必要时更多轮。
3. 工具执行只有受限摄影工具、强 schema、Project Scope、独立 PolicyGate。
4. 按证据动态改变下一步操作，测试提供两组不同观察，确认轨迹发生有意义变化。
5. Agent 仅可写 Draft；正式序列只由 GUI 用户批准 ApplyDraft。
6. 已执行工具调用有 call_id/幂等记录，可安全取消、暂停、重启、回放。
7. 未授权不上传照片，任何安全限制不依赖提示词来保证。
8. 离线 GUI 不受 Provider 故障影响。

## 不得为通过测试做的事

- 不得在 UI 里硬编码 AI 建议，却显示为真实 LLM。
- 不得把 MockProvider 的预设调用轨迹当作真实 Provider 的自主能力。
- 不得绕过领域服务直接更新 SQLite 业务表。
- 不得为了功能方便给 Agent 任意 shell、SQL、File System 或网络读取能力。
- 不得把用户图片、模型密钥、个人路径放入遥测日志。
- 不得将 Agent 草案静默直接设置为正式作品序列。

## 首轮实际开发指令

```
审计仓库现状；复述当前模块与 v1.0 契约差距；仅完成 M0。
使用 Tauri 2 + React + TypeScript，保留旧 GUI 的专业摄影编辑习惯。
实现本地照片导入、缩略图、Contact Sheet、选片、双图比较、序列排序、快捷键、重启后恢复。
建立为 M1 准备的类型化 IPC 与 Domain Command 接口，但暂不调用真实模型。
完成后运行可执行测试，提供界面截图与阻塞项；停止并请求验收。
```
