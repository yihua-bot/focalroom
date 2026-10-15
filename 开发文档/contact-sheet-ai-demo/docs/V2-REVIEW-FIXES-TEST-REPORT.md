# v2 评审修复与验收报告

日期：2026-10-15。基线：33e5ff9，加本轮未提交修复。范围为评审列出的八项问题；没有开启新的云服务、支付服务或生产部署。决策见 [ADR 0023](ADR/0023-v2-review-transaction-boundaries.md)。

## 修复与功能验收

| 问题 | 当前行为 | 验收证据 |
|---|---|---|
| GUI 契约与假成功 | 使用共享 snake_case 契约、当前项目 revision；业务错误与回执身份错误拒绝；成功显示后端实际数量 | TauriProposalsRepository 3 项、ProposalsPanel 5 项；真实后端 GUI 提交/审批/冲突 |
| 多操作部分写入 | 审批、正式编辑、历史、证据与 v2 回执同事务提交；后续操作或回执写入失败全部回滚 | invalid_later_operation、receipt_persistence_failure；失败后可按原基线重新审批 |
| Promotion 单权威 | 写锁内重读状态，v1/v2 和导入/重连在迁移中及完成后拒绝正式修改 | promotion_blocks_legacy_and_v2_writes_even_with_stale_authority |
| 连续移位 | 操作按原提案顺序组合，使用前一步序列；整批一次 revision、一个 Undo/Redo 历史组 | approval_moves_compose_and_whole_batch_undo_redo |
| 元数据幂等 | create/submit/apply/comment 重启后重放返回原回执，冲突 payload 拒绝，不新增记录 | metadata_receipts_survive_restart_and_reject_conflicting_payloads |
| Router 接线 | 实际模型调用和授权图像上传前经过 Router；拒绝不发 HTTP；离线任务保持本地 | agent_vision 的 runtime_router 两项（区域、预算、缺提供方、离线不切云） |
| add 目标位置 | 保留照片并插入获批位置；空选驳回不改变作品版本与撤销栈 | approval_add_honors_position_and_empty_rejection_does_not_edit_project |
| Promotion 失败重试 | 同一 revision 可在 failed/aborted 后以新 ID 重试，旧记录保留；completed 禁止重新 begin | failed_and_aborted_promotion_retry；0017 旧记录复制与活动唯一约束测试 |

提案面板补充了列表、状态、操作与表单间距，避免文字挤在同一行；沿用现有设计 Tokens。[执行后截图](screenshots/v2-review-fixes/proposal-applied.png)、[冲突拒绝截图](screenshots/v2-review-fixes/proposal-conflict.png)。

## 自动化结果

- Rust：**110 通过**；性能测试 1 项 ignored；钥匙串写删回环 1 项主动 skip。临时数据库覆盖旧库升级、备份、回执、历史、记忆和图像处理。
- 前端：**18 文件 / 142 项通过**。契约生成一致性、TypeScript、ESLint、Rust fmt 与严格 Clippy（含 integration-harness）通过。
- 浏览器预览：**16 项通过**。独立 Web 基座：**1 项通过**，复用本机 1424 示例服务器。
- 真实后端 GUI：**6 条旅程通过**（Agent、Beta、诊断、200 张本地编辑、记忆控制、v2 提案）。首轮 5 项通过，诊断因迁移计数固定为 16 失败；更新为 17 后单独复验通过。v2 提案旅程验证页面提交、两项插入的位置、单次撤销/重做、重启与基线冲突。
- Desktop/Web 的 workspace build 通过；API/worker 仍为明确不可用的 skeleton。构建仍有既存的 chunk 大小与混合动态导入提示，不影响构建结果。
- 未进行新的原生窗口、系统文件选择器或签名包验收；真实桌面 IPC 的 Rust 入口已编译，GUI 功能证据来自浏览器接入真实 Rust harness。

过程中的一次测试启动端口竞争，以及 Web 示例服务未运行导致的连接拒绝，均通过顺序运行相关测试和启动示例服务器解决。没有关闭用户应用或改用 Mock 冒充后端。

日志归档见 [自动化结果](evidence/v2-review-fixes/results.json) 及同目录的文本输出。完整报告将历史里程碑截图保留原样，仅本轮提案截图独立归档。

## 复现

在 between-frames 目录执行；不启动原用户桌面应用。

```bash
npm run contracts:check
npm run typecheck
npm run lint
npm test
npm run build:workspaces
cargo fmt --manifest-path "src-tauri/Cargo.toml" -- --check
cargo clippy --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --locked --all-targets --features integration-harness -- -D warnings
BETWEEN_FRAMES_OPENAI_API_KEY="" cargo test --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" -- --skip store_load_delete_roundtrip
cargo build --manifest-path "src-tauri/Cargo.toml" --target-dir "src-tauri/target-v1" --features integration-harness --bin m1-harness
npm run test:e2e
CARGO_TARGET_DIR="target-v1" BETWEEN_FRAMES_OPENAI_API_KEY="" npm run test:integration
```

预览与后端 E2E 使用相同的 1420 入口，须顺序执行。Web 测试标准配置启动 1424；若该端口已经运行 Web 示例，复用现有服务即可。本轮使用临时外部配置禁用额外启动，并保留同一 testDir、baseURL 和测试断言。

## 兼容与剩余问题

新增 **0017**，不修改 0001–0016。回执表与 Promotion 约束调整仅在临时测试库/harness 中运行，原用户库和现有原生评审库未打开或升级；未提交、推送、安装或更新依赖，也未调用真实 AI/支付 API。

v2 仍为部分实现：[总账](V2-COMPLIANCE.md) 调整为 **16 pass / 8 partial**。500 张完整 GUI 旅程与真机性能、Creator 私有记忆隔离、真实区域配置入口、多供应商、质量与账单校准、云认证/共享/实时同步、教育隔离、媒体网关、真实支付仍未完成。Promotion 的 completed 确认仍为测试协议字符串，不是云端签名回执。新回执无法追溯补齐旧版未保存命令身份的元数据请求。

本轮停在评审修复和回归完成，下一里程碑须按实际缺口推进，不据协议单测推定整版完工。
