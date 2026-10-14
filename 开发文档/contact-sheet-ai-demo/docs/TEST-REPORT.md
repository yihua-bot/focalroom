# v1.0 M5 测试与验收（Beta 工程准备）

日期：2026-10-14（Asia/Shanghai）。M5 工程部分已完成并验收：Keychain 密钥存储、诊断命令、性能基线（Release）、Beta 完整旅程 E2E、隐私文档。**发布流程项未完成**：签名/公证（无开发者证书）、DMG 打包、20 位真实用户 Beta 与真实摄影师试用——这些需要证书与真人，工程侧无法替代，留待发布执行。

M4 报告归档于 [V1-M4-TEST-REPORT.md](V1-M4-TEST-REPORT.md)；M3 真实模型 E2E 仍待 API Key 补验。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| Keychain 密钥 | store/load/delete + env→钥匙串解析链 + GUI 存入/移除 | keychain 2 项回环测试（独立 service）+ 旅程 | passed | 写入瞬间进程参数可见（PRIVACY 声明）；非 macOS 未集成 |
| 诊断工具 | get_diagnostics（版本/计数/配置，脱敏声明）+ GUI 复制 | Beta 旅程断言诊断内容 | passed | 缓存体积统计未含 |
| 性能基线 | Release：导入 2.9s/200 张（70 张/秒）、决策 p95 23.0ms、投影 3.4ms | perf_baseline（--ignored 手动运行，退化守卫） | passed | 数字为本机（Apple Silicon），非跨硬件承诺 |
| Beta 旅程 | 意图→导入 200→选片+备注→Mock 委托→双草案→假设纠正（AG-11 旅程级）→局部采纳→诊断→重启一致 | beta-journey E2E（真实后端 52.2s） | passed | 真实模型版本待 key |
| 隐私文档 | PRIVACY.md（照片/上传/密钥/导出/诊断/边界） | 文档评审 | — | 证据删除语义待产品决定 |
| M0–M4 回归 | 全部能力保留 | 43 Rust + 24 单测 + 16 预览 + 3 集成 | passed | — |

## 自动化结果（2026-10-14 18:30 全量复验）

| 检查 | 结果 |
|---|---|
| cargo test（target-v1，debug） | passed，43 tests（agent_runtime 13、agent_vision 4、agent_memory 4、command_history 10、local_core 10、keychain 2） |
| cargo test --release --test perf_baseline | passed（数字见上，--ignored 手动运行） |
| cargo fmt / 严格 Clippy（含 integration-harness） | passed，0 warnings |
| cargo build --release --bin between-frames | passed（unsigned） |
| npm run typecheck / npm run lint | passed，0 warnings（eslint 忽略构建产物目录） |
| npm run test | passed，6 files / 24 tests |
| npm run test:e2e | passed，16 tests |
| CARGO_TARGET_DIR="target-v1" npm run test:integration | passed，3 tests（agent-core、local-core、beta-journey 52.2s） |

## 复现命令

~~~bash
npm run typecheck && npm run lint && npm test && npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1"
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --test perf_baseline -- --nocapture --ignored
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --release --bin between-frames
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
~~~

## 剩余问题与停止点

签名/公证/DMG 需 Apple 开发者证书；20 位用户 Beta 与 3 位摄影师无指导试用需真人执行（验收脚本以 beta-journey 自动化打底）；真实模型 E2E 仍待 API Key；M4 遗留（跨项目假设、FTS5、证据删除入口）继续有效。

六阶段工程实现至此全部落地，停止等待整体评审。见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M5-FILES.md)、[ADR 0008](ADR/0008-v1-m5-beta-readiness.md)、[隐私说明](PRIVACY.md)。
