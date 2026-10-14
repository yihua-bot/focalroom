# BETWEEN FRAMES v1.0 · FINAL AGENT IMPLEMENTATION KIT

这是面向实际编码 Agent 的**最终实施基线**，并不是已完成的桌面应用；参考 HTML 的 AI 功能是演示逻辑。

## 最先阅读

- `docs/FINAL-IMPLEMENTATION-SPEC.md` — 完整产品、UI/UX、前后端、工具循环、数据、记忆、隐私、测试、里程碑。
- `AGENT-EXECUTION.md` — 可直接交给编码 Agent 的主指令。
- `contracts/` — 旧版 12 Tool contracts、领域命令协议与新增 task envelope。
- `migrations/` — 完整初始数据库与新的运行时强化迁移。
- `tests/AGENT-ACCEPTANCE-MATRIX.json` — 15 个必须证明是真正 Agent 的验收场景。
- `reference/` — 传统 GUI Demo 与设计预览，只作外观与交互参考。

## 最重要的实施结论

1. GUI 完全独立，Agent 深度嵌入。
2. 一个 Rust Domain Command Bus 与 SQLite 权威状态，写入统一审计。
3. Agent 采用单 Agent + 动态 Tool Calling Loop + ModelProvider 抽象 + 受限 Tool Registry。
4. Agent 只能操作 Draft，正式 ApplyDraft 始终需要用户明确审批。
5. 检索照片和审美记忆有证据，按权限及上下文限量提供模型。
6. 重试、恢复、预算、上传权限全部由应用后端执行。
7. 先做 M0/M1 GUI，再做 M2 Mock Agent Runtime，再 M3 真实视觉工具，再 M4 审美记忆。

## 自检

```sh
python3 tests/verify_kit.py
```
