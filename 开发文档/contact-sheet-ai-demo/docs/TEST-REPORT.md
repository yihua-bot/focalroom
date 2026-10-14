# v1.0 M2 测试与验收

日期：2026-10-14（Asia/Shanghai）。M2 受控 Mock Runtime 已完成实现与自动化验收：任务 FSM、ModelProvider Trait 与本地 Mock、ToolRegistry/PolicyGate、预算与检查点恢复、草案审批（ApplyDraft/RebaseDraft/UpdateDraft/DiscardDraft）与分层 ContextBuilder。本轮包含一次全量自动化复验；macOS 原生验收证据由同日上一轮开发产生并保留。M3 真实 Provider/视觉/云端 consent、M4 记忆假设/FTS5 未启动。

M1 报告归档于 [V1-M1-TEST-REPORT.md](V1-M1-TEST-REPORT.md)，M0 报告见 [V1-M0-TEST-REPORT.md](V1-M0-TEST-REPORT.md)。产品基线为 v1.0，工程包版本 0.1.0。

## 功能验收

| 项目 | 完成内容 | 验证方式 | 结果 | 未解决问题 |
|---|---|---|---|---|
| 任务 FSM 与生命周期 | queued→planning→running→needs_input/review_ready/paused/stale/interrupted 等合法转移均落库带原因 | Rust agent_runtime 13 测试、GUI E2E | passed | FSM 状态覆盖 11 态；stale 由 finish 时版本比对触发 |
| 动态 Mock 循环 | ModelProvider 每轮依据真实工具结果重选下一工具；无固定流程回退 | AG-01/02/03 双观察轨迹断言工具轨迹而非最终 JSON | passed | Mock 元数据比较不能证明真实模型自主性 |
| 工具白名单与 Schema | 12 个摄影工具严格 JSON Schema 子集校验（const/enum/oneOf/长度/数量） | AG-05/06 伪造参数、未知工具、注入指令拒绝 | passed | Schema 校验器为工程子集，非完整 JSON Schema |
| PolicyGate | 项目隔离、photo_id 授权 scope、draft 归属、任务状态前置检查 | 跨项目/越权/他任务草案拒绝，数据库状态不变 | passed | M2 无 shell/SQL/网络能力；图像 0 |
| 预算与限流 | 轮次/工具数/时长/单轮与累计文本/输出上限均在后端代码执行 | AG-10 各维度触顶暂停；等待时间保守结算一次 | passed | 文本为保守 UTF-8 估算；真实 token/成本属 M3 |
| 幂等与恢复 | 请求回执（request_id+指纹）、task/call_id 唯一回执、检查点、outcome 先落库后执行 | AG-07 超时重放不重复草案；AG-09 重启无幽灵 running | passed | 中断恢复为检查点级，非 token 级续跑 |
| 草案审批 | ApplyDraft 全量/局部采纳、RebaseDraft 只更新基线、丢弃保留历史；均走唯一 CommandHandler | AG-08/15 局部采纳+Undo 后未采纳项不变、审计可回放 | passed | 全量采纳明确替换序列，UI 有影响预览 |
| ContextBuilder | 分层上下文、scope 过滤、限量人工事实、无资源路径、完整协议关联 | 200 张 scope→25 张双草案在默认上下文内不丢关联 | passed | 上下文含人工目标与限量备注，保留本地 |
| GUI | 委托表单（目标/数量/选区限制/六项预算）、暂停/恢复/取消/答复、步骤列表、A/B 草案 diff、局部采纳 | agent-core E2E 真实后端全旅程 54.7s | passed | 浏览器预览为内存 UI，任务需桌面版 |
| M0/M1 回归 | 命令历史、本地核心、既有 GUI 全部保留 | command_history 10、local_core 10、preview E2E 16 | passed | 无 |

## 自动化结果（2026-10-14 14:47–14:57 全量复验）

| 检查 | 结果 |
|---|---|
| npm run contracts:check | passed（覆盖 M1 GUI 命令契约；Agent 契约由 Rust 运行时校验并在 AG-05/06/13/14 测试） |
| npm run typecheck / npm run lint | passed，0 warnings |
| npm run test | passed，5 files / 21 tests（含 TauriAgentRepository 3 项） |
| npm run test:e2e | passed，16 tests / 14.1s |
| cargo test（默认 target） | passed，33 tests：agent_runtime 13 / 4.95s、command_history 10 / 0.13s、local_core 10 / 3.47s |
| cargo fmt -- --check | passed |
| cargo clippy --all-targets --features integration-harness -- -D warnings | passed，0 warnings |
| npm run test:integration | passed，2 tests / 1.1m（agent-core 54.7s、local-core 11.4s） |

复验环境说明：首次 integration 运行失败，根因是 `src-tauri/target/debug/m1-harness` 残留 10-13 旧二进制（无 agent 命令）；以 `cargo build --features integration-harness --bin m1-harness` 重建后全绿，属环境问题而非代码缺陷。正式复现应使用 `CARGO_TARGET_DIR="target-v1"`（见下）。

Rust 覆盖（agent_runtime 13 项）：AG-01 不同人工事实改变检索路径且正式状态不动；AG-02 重复组注释驱动草案修订而非固定工具顺序；AG-03 重排 scope 不做全库读取、两种顺序皆达；AG-05/06/13/14 越权 photo_id、schema 伪造、注入指令、虚构 draft_id 全部拒绝；AG-07 回执重放与超时后重启不产生重复草案；AG-08/15 stale→rebase→局部采纳→Undo 保留未采纳状态；AG-09 暂停/重启/恢复保持协议关联；AG-10 轮次/工具/上下文/时长四维触顶与重试失败；人工反馈与取消持久化；草案与回执故障同事务；正式编辑并发后完成草案转 stale；200 scope 双 25 草案默认上下文容纳；在途等待只结算一次。

## 原生验收证据（同日上一轮产生）

使用 Between Frames M2 Review（identifier `studio.betweenframes.m2review`），独立于用户原应用与 M0/M1 Review 数据目录。12 张 WKWebView 截图覆盖：工作台、预算暂停、中断（含中断中运行界面）、stale、局部 diff、局部采纳确认、全量确认、草案、序列、重启后状态。证据快照：[中断前运行](V1-M2-NATIVE-RUNNING-BEFORE.json)、[中断前](V1-M2-NATIVE-INTERRUPTED-BEFORE.json)、[中断后](V1-M2-NATIVE-INTERRUPTED-AFTER.json)、[局部采纳结果](V1-M2-NATIVE-PARTIAL-RESULT.json)、[范围采纳结果](V1-M2-NATIVE-SCOPED-RESULT.json)、[全量 Undo 结果](V1-M2-NATIVE-FULL-UNDO-RESULT.json)、[重启前后快照](V1-M2-NATIVE-SNAPSHOT-BEFORE.json)/[AFTER](V1-M2-NATIVE-SNAPSHOT-AFTER.json)、[源 checksum](V1-M2-NATIVE-SOURCE-CHECKSUMS.json)。本轮未重跑原生 .app，以自动化 integration 旅程与上述既有证据共同支撑验收结论。

## 复现命令

从 between-frames 目录执行（依赖已安装；本机 Cargo 如需镜像见 M1 报告同节）：

~~~bash
npm run contracts:check
npm run typecheck
npm run lint
npm run test
npm run test:e2e
cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1"
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --all-targets --features integration-harness -- -D warnings
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
CARGO_TARGET_DIR="target-v1" npm run test:integration
~~~

## 实际截图

真实 Rust/SQLite 浏览器（agent-core E2E 自动产出）：[工作台](screenshots/v1-m2/workspace.png)、[预算暂停](screenshots/v1-m2/paused.png)、[草案](screenshots/v1-m2/drafts.png)、[stale](screenshots/v1-m2/stale.png)、[局部采纳](screenshots/v1-m2/partial-approved.png)、[序列](screenshots/v1-m2/sequence.png)、[重启](screenshots/v1-m2/restarted.png)。

macOS/WKWebView 原生（上一轮验收保留）：见 [v1-m2-native](screenshots/v1-m2-native/)，12 张含中断/恢复/局部与全量采纳全旅程。

## 剩余问题与停止点

Mock 工具路径是测试场景，不能当作正式 Agent；真实 Provider 接入（M3）后必须接受模型自主输出的合法工具组合并复验 AG-01–15。inspect/compare 为元数据对照（`mock_metadata_only`），未读像素；费用恒 0、云端图像 0。检索依赖人工评分/备注线索，无视觉相似性。`npm run contracts:check` 尚未覆盖 agent 契约文件本身的一致性检查。原生首拖偶发性、100% 为 1400px 缓存预览、未保存备注草稿、高分辨率性能、签名公证等 M1 遗留项继续有效。

本轮停止 M2，等待评审。M3 交付 OpenAIProvider、图像 consent、真实视觉观察；M4 证据记忆与 Memory Studio。见 [已知问题](KNOWN_ISSUES.md)、[文件清单](V1-M2-FILES.md)、[ADR 0005](ADR/0005-v1-m2-mock-runtime.md)。
