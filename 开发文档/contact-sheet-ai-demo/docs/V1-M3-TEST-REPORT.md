# v1.0 M3 测试与验收（历史归档）

日期：2026-10-14（Asia/Shanghai）。M3 已完成实现与自动化验收：OpenAIProvider（Responses API tool calling）、图像 consent 授权链路、预览上传与视觉观察缓存、inspect/compare 双模式、图像/费用预算放开与 provider 分流。**真实模型 E2E 未验证**：本轮无 API Key，云端闭环以本地 HTTP 模拟 Responses API 验证协议行为；提供 `BETWEEN_FRAMES_OPENAI_API_KEY` 后需按复现命令补跑并另行报告。M4 记忆引擎未启动。

M2 报告归档于 [V1-M2-TEST-REPORT.md](V1-M2-TEST-REPORT.md)。产品基线 v1.0，工程包版本 0.1.0。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| OpenAIProvider | Responses API function calling；strict schema 复用 ToolRegistry；store:false 本地协议保存重发；5xx/429 可重试分类 | 协议构造/解析在集成测试中经本地 HTTP 模拟全链验证 | passed | 真实服务 E2E 待 key |
| AG-04 无授权零外发 | 无 consent 时视觉工具降级 `mock_metadata_only` 并在结果/步骤明示 | agent_vision：模拟服务断言图像请求 0、主循环正常完成 | passed | 降级结论须标注未视觉验证 |
| 授权与计量 | task/project/deny 三态、幂等设置、新授权撤旧；配额按真实传输扣减 | agent_vision：授权后上传计数、consent.used、费用入账 | passed | 费用为保守估算非账单 |
| 观察缓存 | photo_hash+model+prompt+kind 键；命中零上传零扣费 | agent_vision：二次任务缓存命中、图像请求数不变 | passed | 缓存无 TTL，同图同问永久命中 |
| 幻觉防护 | 观察 photo_id 必须属请求集合；虚构整体丢弃 | agent_vision：全幻觉→任务 failed、0 观察落库、配额仍按传输扣 | passed | 部分幻觉保留有效部分 |
| 图像预算触顶 | 上传前检查 budget.max_cloud_images 与费用余量；触顶 paused | agent_vision：预算 1 张请求 2 张→paused、零上传 | passed | — |
| provider 分流 | 有 key→openai 否则 mock；checkpoint 校验 providerId/promptVersion | agent_vision 四场景 + agent_runtime 13 回归 | passed | 无 key 时任务回落 Mock 并明示 |
| GUI 授权 | 授权卡（数量/尺寸/费用/目的/保存策略）、三粒度选择、预算图像/费用字段、用量显示 | AIPanel 三项组件测试 + 集成 E2E 全旅程 | passed | 授权卡在无 key 环境不触发 |
| M0–M2 回归 | 命令历史、本地核心、Mock Runtime、Draft 审批全部保留 | Rust 33 回归 + 预览 16 + 集成 2 全绿 | passed | — |

## 自动化结果（2026-10-14 16:50 全量复验）

| 检查 | 结果 |
|---|---|
| cargo test（target-v1） | passed，37 tests：agent_runtime 13、agent_vision 4、command_history 10、local_core 10 |
| cargo fmt -- --check | passed |
| cargo clippy --all-targets --features integration-harness -- -D warnings | passed，0 warnings |
| npm run typecheck / npm run lint | passed，0 warnings |
| npm run test | passed，6 files / 24 tests（含 AIPanel 授权 3 项） |
| npm run contracts:check | passed（M1 GUI 契约；agent 契约由运行时校验） |
| npm run test:e2e | passed，16 tests |
| CARGO_TARGET_DIR="target-v1" npm run test:integration | passed，2 tests（agent-core 52.5s、local-core 10.6s） |

过程记录：本轮集成测试一次失败，根因是 `src-tauri/target-v1/debug/m1-harness` 残留 M2 旧二进制（不识别 consentId/get_ai_config）；按 M2 报告复现命令以 `--target-dir target-v1` 重建 harness 后全绿。诊断期间确认前端请求载荷与 Rust 校验行为一致，无代码缺陷。

## 真实模型接入说明（未验证项）

设置 `BETWEEN_FRAMES_OPENAI_API_KEY`（可选 `BETWEEN_FRAMES_AI_BASE_URL`、`BETWEEN_FRAMES_AI_MODEL`）后：GUI 显示真实模型与脱敏尾号；视觉任务弹出授权卡；授权后上传最长边 1400px 重编码 JPEG（无 EXIF/GPS）。**本轮未执行**：真实服务的工具次序多样性（AG-01/02/03 真实样本）、真实 token/费用与账单差异、供应商数据政策核对。这些在补跑前不得宣称通过。

## 复现命令

~~~bash
npm run typecheck && npm run lint && npm test && npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1"
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
~~~

## 剩余问题与停止点

API key 环境变量非 Keychain（ADR 0006 记录，M5 完善）；观察缓存无失效策略；`contracts:check` 未覆盖 agent 契约文件；授权 UI 未在真实 key 环境走通（组件级已测）；M2 遗留项（原生首拖偶发、1400px 预览上限、高分辨率性能、签名公证）继续有效。

本轮停止 M3，等待评审与真实 key 补验。M4 交付偏好证据/假设与 Memory Studio。见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M3-FILES.md)、[ADR 0006](ADR/0006-v1-m3-vision-consent.md)、[实施计划](PLAN-V1-M3.md)。
