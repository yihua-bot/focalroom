# v1.0 M2 实施计划

范围：受控动态 Mock Agent、任务恢复、预算、草案审批和分层 ContextBuilder。完成后停止评审；真实 Provider/视觉/云端授权属 M3，记忆假设和 FTS5 属 M4。

1. 修订工程机器契约：工具 items 与主规格一致；GUI 增加 ApplyDraft/RebaseDraft/UpdateDraft/DiscardDraft；任务请求、预算、恢复与协议明确。原始需求包保留。
2. 新增 0004：任务 FSM、append-only 步骤、唯一 task/call 回执、检查点、Mock usage、消息、草案及条目、GUI 生命周期幂等回执。升级前一致性备份，临时 M1 旧库测试；独立 M2 Review identifier 避免升级已有用户/Review DB。
3. Rust ToolRegistry 严格 schema 校验与独立 PolicyGate：可信任务 scope，ID 归属、次数/文本/时间/token/费用/图像预算；不存在 shell/SQL/网络能力，M2 不上传图像。草案写入/工具回执/步骤/检查点同事务并委托 CommandHandler。
4. AgentTaskEngine + ModelProvider Trait：工具→结果→下一轮重决策，后台执行；暂停/取消/反馈/预算调整均持久化；启动标记 interrupted，从保存的 call_id/outcome/结果安全恢复，有限重试。
5. ContextBuilder：目标/可信 policy/GUI scope/项目事实/人工摘要/执行协议分层；按 scope 过滤且限量，不冒充长期偏好。保留完整 call/result linkage 和剩余预算，不因裁剪丢失恢复信息。
6. GUI：显式发起 Mock、当前选区/目标数量/预算、真实状态与步骤、暂停/取消/恢复/反馈；A/B 草案 diff、单项采纳、顺序调整、显式 rebase；未采纳项保持，ApplyDraft 为一组可撤销命令。
7. 测试：AG-01/02/03 双观察轨迹、AG-05/06/07/08/09/10/13/14/15 的确定性安全/恢复；M3/4 未实现场景单独注明。Rust/契约/typecheck/lint/Vitest/Playwright/真实 Store/Clippy/build、原生重启、只读 hash、截图、文件清单与 ADR。

KISS/YAGNI：复用 sqlx/现有 Repository/UI/契约生成器，不加模型 SDK、网络服务或向量库。M2 Mock 的元数据比较和评估不宣称看懂图片。预算估算为保守文本计量，真实模型 token/成本留 M3。
