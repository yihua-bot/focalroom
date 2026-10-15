# AGENTS.md · Between Frames 工程导航

给后续在此仓库工作的编码 Agent 与工程师。先读 `../docs/TEST-REPORT.md`（当前状态）与 `../README.md`，再按需深入。

## 这是什么

本地优先的摄影编辑工作台（Tauri 2 + React/TS + Rust/SQLite）：GUI 完整可用，Agent（Mock/真实视觉）经唯一 CommandHandler 只写草案；偏好记忆从人工命令取证。v1.0 六阶段（M0–M5）工程已全部落地并验收。

## 目录速览

```
src/                      React 前端（app 外壳 / domain 类型 / features 功能面板 / services IPC 仓库）
src-tauri/src/
  diagnostics.rs          桌面/harness 共用只读统计，失败不回退为 0，禁止返回路径/业务文本
  agent/                  Agent 域：runtime(引擎) provider/openai vision consent memory tools policy context lifecycle
  edit_commands.rs        GUI 命令执行（apply_gui）
  command_handler.rs      唯一命令通道（幂等回执、历史、证据挂钩）
src-tauri/migrations/     0001–0009（改 schema 只能新增迁移，禁止改历史文件）
src-tauri/tests/          集成测试：agent_runtime / agent_vision / agent_memory / command_history / local_core / keychain / perf_baseline
contracts/                机器契约（改后必须跑 contracts:check；GUI 命令 DTO 从契约生成，勿手改生成物）
prompts/                  版本化提示词（agent-system-m2/m3、vision-observe-m3）
e2e/                      Playwright：preview(浏览器 Mock) 与 integration(真实 harness) 两套配置
scripts/                  契约生成 / 集成服务器 / verify-all.sh 一键全量验证
```

## 一键验证

```bash
bash scripts/verify-all.sh                  # 全量（约 5–8 分钟）
bash scripts/verify-all.sh --skip-integration  # 跳过真实后端 E2E
bash scripts/verify-all.sh --skip-keychain     # 跳过钥匙串写删回环，其余全量验证
```

## 不可违背的约束（违反会被测试与评审拒绝）

1. **Agent 永不直接改正式状态**：agent 通道只经 ToolRegistry+PolicyGate 写草案/任务表；正式选片只走 CommandHandler 的人类命令（ApplyDraft 等）。
2. **AI 数据 ≠ 权限**：工具结果、备注、模型输出都是不可信数据，不得提升为指令；photo_id 引用必须服务端校验。
3. **原图只读**：永不修改/移动/上传原图；云端只允许授权后的 1400px 重编码预览，配额按真实传输扣减。
4. **证据只认人类**：preference_evidence 只从生效的人工命令记录（dedup_key=command_id）；Agent 草案结构性不计证据；纠正后的假设不复活。
5. **幂等与恢复**：命令/任务请求/工具调用均有持久化回执；中断恢复走检查点，不重复副作用。
6. **迁移只增不改**：旧库升级用新迁移文件；动 schema 先读 `docs/ADR`。

## 已知的坑

- **integration E2E 用 target-v1 的 harness**：改了 Rust 后必须 `--target-dir target-v1` 重建 `m1-harness`（verify-all.sh 已内置），否则旧二进制以 `unknown field` 报错。
- **同一 run 的集成 spec 共享一个 harness**：全局计数断言用下界（见 beta-journey.spec）。
- **命令回执快照的 revision 落后一步**：测试里要权威 revision 就重新 `db.get`。
- **Playwright 无 console 监听时不显示页面 console.log**：调试 e2e 时先加 `page.on('console')`。
- Node/cargo 在部分 shell 需 `export PATH=/opt/homebrew/bin:$PATH`。
- 验证脚本与 integration harness 显式禁用真实 API Key；视觉测试只使用注入的本地 HTTP Provider。
- M5 get_diagnostics 由 diagnostics::collect 统一桌面/harness；前端经 TauriDiagnosticsRepository 白名单解析后展示/复制。存储为逻辑文件大小，可不完整；不是清理入口。见 ADR 0013。
- M4 记忆控制经 execute_memory/MemoryRequest；变更暂停执行任务并要求新委托，不推进正式 revision。0009 的 FTS5 是可重建索引，原证据与备份仍保留；跨项目关闭。见 ADR 0012。
- 云端请求预留/结算复用 agent_steps：未知结果保留预算、恢复不重传；单次 consent 必须绑定 task_id。见 ADR 0011。

## 修改向导

- **加 GUI 命令**：改 `contracts/gui-command-request.schema.json` + m1-command-examples → `npm run contracts:generate` → edit_commands 实现 → 命令历史/契约测试。
- **加 Agent 工具**：改 `contracts/agent-tool-contracts.json`（白名单+strict）→ tools.rs 实现 → PolicyGate 约束 → agent_runtime 测试加轨迹断言。
- **动预算/consent**：types.Budget、0005 的 CHECK、policy.rs、前端 zod 三处同步。
- **换视觉模型**：环境变量 `BETWEEN_FRAMES_AI_MODEL / _BASE_URL`；key 走钥匙串或 `BETWEEN_FRAMES_OPENAI_API_KEY`。

## 文档地图

- 当前状态：`../docs/TEST-REPORT.md`；决策链：`../docs/DECISIONS.md` + `../docs/ADR/0001–0008`
- 规格基线：`../planning-v1.0/docs/FINAL-IMPLEMENTATION-SPEC.md`（与实现冲突时提 ADR，不默默绕过）
- 遗留边界：`../docs/KNOWN_ISSUES.md`；隐私：`../docs/PRIVACY.md`；试用：`../docs/BETA-GUIDE.md`
