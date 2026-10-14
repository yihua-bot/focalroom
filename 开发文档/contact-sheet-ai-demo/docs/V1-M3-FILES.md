# v1.0 M3 文件清单

范围为真实视觉 Provider、图像授权、观察缓存与预算放开。工程路径相对仓库根目录。M4 记忆假设/FTS5、M5 Keychain/签名不在本轮。

| 文件 | 职责与修改 |
|---|---|
| `between-frames/src-tauri/Cargo.toml` | 新增 reqwest(json, rustls-tls)，无默认 TLS |
| `between-frames/src-tauri/migrations/0005_vision_consent.sql` | privacy_consents、photo_observations、consent_receipts；model_usage 重建去 =0 CHECK；agent_tasks 增 provider_id/consent_id |
| `between-frames/src-tauri/src/agent/types.rs` | Budget 放开云端图像 0–100；Task/TaskView 增 provider 维度；AiConfig/ConsentView/ConsentRequest DTO |
| `between-frames/src-tauri/src/agent/openai.rs` | OpenAIProvider：Responses API tool calling、store:false 本地协议重发、视觉观察子请求与严格 JSON schema、保守费用估算 |
| `between-frames/src-tauri/src/agent/consent.rs` | 授权生效/配额扣减/幂等设置/撤回；传输即扣费语义 |
| `between-frames/src-tauri/src/agent/vision.rs` | 视觉预取：scope→缓存→授权→上传；幻觉 photo_id 丢弃；BUDGET_EXCEEDED 上抛暂停 |
| `between-frames/src-tauri/src/agent/tools.rs` | inspect/compare 双模式（vision_observed / mock_metadata_only）；观察落缓存与配额同事务 |
| `between-frames/src-tauri/src/agent/runtime.rs` | 双 provider 注入与按任务分流；视觉预取接线；图像预算触顶转 paused |
| `between-frames/src-tauri/src/agent/lifecycle.rs` | StartTask 由环境决定 provider、consent 绑定与归属校验 |
| `between-frames/src-tauri/src/agent/context.rs` | checkpoint 按 provider 校验；system prompt/预算策略按 provider 分层 |
| `between-frames/src-tauri/src/agent/store.rs`、`execution.rs`、`mod.rs` | 新列持久化、模型步骤按 provider 记录、模块导出 |
| `between-frames/src-tauri/src/commands.rs`、`lib.rs` | get_ai_config / set_privacy_consent IPC 注册 |
| `between-frames/src-tauri/src/bin/m1-harness.rs` | 新 RPC 分支（配置查询/授权设置） |
| `between-frames/contracts/agent-task-request.schema.json` | 预算图像上限 0–100；StartTask 增可选 consentId |
| `between-frames/contracts/agent-tool-contracts.json` | inspect/compare 描述升级为 M3 授权语义 |
| `between-frames/prompts/agent-system-m3.md`、`vision-observe-m3.md` | 版本化系统约束与视觉观察提示词 |
| `between-frames/src/domain/agent.ts`、`services/TauriAgentRepository.ts` | provider/consent 类型与 zod；aiConfig/setConsent 方法 |
| `between-frames/src/features/ai-editor/AIPanel.tsx` | 授权对话框（本次任务/本项目/禁止）、图像与费用预算字段、provider 状态与用量显示 |
| `between-frames/src/features/ai-editor/AIPanel.test.tsx` | 授权流三项组件测试 |
| `between-frames/src-tauri/tests/agent_vision.rs` | 本地 HTTP 模拟 Responses API：AG-04 零外发、授权上传+缓存零重传、幻觉扣配额、预算触顶 |
| `between-frames/src-tauri/tests/agent_runtime.rs`、`local_core.rs` | 既有测试适配（consent 字段、迁移计数 5） |
