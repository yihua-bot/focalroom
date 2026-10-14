# v1.0 M4 测试与验收（历史归档）

日期：2026-10-14（Asia/Shanghai）。M4 已完成实现与自动化验收：L2 人工偏好证据（因果去重）、L3 可纠正假设（确定性物化、纠正不复活）、retrieve_preferences 证据化检索与上下文注入、Memory Studio（证据时间线/假设卡/纠正/停用）。M5（Keychain、签名公证、性能与 Beta）未启动。

M3 报告归档于 [V1-M3-TEST-REPORT.md](V1-M3-TEST-REPORT.md)；M3 的真实模型 E2E 仍待 API Key 补验。产品基线 v1.0，工程包版本 0.1.0。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| L2 证据采集 | keep/reject/比较/备注/草案采纳与丢弃六类人工命令记录；dedup_key=command_id | agent_memory：幂等重放/撤销/重做证据计数不变 | passed | 证据不提供删除（append-only 审计） |
| Agent 不产生证据 | 草案工具在 agent 通道，结构性排除 | agent_memory：全流程跑完两份草案证据为 0；人工采纳后 1 条 | passed | — |
| L3 假设物化 | 证据文本 2-gram 聚合 ≥2 条支持生成 tentative；默认 project_local | agent_memory：「留白」「逆光」场景确定性生成且含证据 ID | passed | 规则可解释但不做语义理解 |
| AG-11 纠正不复活 | 纠正→corrected+停用检索；同表述不重建 | agent_memory：纠正后检索不含、再物化无新行 | passed | — |
| 检索与注入 | retrieve_preferences 返回假设+证据 ID 绑定；ContextBuilder 限量 3 条 | agent_memory：脚本任务两轮工具断言（成功+跨项目白名单拒绝） | passed | FTS5 未引入（基数小，ADR 0007） |
| Memory Studio | 假设卡（证据链接/反例/纠正/停用恢复）、证据时间线、边界文案 | 组件编译回归 + 集成 E2E | passed | 专项组件测试未补（面板为只读+两动作） |
| M0–M3 回归 | 全部既有能力保留 | 41 Rust + 24 单测 + 16 预览 + 2 集成 | passed | — |

## 自动化结果（2026-10-14 17:30 全量复验）

| 检查 | 结果 |
|---|---|
| cargo test（target-v1） | passed，41 tests：agent_runtime 13、agent_vision 4、agent_memory 4、command_history 10、local_core 10 |
| cargo fmt -- --check / 严格 Clippy（含 integration-harness） | passed，0 warnings |
| npm run typecheck / npm run lint | passed，0 warnings |
| npm run test | passed，6 files / 24 tests |
| npm run test:e2e | passed，16 tests |
| CARGO_TARGET_DIR="target-v1" npm run test:integration | passed，2 tests（agent-core 54.4s、local-core 11.3s） |

过程记录：迁移计数断言随 0006 更新为 6；gui 测试辅助改为执行后回读权威 revision（原 receipt 快照 revision 滞后一步，非生产缺陷）；幂等重放断言改为证据计数语义（重放返回首次回执快照属既定行为）。

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

假设为关键词规则推断，不做语义理解（statement 明示证据来源）；跨项目假设与 opt-in 未实现；FTS5 未引入；证据删除入口未提供（ADR 0007 说明）；Memory Studio 专项组件测试待补；M3 遗留（真实模型 E2E、Keychain、签名、性能）继续有效。

本轮停止 M4，等待评审。M5 为 Beta 质量与发布准备。见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M4-FILES.md)、[ADR 0007](ADR/0007-v1-m4-preference-memory.md)、[实施计划](PLAN-V1-M4.md)。
