# v1.0 M5 文件清单

范围为 Beta 工程准备：Keychain、诊断、性能基线、Beta 旅程与隐私文档。签名/公证/DMG/用户 Beta 不在本轮。

| 文件 | 职责与修改 |
|---|---|
| `between-frames/src-tauri/src/keychain.rs` | macOS 钥匙串读写删（security 命令）；resolve 链 env→keychain；脱敏尾号 |
| `between-frames/src-tauri/src/agent/openai.rs`、`runtime.rs`、`types.rs` | key 解析接入钥匙串；AiConfig 增 keySource |
| `between-frames/src-tauri/src/commands.rs`、`lib.rs`、`bin/m1-harness.rs` | store_api_key / delete_api_key / get_diagnostics 三命令与 harness 分支 |
| `between-frames/src-tauri/tests/keychain.rs` | 钥匙串读写删回环（独立测试 service，macOS） |
| `between-frames/src-tauri/tests/perf_baseline.rs` | 性能基线（200 导入吞吐、决策 p50/p95、投影读取；宽松退化守卫） |
| `between-frames/src/features/ai-editor/AIPanel.tsx` | 模型与密钥配置区（密码框存钥匙串/移除/复制诊断） |
| `between-frames/src/services/TauriAgentRepository.ts`、`domain/agent.ts` | keySource 类型与 zod |
| `between-frames/e2e/beta-journey.spec.ts`、`playwright.integration.config.ts` | Beta 完整旅程（意图→导入→选片→委托→草案→纠正假设→局部采纳→诊断与重启一致） |
| `between-frames/eslint.config.js` | 忽略 Rust 构建目录产物 |
| `docs/PRIVACY.md` | 隐私与数据说明（照片/上传/密钥/导出/诊断/已知边界） |
